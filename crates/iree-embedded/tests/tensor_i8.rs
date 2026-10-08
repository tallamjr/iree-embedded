//! `Tensor::from_i8` on the host: the real int8 micro_speech model on the
//! fixed upstream "yes" features.

use iree_embedded::{Arena, Context, Device, Instance, Tensor, include_vmfb};

// Embedded-ELF kernels are architecture-specific; pick the host's build.
#[cfg(target_arch = "aarch64")]
static VMFB: &[u8] = include_vmfb!("fixtures/micro_speech_int8-aarch64.vmfb");
#[cfg(target_arch = "x86_64")]
static VMFB: &[u8] = include_vmfb!("fixtures/micro_speech_int8-x86_64.vmfb");
static YES_FEATURES_INT8: &[u8] =
    include_bytes!("../../../examples/microbit-v2-kws/models/int8/yes_features_int8.bin");

const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];
// Pre-softmax int8 [-50, -4, 121, -4], scale 0.09173192, zero point 14.
const EXPECTED_LOGITS: [f32; 4] = [-5.8708, -1.6512, 9.8153, -1.6512];
const ONE_QUANTISATION_STEP: f32 = 0.091_731_92;

#[test]
fn int8_micro_speech_predicts_yes() {
    static mut BUF: [u8; 4 * 1024 * 1024] = [0; 4 * 1024 * 1024];
    // SAFETY: single-threaded test with exclusive access to BUF.
    let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(BUF)) };

    let instance = Instance::new(&arena).expect("instance");
    let device = Device::local_sync(&arena).expect("device");
    let ctx = Context::new(&instance, &device, VMFB, &arena).expect("context");
    let infer = ctx.resolve("module.tf2onnx").expect("resolve");

    assert_eq!(YES_FEATURES_INT8.len(), 49 * 40);
    let features: Vec<i8> = YES_FEATURES_INT8.iter().map(|&byte| byte as i8).collect();
    let input = Tensor::from_i8(&device, &[1, 1960], &features).expect("input");

    let outputs = ctx.invoke(infer, &[&input], &arena).expect("invoke");
    let mut logits = [0.0f32; 4];
    outputs[0]
        .read_into_f32(&device, &mut logits)
        .expect("read");

    let best = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
        .unwrap();
    assert_eq!(LABELS[best], "yes", "logits = {logits:?}");
    for (actual, expected) in logits.iter().zip(EXPECTED_LOGITS) {
        assert!(
            (actual - expected).abs() <= ONE_QUANTISATION_STEP,
            "logits = {logits:?}, expected {EXPECTED_LOGITS:?}"
        );
    }
}
