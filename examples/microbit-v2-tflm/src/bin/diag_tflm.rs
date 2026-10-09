//! Diagnostic firmware: prints status codes and raw bytes of one model run.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use defmt_rtt as _;
use embassy_executor::Spawner;
use microbit_v2_tflm::{
    Aligned, EXPECTED_LOGITS, FEATURE_COUNT, Interpreter, MODEL, double_to_i64, features, frexp,
    quantize_multiplier, round,
};
use panic_probe as _;
use static_cell::StaticCell;

const ARENA_BYTES: usize = 16 * 1024;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Default::default());
    static ARENA: StaticCell<Aligned<[u8; ARENA_BYTES]>> = StaticCell::new();
    let arena = ARENA.init(Aligned([0; ARENA_BYTES]));

    defmt::info!("msp {=u32:x}", cortex_m::register::msp::read());
    defmt::info!(
        "model first 16 bytes {=[u8]:x} len {}",
        &MODEL.0[..16],
        MODEL.0.len()
    );
    defmt::info!("features first 16 {}", &features()[..16]);

    // Double-precision helpers used by the kernels. Host values are in the comment.
    for real in [0.5f64, 0.0123f64, 1.0e-5f64] {
        let (multiplier, shift) = quantize_multiplier(real);
        let (fraction, exponent) = frexp(real);
        defmt::info!(
            "real bits {=u64:x}: multiplier {} shift {} | frexp bits {=u64:x} exp {} | round {=u64:x}",
            real.to_bits(),
            multiplier,
            shift,
            fraction.to_bits(),
            exponent,
            round(real * 2147483648.0).to_bits()
        );
    }
    defmt::info!("d2i64 {} (expect 1234567891)", double_to_i64(1234567890.7));

    let mut interpreter = match Interpreter::new(&MODEL.0, &mut arena.0) {
        Ok(i) => i,
        Err(e) => {
            defmt::error!("init failed {}", e);
            loop {
                cortex_m::asm::wfi();
            }
        }
    };
    defmt::info!("arena used {}", interpreter.arena_used_bytes());
    defmt::info!("set_input {}", interpreter.set_input(features()));
    let mut readback = [0i8; FEATURE_COUNT];
    defmt::info!("read_input {}", interpreter.read_input(&mut readback));
    defmt::info!("input readback first 16 {}", &readback[..16]);
    defmt::info!("input equal: {}", readback[..] == *features());
    defmt::info!("invoke {}", interpreter.invoke());
    let mut logits = [0i8; 4];
    defmt::info!("read_output {}", interpreter.read_output(&mut logits));
    defmt::info!("output {} expected {}", &logits[..], &EXPECTED_LOGITS[..]);
    loop {
        cortex_m::asm::wfi();
    }
}
