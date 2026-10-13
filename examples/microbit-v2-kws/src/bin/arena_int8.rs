//! Arena check for the int8 model: init and one invoke, then the result over
//! defmt RTT. Set the arena size with `IREE_ARENA_BYTES` at build time. Run it
//! with `examples/microbit-v2-tflm/scripts/arena-bisect.sh` to find the
//! smallest arena that works.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use core::sync::atomic::Ordering;

use defmt_rtt as _;
use embassy_executor::Spawner;
use iree_embedded::{Arena, Context, Device, Instance, Result, link_kernels, singleton};
use microbit_v2_kws::best_label;
use microbit_v2_kws::int8::{ARENA_BYTES, EXPECTED_LOGITS, VMFB, input_tensor, matches_reference};
use panic_probe as _;

iree_embedded::libc_stubs!();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Default::default());
    defmt::info!("arena check, IREE int8, arena {} bytes", ARENA_BYTES);
    let arena = Arena::new(singleton!([u8; ARENA_BYTES] = [0; ARENA_BYTES]));

    match run(&arena) {
        Ok(logits) => defmt::info!(
            "classified: {}, logits {} (reference {}), within one step: {}",
            best_label(&logits),
            logits,
            EXPECTED_LOGITS,
            matches_reference(&logits)
        ),
        Err(e) => defmt::error!(
            "failed: {} (raw {}): {} | largest failed alloc = {} bytes",
            defmt::Debug2Format(&e.code()),
            e.raw_code(),
            e.message(),
            iree_embedded::LAST_ALLOC_FAIL_SIZE.load(Ordering::Relaxed)
        ),
    }
    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &Arena) -> Result<[f32; 4]> {
    let instance = Instance::new(arena)?;
    let device = Device::local_sync_static(
        arena,
        &[link_kernels!(micro_speech_int8_nosm_linked_library_query)],
    )?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;
    let input = input_tensor(&device)?;
    let outputs = ctx.invoke(infer, &[&input], arena)?;
    let mut logits = [0.0f32; 4];
    outputs[0].read_into_f32(&device, &mut logits)?;
    Ok(logits)
}
