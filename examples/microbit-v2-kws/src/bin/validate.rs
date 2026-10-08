//! Validation of the cycle counter used by the timing firmware.
//!
//! Part 1 times known busy-wait delays. Part 2 times the model with three
//! different inputs. Results go out over defmt RTT.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use cortex_m::peripheral::DWT;
use cycle_stats::summarise;
use defmt_rtt as _;
use embassy_executor::Spawner;
use iree_embedded::{Arena, Context, Device, Instance, Result, Tensor, link_kernels, singleton};
use kws_frontend::{FEATURE_BYTES, Frontend};
use microbit_v2_kws::{VMFB, best_label, self_test_clip};
use panic_probe as _;

const REPEATS: usize = 20;

iree_embedded::libc_stubs!();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Default::default());
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dcb = cp.DCB;
    let mut dwt = cp.DWT;
    dcb.enable_trace();
    dwt.enable_cycle_counter();

    for delay_cycles in [100_000u32, 1_000_000, 6_400_000] {
        let start = DWT::cycle_count();
        cortex_m::asm::delay(delay_cycles);
        let counted = DWT::cycle_count().wrapping_sub(start);
        defmt::info!("delay({}) counted {} cycles", delay_cycles, counted);
    }

    let arena = Arena::new(singleton!([u8; 56 * 1024] = [0; 56 * 1024]));
    let fe = singleton!(Frontend = Frontend::new());
    match run(&arena, fe) {
        Ok(()) => defmt::info!("validation done"),
        Err(e) => defmt::error!("failed: {}", e.message()),
    }
    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &Arena, fe: &mut Frontend) -> Result<()> {
    fe.init();
    let instance = Instance::new(arena)?;
    let device = Device::local_sync_static(
        arena,
        &[link_kernels!(micro_speech_nosm_linked_library_query)],
    )?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;

    let mut yes_features = [0u8; FEATURE_BYTES];
    fe.features_oneshot(self_test_clip(), &mut yes_features);
    let silence_features = [0u8; FEATURE_BYTES];
    let mut noise_features = [0u8; FEATURE_BYTES];
    let mut state: u32 = 0x1234_5678;
    for byte in noise_features.iter_mut() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *byte = (state >> 24) as u8;
    }

    for cache_on in [false, true] {
        nrf_pac::NVMC.icachecnf().write(|w| w.set_cacheen(cache_on));
        defmt::info!("instruction cache enabled: {}", cache_on);
        for (name, features) in [
            ("yes clip", &yes_features),
            ("all zeros", &silence_features),
            ("noise", &noise_features),
        ] {
            let input = Tensor::from_u8(&device, &[1, 49, 40, 1], features)?;
            ctx.invoke(infer, &[&input], arena)?;
            let mut cycles = [0u32; REPEATS];
            let mut logits = [0.0f32; 4];
            for slot in cycles.iter_mut() {
                let start = DWT::cycle_count();
                let outputs = ctx.invoke(infer, &[&input], arena)?;
                *slot = DWT::cycle_count().wrapping_sub(start);
                outputs[0].read_into_f32(&device, &mut logits)?;
            }
            let stats = summarise(&mut cycles).expect("REPEATS is above zero");
            defmt::info!(
                "{}: label {} logits {} cycles min {} median {} max {}",
                name,
                best_label(&logits),
                logits,
                stats.min,
                stats.median,
                stats.max
            );
        }
    }
    Ok(())
}
