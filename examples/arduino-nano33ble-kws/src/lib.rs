//! Code shared by the keyword-spotting firmware (`main.rs`) and the timing
//! firmware (`bin/bench.rs`): model data, the model call and the fatal blink.
#![no_std]
#![forbid(unsafe_code)]

use embassy_nrf::gpio::Output;
use iree_embedded::{Arena, Context, Device, Function, Result, Tensor, include_vmfb};
use kws_frontend::FEATURE_BYTES;

// The keyword-spotting model (TFLite-Micro micro_speech, softmax stripped).
// Same artefact as the micro:bit example: both boards are Cortex-M4F, so the
// statically linked kernels (models/micro_speech.o) are identical.
pub static VMFB: &[u8] = include_vmfb!("../models/micro_speech.vmfb");

// Real 1-second "yes" recording (16 kHz mono int16) used as a boot-time
// self-test.
#[repr(C, align(4))]
struct Align4<T: ?Sized>(T);
static AUDIO: &Align4<[u8]> = &Align4(*include_bytes!("../models/yes_audio.bin"));

pub const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];

/// Core clock of the nRF52840 in Hz.
pub const CORE_CLOCK_HZ: u32 = 64_000_000;

/// The embedded "yes" clip as samples. The Align4 wrapper satisfies the
/// alignment check of `cast_slice`.
pub fn self_test_clip() -> &'static [i16] {
    bytemuck::cast_slice(&AUDIO.0)
}

/// Fatal-error signature without a probe: rapid orange blink, forever.
pub fn fatal_blink(status: &mut Output) -> ! {
    loop {
        status.set_high();
        cortex_m::asm::delay(6_400_000); // ~100 ms at 64 MHz
        status.set_low();
        cortex_m::asm::delay(6_400_000);
    }
}

/// Label of the largest logit.
pub fn best_label(logits: &[f32; 4]) -> &'static str {
    let best = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .unwrap();
    LABELS[best]
}

/// Run the model over a 49x40 feature window.
pub fn classify(
    ctx: &Context,
    device: &Device,
    infer: Function,
    features: &[u8; FEATURE_BYTES],
    arena: &Arena,
) -> Result<(&'static str, [f32; 4])> {
    let input = Tensor::from_u8(device, &[1, 49, 40, 1], features)?;
    let outputs = ctx.invoke(infer, &[&input], arena)?;
    let mut logits = [0.0f32; 4];
    outputs[0].read_into_f32(device, &mut logits)?;
    Ok((best_label(&logits), logits))
}
