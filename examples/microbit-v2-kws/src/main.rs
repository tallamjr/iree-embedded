#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt_rtt as _;
use iree_embedded::{include_vmfb, Arena, Context, Device, Instance, Result, Tensor};
use panic_probe as _;

// The keyword-spotting model (TFLite-Micro micro_speech, softmax stripped),
// compiled for thumbv7em and embedded (16-byte aligned).
static VMFB: &[u8] = include_vmfb!("../models/micro_speech.vmfb");

// Real "yes" audio features (uint8 spectrogram [1,49,40,1]) produced by the
// TFLite-Micro front end. Used until the live PDM-mic pipeline is wired in.
static YES_FEATURES: &[u8] = include_bytes!("../models/yes_features.bin");

const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];

// Fixed arena for every IREE allocation (incl. the loaded ELF kernels). Sized
// to leave room for the stack in the nRF52833's 128 KB RAM.
static mut HEAP: [u8; 96 * 1024] = [0; 96 * 1024];

// newlib's malloc/_sbrk reference the `end` heap symbol; we use the arena
// instead, so provide a failing _sbrk to keep newlib's heap out of the image.
#[no_mangle]
pub extern "C" fn _sbrk(_incr: isize) -> *mut core::ffi::c_void {
    -1isize as *mut core::ffi::c_void
}

#[entry]
fn main() -> ! {
    defmt::info!("iree-embedded KWS: micro_speech on micro:bit v2");

    // SAFETY: single core; exclusive access to HEAP, taken once.
    let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(HEAP)) };

    match run(&arena) {
        Ok(label) => defmt::info!("KWS prediction = {} (expected 'yes')", label),
        Err(e) => defmt::error!("inference failed: {}", defmt::Debug2Format(&e)),
    }

    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &Arena) -> Result<&'static str> {
    let instance = Instance::new(arena)?;
    let device = Device::local_sync(arena)?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;

    // micro_speech input: uint8 spectrogram [1, 49, 40, 1] = 1960 features.
    let input = Tensor::from_u8(&device, &[1, 49, 40, 1], YES_FEATURES)?;

    let outputs = ctx.invoke(infer, &[&input], arena)?;
    let mut logits = [0.0f32; 4];
    outputs[0].read_into_f32(&device, &mut logits)?;

    let best = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .unwrap();
    Ok(LABELS[best])
}
