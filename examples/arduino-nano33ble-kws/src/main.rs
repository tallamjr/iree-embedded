#![no_std]
#![no_main]
// Same promise as the micro:bit example: running IREE inference on bare metal
// needs no unsafe code in user firmware. The irreducible unsafe lives behind
// iree-embedded's macros.
#![forbid(unsafe_code)]

use core::sync::atomic::Ordering;

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::pdm::{self, Config as PdmConfig, Frequency, Pdm, SamplerState};
use embassy_nrf::{bind_interrupts, peripherals};
use iree_embedded::{
    Arena, Context, Device, Instance, Result, Tensor, include_vmfb, link_kernels, singleton,
};
use kws_frontend::{FEATURE_BYTES, Frontend};
use panic_probe as _;

bind_interrupts!(struct Irqs {
    PDM => pdm::InterruptHandler<peripherals::PDM>;
});

// The keyword-spotting model (TFLite-Micro micro_speech, softmax stripped).
// Same artefact as the micro:bit example: both boards are Cortex-M4F, so the
// statically linked kernels (models/micro_speech.o) are identical.
static VMFB: &[u8] = include_vmfb!("../models/micro_speech.vmfb");

// Real 1-second "yes" recording (16 kHz mono int16) used as a boot-time
// self-test before the live microphone loop starts.
#[repr(C, align(4))]
struct Align4<T: ?Sized>(T);
static AUDIO: &Align4<[u8]> = &Align4(*include_bytes!("../models/yes_audio.bin"));

const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];

// A detection needs the winning yes/no logit to beat the runner-up by this
// much; one window is classified every 250 ms, so weak wins are just noise.
const DETECT_MARGIN: f32 = 2.0;

// ... and the window must contain real acoustic energy. PDM levels differ
// from the micro:bit's SAADC operating point; this initial gate is tuned on
// hardware (raise it if ambient noise triggers detections, lower it if
// near-mic speech does not).
const SPEECH_LEVEL: i32 = 150;

// Digital gain applied after DC removal. The PDM peripheral outputs full
// int16 PCM, so far less boost is needed than the SAADC's millivolt swings.
const GAIN: i32 = 4;

// 16 kHz mono (PDM clock 1.280 MHz / ratio 80). One buffer is 250 ms; the
// front end runs continuously and the rolling 1 s spectrogram is classified
// each time a buffer completes.
const SAMPLE_RATE: u32 = 16_000;
const CHUNK: usize = SAMPLE_RATE as usize / 4;

// newlib's malloc references _sbrk; the IREE runtime allocates from the
// arena instead, so the libc heap can never grow.
iree_embedded::libc_stubs!();

/// Milliseconds elapsed since `start` (DWT cycle counter, 64 MHz core clock).
fn ms_since(start: u32) -> u32 {
    cortex_m::peripheral::DWT::cycle_count().wrapping_sub(start) / 64_000
}

