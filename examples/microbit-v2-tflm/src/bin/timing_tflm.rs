//! Timing firmware for the TFLite-Micro baseline: cycles per model invoke.
//!
//! Same method as `timing_int8.rs` of `microbit-v2-kws`: 10 warm-up invokes,
//! then 1000 timed invokes of the model only, with the instruction cache off
//! and then on. The input is the fixed upstream feature file. The label and
//! the four int8 logits go out over defmt RTT after each pass.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use cortex_m::peripheral::DWT;
use cycle_stats::{cycles_to_micros, summarise};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use microbit_v2_tflm::{
    Aligned, CORE_CLOCK_HZ, EXPECTED_LOGITS, Error, Interpreter, MODEL, best_label, features,
};
use panic_probe as _;
use static_cell::StaticCell;

const WARMUP_INVOKES: usize = 10;
const TIMED_INVOKES: usize = 1000;
const ARENA_BYTES: usize = 16 * 1024;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    defmt::info!("TFLite-Micro int8 timing: {} timed invokes", TIMED_INVOKES);

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
