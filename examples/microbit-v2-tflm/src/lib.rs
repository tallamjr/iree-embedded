//! TFLite-Micro baseline for the micro:bit v2: a safe wrapper over the C++
//! interpreter, plus the model and the fixed feature data.
//!
//! Only `ffi.rs` has unsafe code. The firmware in `bin/timing_tflm.rs` has none.
#![no_std]
#![deny(unsafe_code)]

mod ffi;

pub use ffi::{double_to_i64, frexp, paint_stack, quantize_multiplier, round};

/// Core clock of the nRF52833 in Hz.
pub const CORE_CLOCK_HZ: u32 = 64_000_000;

/// True when the build uses the CMSIS-NN kernels instead of the reference kernels.
pub const CMSIS_NN: bool = cfg!(feature = "cmsis-nn");

/// Name of the kernel set in this build.
pub const KERNELS: &str = if CMSIS_NN { "cmsis-nn" } else { "reference" };

/// Labels of the four model outputs.
pub const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];

/// Pre-softmax int8 logits that the TFLite reference interpreter gives on the
/// fixed features, from the other example's `models/int8/yes_logits_tflite.txt`.
pub const EXPECTED_LOGITS: [i8; 4] = [-50, -4, 121, -4];

/// Arena size in bytes. Set `TFLM_ARENA_BYTES` at build time to change it.
pub const ARENA_BYTES: usize = parse_decimal(env!("TFLM_ARENA_BYTES_RESOLVED"));

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

/// Number of int8 values the model takes: 49 frames of 40 channels.
pub const FEATURE_COUNT: usize = 49 * 40;

/// Alignment wrapper: TFLite-Micro wants a 16-byte aligned model and arena.
#[repr(C, align(16))]
pub struct Aligned<T: ?Sized>(pub T);

/// The no-softmax int8 micro_speech model (SHA-256 34f10a8e...196a).
pub static MODEL: &Aligned<[u8]> =
    &Aligned(*include_bytes!("../models/micro_speech_int8_nosm.tflite"));

static FEATURES: &[u8; FEATURE_COUNT] = include_bytes!("../models/yes_features_int8.bin");

/// The fixed upstream features for the "yes" clip.
pub fn features() -> &'static [i8] {
    bytemuck::cast_slice(FEATURES)
}

/// Label of the largest logit. The first one wins a tie.
pub fn best_label(logits: &[i8; 4]) -> &'static str {
    let mut best = 0;
    for (index, logit) in logits.iter().enumerate() {
        if *logit > logits[best] {
            best = index;
        }
    }
    LABELS[best]
}

/// Errors from the interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Error {
    /// The model or the arena is not 16-byte aligned.
    Misaligned,
    /// An interpreter exists already.
    AlreadyCreated,
    /// The model schema version is not supported.
    ModelVersion,
    /// The arena is too small, or an operator is missing.
    Allocate,
    /// A buffer length does not match the tensor.
    Size,
    /// The model run failed.
    Invoke,
    /// An unknown shim status code.
    Shim(i32),
}

/// The TFLite-Micro interpreter. There is one per program.
pub struct Interpreter {
    _private: (),
}

impl Interpreter {
    /// Load the model and plan the tensors inside `arena`.
    pub fn new(model: &'static [u8], arena: &'static mut [u8]) -> Result<Self, Error> {
        ffi::init(model, arena)?;
        Ok(Self { _private: () })
    }

    /// Copy the features into the input tensor.
    pub fn set_input(&mut self, features: &[i8]) -> Result<(), Error> {
        ffi::set_input(features)
    }

    /// Copy the input tensor into `features`. Used to check the input copy.
    pub fn read_input(&self, features: &mut [i8]) -> Result<(), Error> {
        ffi::read_input(features)
    }

    /// Run the model once.
    pub fn invoke(&mut self) -> Result<(), Error> {
        ffi::invoke()
    }

    /// Copy the output tensor into `logits`.
    pub fn read_output(&self, logits: &mut [i8]) -> Result<(), Error> {
        ffi::read_output(logits)
    }

    /// Bytes of the arena that the interpreter uses.
    pub fn arena_used_bytes(&self) -> usize {
        ffi::arena_used()
    }
}
