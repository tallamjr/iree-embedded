//! Code shared by the keyword-spotting firmware (`main.rs`) and the timing
//! firmware (`bin/timing.rs`): model data, the model call and the fatal blink.
#![no_std]
#![forbid(unsafe_code)]

use cortex_m::peripheral::DWT;
use cycle_stats::report::CacheRun;
use cycle_stats::summarise;
use embassy_nrf::gpio::Output;
use iree_embedded::{
    Arena, Context, Device, Function, Instance, Result, Tensor, include_vmfb, link_kernels,
};
use kws_frontend::{FEATURE_BYTES, Frontend};

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

pub const WARMUP_INVOKES: usize = 10;
pub const TIMED_INVOKES: usize = 1000;

/// Time the model with the instruction cache off, then on. The audio front
/// end runs once, before the loops. Nothing prints during the loops.
pub fn run_timing(
    arena: &Arena,
    fe: &mut Frontend,
    cycles: &mut [u32; TIMED_INVOKES],
) -> Result<[CacheRun<'static>; 2]> {
    fe.init();
    let instance = Instance::new(arena)?;
    let device = Device::local_sync_static(
        arena,
        &[link_kernels!(micro_speech_nosm_linked_library_query)],
    )?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;

    // Features are computed once, so the loop times the model alone.
    let mut features = [0u8; FEATURE_BYTES];
    fe.features_oneshot(self_test_clip(), &mut features);
    let input = Tensor::from_u8(&device, &[1, 49, 40, 1], &features)?;

    // The instruction cache is off at reset. Cycle counts with it off change
    // by about 10% between builds, so both settings are reported.
    let cache_off = time_invokes(&ctx, &device, infer, &input, arena, cycles, false)?;
    let cache_on = time_invokes(&ctx, &device, infer, &input, arena, cycles, true)?;
    Ok([cache_off, cache_on])
}

fn time_invokes(
    ctx: &Context,
    device: &Device,
    infer: Function,
    input: &Tensor,
    arena: &Arena,
    cycles: &mut [u32; TIMED_INVOKES],
    cache_on: bool,
) -> Result<CacheRun<'static>> {
    nrf_pac::NVMC.icachecnf().write(|w| w.set_cacheen(cache_on));
    for _ in 0..WARMUP_INVOKES {
        ctx.invoke(infer, &[input], arena)?;
    }

    let mut logits = [0.0f32; 4];
    for slot in cycles.iter_mut() {
        let start = DWT::cycle_count();
        let outputs = ctx.invoke(infer, &[input], arena)?;
        *slot = DWT::cycle_count().wrapping_sub(start);
        outputs[0].read_into_f32(device, &mut logits)?;
    }
    Ok(CacheRun {
        cache_on,
        label: best_label(&logits),
        logits,
        stats: summarise(cycles).expect("TIMED_INVOKES is above zero"),
    })
}
