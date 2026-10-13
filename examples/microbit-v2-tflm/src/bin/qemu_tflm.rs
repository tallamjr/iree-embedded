//! Emulator check: runs the model once on an emulated Cortex-M4F (qemu
//! mps2-an386) and prints the result over semihosting. The cycle counts of
//! the emulator mean nothing. Run: scripts/qemu-check.sh
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use cortex_m_rt::entry;
use cortex_m_semihosting::debug::{EXIT_FAILURE, EXIT_SUCCESS, exit};
use cortex_m_semihosting::hprintln;
use defmt_rtt as _;
// Links the nRF interrupt vector table that cortex-m-rt asks for.
use embassy_nrf as _;
use microbit_v2_tflm::{
    Aligned, EXPECTED_LOGITS, Interpreter, KERNELS, MODEL, double_to_i64, features, frexp,
    paint_stack, quantize_multiplier, round,
};
use panic_probe as _;
use static_cell::StaticCell;

const ARENA_BYTES: usize = 16 * 1024;

#[entry]
fn main() -> ! {
    static ARENA: StaticCell<Aligned<[u8; ARENA_BYTES]>> = StaticCell::new();
    let arena = ARENA.init(Aligned([0xA5; ARENA_BYTES]));

    hprintln!("kernels {}", KERNELS);
    paint_stack(0xA5, 8192);
    for real in [0.5f64, 0.0123f64, 1.0e-5f64] {
        let (multiplier, shift) = quantize_multiplier(real);
        let (fraction, exponent) = frexp(real);
        hprintln!(
            "real {:x}: multiplier {} shift {} frexp {:x} exp {} round {:x}",
            real.to_bits(),
            multiplier,
            shift,
            fraction.to_bits(),
            exponent,
            round(real * 2147483648.0).to_bits()
        );
    }
    hprintln!("d2i64 {}", double_to_i64(1234567890.7));

    let mut interpreter = match Interpreter::new(&MODEL.0, &mut arena.0) {
        Ok(interpreter) => interpreter,
        Err(error) => {
            hprintln!("init failed: {:?}", error);
            exit(EXIT_FAILURE);
            loop {}
        }
    };
    hprintln!("arena used {}", interpreter.arena_used_bytes());
    let mut logits = [0i8; 4];
    // Same shape as the timing loop: the input is set before every invoke.
    let mut result = Ok(());
    for _ in 0..20 {
        result = result
            .and_then(|()| interpreter.set_input(features()))
            .and_then(|()| interpreter.invoke())
            .and_then(|()| interpreter.read_output(&mut logits));
    }
    let mut stale = [0i8; 4];
    let stale_result = interpreter
        .invoke()
        .and_then(|()| interpreter.read_output(&mut stale));
    hprintln!("invoke without set_input {:?}: {:?}", stale_result, stale);
    hprintln!(
        "result {:?} logits {:?} expected {:?}",
        result,
        logits,
        EXPECTED_LOGITS
    );
    exit(if result.is_ok() && logits == EXPECTED_LOGITS {
        EXIT_SUCCESS
    } else {
        EXIT_FAILURE
    });
    loop {}
}
