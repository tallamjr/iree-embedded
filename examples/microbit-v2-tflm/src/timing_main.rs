// Body of the timing firmware. `bin/timing_tflm.rs` and
// `bin/timing_tflm_cmsis.rs` include it, so one source makes both binaries.
use cortex_m::peripheral::DWT;
use cycle_stats::{cycles_to_micros, summarise};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use microbit_v2_tflm::{
    Aligned, CMSIS_NN, CORE_CLOCK_HZ, EXPECTED_LOGITS, Error, Interpreter, KERNELS, MODEL,
    best_label, features,
};
use panic_probe as _;
use static_cell::StaticCell;

/// True when the binary name ends in `_cmsis`.
const fn is_cmsis_name(name: &[u8]) -> bool {
    let suffix = b"_cmsis";
    if name.len() < suffix.len() {
        return false;
    }
    let start = name.len() - suffix.len();
    let mut index = 0;
    while index < suffix.len() {
        if name[start + index] != suffix[index] {
            return false;
        }
        index += 1;
    }
    true
}

// The binary name must tell which kernels it holds: `timing_tflm_cmsis` is
// the only binary built with the `cmsis-nn` feature.
const _: () = assert!(
    is_cmsis_name(env!("CARGO_BIN_NAME").as_bytes()) == CMSIS_NN,
    "build timing_tflm without the cmsis-nn feature, and timing_tflm_cmsis with it"
);

const WARMUP_INVOKES: usize = 10;
const TIMED_INVOKES: usize = 1000;
const ARENA_BYTES: usize = 16 * 1024;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    defmt::info!(
        "TFLite-Micro int8 timing, {} kernels: {} timed invokes",
        KERNELS,
        TIMED_INVOKES
    );

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dcb = cp.DCB;
    let mut dwt = cp.DWT;
    dcb.enable_trace();
    dwt.enable_cycle_counter();

    static ARENA: StaticCell<Aligned<[u8; ARENA_BYTES]>> = StaticCell::new();
    static CYCLES: StaticCell<[u32; TIMED_INVOKES]> = StaticCell::new();
    let arena = ARENA.init(Aligned([0; ARENA_BYTES]));
    let cycles = CYCLES.init([0; TIMED_INVOKES]);
    // The top-left LED of the matrix: row 1 (P0.21) high, column 1 (P0.28) low.
    let mut row1 = Output::new(p.P0_21, Level::Low, OutputDrive::Standard);
    let _col1 = Output::new(p.P0_28, Level::Low, OutputDrive::Standard);

    match run(&mut arena.0, cycles) {
        // Solid LED: the run finished and the results are in RTT.
        Ok(()) => row1.set_high(),
        Err(e) => {
            defmt::error!("failed: {}", e);
            // Fast blink: the run failed.
            loop {
                row1.set_high();
                cortex_m::asm::delay(6_400_000); // ~150 ms: asm::delay takes 1.5 cycles per count
                row1.set_low();
                cortex_m::asm::delay(6_400_000);
            }
        }
    }
    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &'static mut [u8], cycles: &mut [u32; TIMED_INVOKES]) -> Result<(), Error> {
    let mut interpreter = Interpreter::new(&MODEL.0, arena)?;
    interpreter.set_input(features())?;
    defmt::info!(
        "arena used by TFLite-Micro: {} of {} bytes",
        interpreter.arena_used_bytes(),
        ARENA_BYTES
    );

    // The instruction cache is off at reset. Cycle counts with it off change
    // by about 10% between builds, so both settings are reported.
    for cache_on in [false, true] {
        nrf_pac::NVMC.icachecnf().write(|w| w.set_cacheen(cache_on));
        for _ in 0..WARMUP_INVOKES {
            interpreter.set_input(features())?;
            interpreter.invoke()?;
        }

        let mut logits = [0i8; 4];
        for slot in cycles.iter_mut() {
            // The model overwrites its input while it runs, so set it each time.
            interpreter.set_input(features())?;
            let start = DWT::cycle_count();
            interpreter.invoke()?;
            *slot = DWT::cycle_count().wrapping_sub(start);
            interpreter.read_output(&mut logits)?;
        }
        defmt::info!(
            "instruction cache {}: last invoke classified: {} (expected 'yes')",
            if cache_on { "on" } else { "off" },
            best_label(&logits)
        );
        defmt::info!(
            "logits {} (reference {}), equal: {}",
            logits,
            EXPECTED_LOGITS,
            logits == EXPECTED_LOGITS
        );

        let stats = summarise(cycles).expect("TIMED_INVOKES is above zero");
        defmt::info!(
            "cycles: min {} median {} p99 {} max {}",
            stats.min,
            stats.median,
            stats.p99,
            stats.max
        );
        defmt::info!(
            "microseconds at {} Hz: min {} median {} p99 {} max {}",
            CORE_CLOCK_HZ,
            cycles_to_micros(stats.min, CORE_CLOCK_HZ),
            cycles_to_micros(stats.median, CORE_CLOCK_HZ),
            cycles_to_micros(stats.p99, CORE_CLOCK_HZ),
            cycles_to_micros(stats.max, CORE_CLOCK_HZ)
        );
    }
    Ok(())
}
