//! End-to-end keyword spotting on the host: run the real TFLite-Micro
//! `micro_speech` model on real "yes" features and check it predicts "yes".
//!
//! The features were produced from a real 16 kHz "yes" recording by the
//! TFLite-Micro audio front end (see scripts in .iree/), and the model output
//! matches the reference TFLite interpreter byte-for-byte.

use iree_embedded::{include_vmfb, Arena, Context, Device, Instance, Tensor};

static VMFB: &[u8] = include_vmfb!("fixtures/micro_speech.vmfb");
static YES_FEATURES: &[u8] = include_bytes!("fixtures/yes_features.bin");

const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];

#[test]
fn micro_speech_predicts_yes() {
    static mut BUF: [u8; 4 * 1024 * 1024] = [0; 4 * 1024 * 1024];
    // SAFETY: single-threaded test with exclusive access to BUF.
    let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(BUF)) };

    let instance = Instance::new(&arena).expect("instance");
    let device = Device::local_sync(&arena).expect("device");
    let ctx = Context::new(&instance, &device, VMFB, &arena).expect("context");
    let infer = ctx.resolve("module.tf2onnx").expect("resolve");

    // micro_speech input: uint8 spectrogram [1, 49, 40, 1] = 1960 features.
    assert_eq!(YES_FEATURES.len(), 1 * 49 * 40 * 1);
    let input = Tensor::from_u8(&device, &[1, 49, 40, 1], YES_FEATURES).expect("input");

    let outputs = ctx.invoke(infer, &[&input], &arena).expect("invoke");
    // Softmax was stripped at compile time (argmax of logits == argmax of
    // softmax), so the output is f32 logits [1, 4].
    let mut logits = [0.0f32; 4];
    outputs[0].read_into_f32(&device, &mut logits).expect("read");

    let best = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .unwrap();
    assert_eq!(LABELS[best], "yes", "logits = {logits:?}");
}
