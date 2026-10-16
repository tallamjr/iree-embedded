//! Timing firmware: cycles per model invoke on the Nano 33 BLE Sense.
//!
//! Runs 10 warm-up invokes, then 1000 timed invokes of the model only, with
//! the instruction cache off and then on. The audio front end runs once,
//! before the loops. Results go out over defmt RTT after both runs.
//! `timing_usb` prints the same lines over USB serial.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use core::sync::atomic::Ordering;

use arduino_nano33ble_kws::{CORE_CLOCK_HZ, TIMED_INVOKES, fatal_blink, run_timing};
use cycle_stats::cycles_to_micros;
use cycle_stats::report::EXPECTED_LABEL;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use iree_embedded::{Arena, singleton};
use kws_frontend::Frontend;
use panic_probe as _;

// newlib's malloc references _sbrk; the IREE runtime allocates from the
// arena instead, so the libc heap can never grow.
iree_embedded::libc_stubs!();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    defmt::info!("iree-embedded KWS timing: {} timed invokes", TIMED_INVOKES);

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dcb = cp.DCB;
    let mut dwt = cp.DWT;
    dcb.enable_trace();
    dwt.enable_cycle_counter();

    let arena = Arena::new(singleton!([u8; 56 * 1024] = [0; 56 * 1024]));
    let fe = singleton!(Frontend = Frontend::new());
    let cycles = singleton!([u32; TIMED_INVOKES] = [0; TIMED_INVOKES]);
    let mut status = Output::new(p.P0_13, Level::Low, OutputDrive::Standard);

    match run_timing(&arena, fe, cycles) {
        Ok(runs) => {
            for run in runs {
                defmt::info!(
                    "instruction cache {}: last invoke classified: {} (expected '{}')",
                    if run.cache_on { "on" } else { "off" },
                    run.label,
                    EXPECTED_LABEL
                );
                defmt::info!("logits {}", run.logits);
                defmt::info!(
                    "cycles: min {} median {} p99 {} max {}",
                    run.stats.min,
                    run.stats.median,
                    run.stats.p99,
                    run.stats.max
                );
                defmt::info!(
                    "microseconds at {} Hz: min {} median {} p99 {} max {}",
                    CORE_CLOCK_HZ,
                    cycles_to_micros(run.stats.min, CORE_CLOCK_HZ),
                    cycles_to_micros(run.stats.median, CORE_CLOCK_HZ),
                    cycles_to_micros(run.stats.p99, CORE_CLOCK_HZ),
                    cycles_to_micros(run.stats.max, CORE_CLOCK_HZ)
                );
            }
            // Solid orange LED: the run finished and the results are in RTT.
            status.set_high();
        }
        Err(e) => {
            defmt::error!(
                "failed: {} (raw {}): {} | largest failed alloc = {} bytes",
                defmt::Debug2Format(&e.code()),
                e.raw_code(),
                e.message(),
                iree_embedded::LAST_ALLOC_FAIL_SIZE.load(Ordering::Relaxed)
            );
            fatal_blink(&mut status);
        }
    }
    loop {
        cortex_m::asm::wfi();
    }
}