/// Fatal-error signature without a probe: rapid orange blink, forever.
fn fatal_blink(status: &mut Output) -> ! {
    loop {
        status.set_high();
        cortex_m::asm::delay(6_400_000); // ~100 ms at 64 MHz
        status.set_low();
        cortex_m::asm::delay(6_400_000);
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    defmt::info!("iree-embedded KWS: PDM mic -> front end -> micro_speech on Nano 33 BLE Sense");

    // Enable the cycle counter so each window's compute can be timed.
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dcb = cp.DCB;
    let mut dwt = cp.DWT;
    dcb.enable_trace();
    dwt.enable_cycle_counter();

    // IREE arena (micro_speech needs ~50 KB across context + invoke
    // transients). singleton! hands out statics living in .bss (no stack
    // cost), each takeable exactly once.
    let arena = Arena::new(singleton!([u8; 56 * 1024] = [0; 56 * 1024]));
    // The pure-Rust audio front end (~14 KiB of fixed-point state).
    let fe = singleton!(Frontend = Frontend::new());

    // Orange status LED (P0.13, active high): self-test and fatal patterns.
    let mut status = Output::new(p.P0_13, Level::Low, OutputDrive::Standard);
    // RGB LED (active LOW): green = "yes", red = "no" detection feedback.
    let green = Output::new(p.P0_16, Level::High, OutputDrive::Standard);
    let red = Output::new(p.P0_24, Level::High, OutputDrive::Standard);

    // MP34DT05/06 PDM microphone power (P0.17); give it ~100 ms to settle.
    let _mic_pwr = Output::new(p.P0_17, Level::High, OutputDrive::HighDrive);
    cortex_m::asm::delay(6_400_000);

    // PDM at exactly 16 kHz: 1.280 MHz clock / ratio 80 (the Config default
    // ratio on nRF52840). Mono, default gain; GAIN above does digital boost.
    let config = PdmConfig {
        frequency: Frequency::_1280K,
        ..Default::default()
    };
    let pdm = Pdm::new(p.PDM, Irqs, p.P0_26, p.P0_25, config);

    if let Err(e) = run(&arena, fe, pdm, &mut status, green, red).await {
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

async fn run(
    arena: &Arena,
    fe: &mut Frontend,
    mut pdm: Pdm<'_>,
    status: &mut Output<'_>,
    mut green: Output<'_>,
    mut red: Output<'_>,
) -> Result<()> {
    fe.init();

    // Bring up the IREE pipeline once; every window reuses it.
    let instance = Instance::new(arena)?;
    // The query entry point of the statically linked micro_speech kernels
    // (declared in models/micro_speech.h, generated by iree-compile).
    let device = Device::local_sync_static(
        arena,
        &[link_kernels!(micro_speech_nosm_linked_library_query)],
    )?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;

    // Boot self-test on the embedded "yes" clip: proves the whole pipeline
    // (front end + model) before live audio. Pass = two slow orange blinks.
    // The Align4 wrapper on AUDIO satisfies cast_slice's alignment check.
    let clip: &[i16] = bytemuck::cast_slice(&AUDIO.0);
    let mut features = [0u8; FEATURE_BYTES];
    fe.features_oneshot(clip, &mut features);
    let (label, logits) = classify(&ctx, &device, infer, &features, arena)?;
    defmt::info!(
        "self-test on embedded clip: {} (expected 'yes'), logits = {}",
        label,
        logits
    );
    if label != "yes" {
        defmt::error!("self-test misclassified; not starting the live loop");
        fatal_blink(status);
    }
    for _ in 0..2 {
        status.set_high();
        cortex_m::asm::delay(19_200_000); // ~300 ms
        status.set_low();
        cortex_m::asm::delay(19_200_000);
    }

    // Live loop: gapless double-buffered PDM capture; one 250 ms buffer fills
    // while the previous one is classified in the callback (~130 ms compute).
    fe.reset();
    defmt::info!("listening: say 'yes' (green) or 'no' (red) near the mic");

    let mut cooldown: u32 = 0;
    let mut first = true;
    // Detection feedback: LED stays on for this many 250 ms ticks.
    let mut green_ticks: u32 = 0;
    let mut red_ticks: u32 = 0;
    let mut result: Result<()> = Ok(());
    let bufs = singleton!([[i16; CHUNK]; 2] = [[0; CHUNK]; 2]);
    let sampler_run = pdm
        .run_task_sampler(bufs, |buf| {
            // The PDM filter needs one chunk to settle; discard it.
            if first {
                first = false;
                return SamplerState::Sampled;
            }

            // Drive the detection LEDs (active low) from tick counters so
            // nothing blocks inside the 250 ms compute budget.
            if green_ticks > 0 {
                green_ticks -= 1;
                green.set_low();
            } else {
                green.set_high();
            }
            if red_ticks > 0 {
                red_ticks -= 1;
                red.set_low();
            } else {
                red.set_high();
            }

            let t0 = cortex_m::peripheral::DWT::cycle_count();
            let (mean, level) = mean_and_level(buf);
            fe.push(&buf[..], mean, GAIN);
            fe.window(&mut features);
            let (label, logits) = match classify(&ctx, &device, infer, &features, arena) {
                Ok(v) => v,
                Err(e) => {
                    result = Err(e);
                    return SamplerState::Stopped;
                }
            };

            let compute = ms_since(t0);
            if compute > 240 {
                defmt::warn!("compute {} ms exceeds the 250 ms chunk budget", compute);
            }

            let mut sorted = logits;
            sorted.sort_unstable_by(|a, b| b.total_cmp(a));
            let margin = sorted[0] - sorted[1];
            cooldown = cooldown.saturating_sub(1);
            if (label == "yes" || label == "no")
                && level >= SPEECH_LEVEL
                && margin >= DETECT_MARGIN
                && cooldown == 0
            {
                defmt::info!("==> DETECTED '{}' (margin {})", label, margin);
                if label == "yes" {
                    green_ticks = 2; // ~500 ms green
                } else {
                    red_ticks = 2; // ~500 ms red
                }
                cooldown = 4; // suppress duplicates for ~1 s
            }
            defmt::debug!(
                "heard: {} | level = {} | compute {} ms | logits = {}",
                label,
                level,
                compute,
                logits
            );
            SamplerState::Sampled
        })
        .await;

    if sampler_run.is_err() {
        defmt::error!("PDM sampler stopped unexpectedly");
        fatal_blink(status);
    }
    result
}

/// Run the model over a 49x40 feature window.
fn classify(
    ctx: &Context,
    device: &Device,
    infer: iree_embedded::Function,
    features: &[u8; FEATURE_BYTES],
    arena: &Arena,
) -> Result<(&'static str, [f32; 4])> {
    let input = Tensor::from_u8(device, &[1, 49, 40, 1], features)?;
    let outputs = ctx.invoke(infer, &[&input], arena)?;
    let mut logits = [0.0f32; 4];
    outputs[0].read_into_f32(device, &mut logits)?;

    let best = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .unwrap();
    Ok((LABELS[best], logits))
}

/// DC mean of a chunk and its post-gain mean absolute level (loudness).
fn mean_and_level(chunk: &[i16]) -> (i32, i32) {
    let n = chunk.len() as i32;
    let mut sum: i32 = 0;
    for &x in chunk {
        sum += x as i32;
    }
    let mean = sum / n;
    let mut dev: i32 = 0;
    for &x in chunk {
        dev += (x as i32 - mean).abs();
    }
    (mean, dev / n * GAIN)
}
