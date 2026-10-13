//! Arena check: one init and one invoke, then the result over defmt RTT.
//! Set the arena size with `TFLM_ARENA_BYTES` at build time. Run it with
//! `scripts/arena-bisect.sh` to find the smallest arena that works.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use defmt_rtt as _;
use embassy_executor::Spawner;
use microbit_v2_tflm::{
    ARENA_BYTES, Aligned, EXPECTED_LOGITS, Error, Interpreter, KERNELS, MODEL, features,
};
use panic_probe as _;
use static_cell::StaticCell;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Default::default());
    static ARENA: StaticCell<Aligned<[u8; ARENA_BYTES]>> = StaticCell::new();
    let arena = ARENA.init(Aligned([0; ARENA_BYTES]));
    defmt::info!(
        "arena check, {} kernels, arena {} bytes",
        KERNELS,
        ARENA_BYTES
    );

    match run(&mut arena.0) {
        Ok(logits) => defmt::info!(
            "logits {} (reference {}), equal: {}",
            logits,
            EXPECTED_LOGITS,
            logits == EXPECTED_LOGITS
        ),
        Err(error) => defmt::error!("failed: {}", error),
    }
    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &'static mut [u8]) -> Result<[i8; 4], Error> {
    let mut interpreter = Interpreter::new(&MODEL.0, arena)?;
    defmt::info!(
        "arena used by TFLite-Micro: {} bytes",
        interpreter.arena_used_bytes()
    );
    interpreter.set_input(features())?;
    interpreter.invoke()?;
    let mut logits = [0i8; 4];
    interpreter.read_output(&mut logits)?;
    Ok(logits)
}
