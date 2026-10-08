//! Timing firmware for the int8 model: cycles per model invoke.
//!
//! Same method as `timing.rs`: 10 warm-up invokes, then 1000 timed invokes of
//! the model only, with the instruction cache off and then on. The input is
//! the fixed upstream feature file, so there is no audio front end. The
//! label and the four logits go out over defmt RTT after each pass.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use core::sync::atomic::Ordering;

use cortex_m::peripheral::DWT;
use cycle_stats::{cycles_to_micros, summarise};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use iree_embedded::{Arena, Context, Device, Instance, Result, link_kernels, singleton};
use microbit_v2_kws::int8::{EXPECTED_LOGITS, VMFB, input_tensor, matches_reference};
use microbit_v2_kws::{CORE_CLOCK_HZ, best_label};
use panic_probe as _;

const WARMUP_INVOKES: usize = 10;
const TIMED_INVOKES: usize = 1000;

// newlib's malloc references _sbrk; the IREE runtime allocates from the
// arena instead, so the libc heap can never grow.
iree_embedded::libc_stubs!();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    defmt::info!(
        "iree-embedded KWS int8 timing: {} timed invokes",
        TIMED_INVOKES
    );

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dcb = cp.DCB;
    let mut dwt = cp.DWT;
    dcb.enable_trace();
    dwt.enable_cycle_counter();

    let arena = Arena::new(singleton!([u8; 56 * 1024] = [0; 56 * 1024]));
    let cycles = singleton!([u32; TIMED_INVOKES] = [0; TIMED_INVOKES]);
    // The top-left LED of the matrix: row 1 (P0.21) high, column 1 (P0.28) low.
    let mut row1 = Output::new(p.P0_21, Level::Low, OutputDrive::Standard);
    let _col1 = Output::new(p.P0_28, Level::Low, OutputDrive::Standard);

    match run(&arena, cycles) {
        // Solid LED: the run finished and the results are in RTT.
        Ok(()) => row1.set_high(),
        Err(e) => {
            defmt::error!(
                "failed: {} (raw {}): {} | largest failed alloc = {} bytes",
                defmt::Debug2Format(&e.code()),
                e.raw_code(),
                e.message(),
                iree_embedded::LAST_ALLOC_FAIL_SIZE.load(Ordering::Relaxed)
            );
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

fn run(arena: &Arena, cycles: &mut [u32; TIMED_INVOKES]) -> Result<()> {
    let instance = Instance::new(arena)?;
    let device = Device::local_sync_static(
        arena,
        &[link_kernels!(micro_speech_int8_nosm_linked_library_query)],
    )?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;

    let input = input_tensor(&device)?;

    // The instruction cache is off at reset. Cycle counts with it off change
    // by about 10% between builds, so both settings are reported.
    for cache_on in [false, true] {
        nrf_pac::NVMC.icachecnf().write(|w| w.set_cacheen(cache_on));
        for _ in 0..WARMUP_INVOKES {
            ctx.invoke(infer, &[&input], arena)?;
        }

        let mut logits = [0.0f32; 4];
        for slot in cycles.iter_mut() {
            let start = DWT::cycle_count();
            let outputs = ctx.invoke(infer, &[&input], arena)?;
            *slot = DWT::cycle_count().wrapping_sub(start);
            outputs[0].read_into_f32(&device, &mut logits)?;
        }
        defmt::info!(
            "instruction cache {}: last invoke classified: {} (expected 'yes')",
            if cache_on { "on" } else { "off" },
            best_label(&logits)
        );
        defmt::info!(
            "logits {} (reference {}), within one step: {}",
            logits,
            EXPECTED_LOGITS,
            matches_reference(&logits)
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
