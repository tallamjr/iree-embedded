#![no_std]
#![no_main]
// The promise of this example: running IREE inference on bare metal needs no
// unsafe code in user firmware. The irreducible unsafe (static buffers, the
// kernel query symbol, libc stubs) lives behind iree-embedded's macros.
#![forbid(unsafe_code)]

use core::sync::atomic::Ordering;

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::saadc::{
    CallbackResult, ChannelConfig, Config, Gain, Reference, Resolution, Saadc, Time,
};
use embassy_nrf::timer::Frequency;
use embassy_nrf::{bind_interrupts, saadc};
use iree_embedded::{
    Arena, Context, Device, Instance, Result, Tensor, include_vmfb, link_kernels, singleton,
};
use kws_frontend::{FEATURE_BYTES, Frontend};
use panic_probe as _;

bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});

// The keyword-spotting model (TFLite-Micro micro_speech, softmax stripped).
// The VM program is embedded here; its kernels are real Cortex-M machine code
// statically linked into this firmware (models/micro_speech.o) and executed in
// place from flash by the static-library loader.
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

// ... and the window must contain real acoustic energy. At the SAADC's
// 0..150 mV operating point, quiet-room ambient measures level ~96-112 and
// near-mic speech 200-900 (calibrated on hardware), so gate between them.
const SPEECH_LEVEL: i32 = 160;

// Digital gain applied after DC removal; the mic swings only millivolts.
const GAIN: i32 = 16;

// 16 kHz mono. One SAADC buffer is 250 ms; the front end runs continuously and
// the rolling 1 s spectrogram is classified each time a buffer completes.
const SAMPLE_RATE: u32 = 16_000;
const CHUNK: usize = SAMPLE_RATE as usize / 4;

// newlib's malloc references _sbrk; the IREE runtime allocates from the
// arena instead, so the libc heap can never grow.
iree_embedded::libc_stubs!();

/// Milliseconds elapsed since `start` (DWT cycle counter, 64 MHz core clock).
fn ms_since(start: u32) -> u32 {
    cortex_m::peripheral::DWT::cycle_count().wrapping_sub(start) / 64_000
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    defmt::info!("iree-embedded KWS: live mic -> front end -> micro_speech on micro:bit v2");

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

    // RUN_MIC (P0.20) is the mic's power supply, not an enable line: it feeds
    // the mic (105R) and a ~19 mA LED (68R), so it must be high-drive or the
    // rail sags. Give it ~100 ms to settle.
    let _run_mic = Output::new(p.P0_20, Level::High, OutputDrive::HighDrive);
    cortex_m::asm::delay(6_400_000); // ~100 ms at 64 MHz

    // SAADC on P0.05 (AIN3), single-ended. The mic signal is AC-coupled and
    // re-biased to ~97 mV, so measure 0..150 mV: gain 4 against the internal
    // 0.6 V reference, 12-bit (the operating point CODAL uses, and the one the
    // SPEECH_LEVEL/GAIN constants were calibrated against).
    let mut config = Config::default();
    config.resolution = Resolution::_12BIT;
    let mut ch = ChannelConfig::single_ended(p.P0_05);
    ch.gain = Gain::GAIN4;
    ch.reference = Reference::INTERNAL;
    ch.time = Time::_10US;
    let adc = Saadc::new(p.SAADC, Irqs, config, [ch]);
    adc.calibrate().await;

    if let Err(e) = run(&arena, fe, adc, p.TIMER0, p.PPI_CH0, p.PPI_CH1).await {
        loop {
            defmt::error!(
                "failed: {} (raw {}): {} | largest failed alloc = {} bytes",
                defmt::Debug2Format(&e.code()),
                e.raw_code(),
                e.message(),
                iree_embedded::LAST_ALLOC_FAIL_SIZE.load(Ordering::Relaxed)
            );
            cortex_m::asm::delay(64_000_000); // ~1 s
        }
    }
}

async fn run(
    arena: &Arena,
    fe: &mut Frontend,
    mut adc: Saadc<'_, 1>,
    timer: embassy_nrf::Peri<'static, embassy_nrf::peripherals::TIMER0>,
    ppi0: embassy_nrf::Peri<'static, embassy_nrf::peripherals::PPI_CH0>,
    ppi1: embassy_nrf::Peri<'static, embassy_nrf::peripherals::PPI_CH1>,
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
    // (front end + model) before live audio, which depends on mic gain and
    // room noise, runs.
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

    // Live loop: gapless double-buffered capture. `run_task_sampler` fills one
    // 250 ms buffer while the previous one is classified in the callback (~130
    // ms compute, comfortably inside the 250 ms budget).
    fe.reset();
    defmt::info!("listening: say 'yes' or 'no' near the mic (sliding 1 s window, 4x/s)");

    let mut cooldown: u32 = 0;
    let mut result: Result<()> = Ok(());
    // SAADC EasyDMA double buffer: one chunk fills while the CPU classifies
    // the other. `[samples][channels]`, one channel.
    let bufs = singleton!([[[i16; 1]; CHUNK]; 2] = [[[0; 1]; CHUNK]; 2]);
    // Internal timer at 16 MHz / 1000 = 16 kHz sample rate.
    adc.run_task_sampler(timer, ppi0, ppi1, Frequency::F16MHz, 1000, bufs, |buf| {
        let t0 = cortex_m::peripheral::DWT::cycle_count();
        let samples: &[i16] = buf.as_flattened();

        let (mean, level) = mean_and_level(samples);
        fe.push(samples, mean, GAIN);
        fe.window(&mut features);
        let (label, logits) = match classify(&ctx, &device, infer, &features, arena) {
            Ok(v) => v,
            Err(e) => {
                result = Err(e);
                return CallbackResult::Stop;
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
            cooldown = 4; // suppress duplicates for ~1 s
        }
        // Raw per-window diagnostics; compiled out at the default DEFMT_LOG
        // level (set DEFMT_LOG=debug in .cargo/config.toml when calibrating).
        defmt::debug!(
            "heard: {} | level = {} | compute {} ms | logits = {}",
            label,
            level,
            compute,
            logits
        );
        CallbackResult::Continue
    })
    .await;

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
