//! Int8 model data for the TFLite-Micro baseline test (feature `int8`).
//!
//! The model is the upstream int8 micro_speech, cut before the softmax. The
//! input is a fixed upstream feature file, not the output of `kws-frontend`.
//! See `models/int8/README.md` for where each file comes from.

use iree_embedded::{Device, Result, Tensor, include_vmfb};

/// Kernels are in `models/micro_speech_int8.o`; run from flash.
pub static VMFB: &[u8] = include_vmfb!("../models/micro_speech_int8.vmfb");

/// Arena size in bytes. Set `IREE_ARENA_BYTES` at build time to change it.
pub const ARENA_BYTES: usize = parse_decimal(env!("IREE_ARENA_BYTES_RESOLVED"));

/// Parse a decimal number in a constant. `build.rs` checked the text.
const fn parse_decimal(text: &str) -> usize {
    let digits = text.as_bytes();
    let mut value = 0;
    let mut index = 0;
    while index < digits.len() {
        value = value * 10 + (digits[index] - b'0') as usize;
        index += 1;
    }
    value
}

/// 49 frames of 40 int8 values for the upstream "yes" clip.
const FEATURE_COUNT: usize = 49 * 40;
static FEATURES: &[u8; FEATURE_COUNT] = include_bytes!("../models/int8/yes_features_int8.bin");

/// Model input shape: one flat row of features.
pub const INPUT_SHAPE: [usize; 2] = [1, FEATURE_COUNT];

/// Dequantised logits that the TFLite reference interpreter gives on the
/// fixed features (`models/int8/yes_logits_tflite.txt`).
pub const EXPECTED_LOGITS: [f32; 4] = [-5.8708, -1.6512, 9.8153, -1.6512];

/// Output scale of the model: one quantisation step in logit units.
const ONE_STEP: f32 = 0.091_731_92;

/// The fixed features as device input.
pub fn input_tensor(device: &Device) -> Result<Tensor> {
    let features: &[i8] = bytemuck::cast_slice(FEATURES);
    Tensor::from_i8(device, &INPUT_SHAPE, features)
}

/// True when every logit is within one quantisation step of the reference.
pub fn matches_reference(logits: &[f32; 4]) -> bool {
    logits
        .iter()
        .zip(EXPECTED_LOGITS)
        .all(|(actual, expected)| (actual - expected).abs() <= ONE_STEP)
}
