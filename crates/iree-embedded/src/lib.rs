#![cfg_attr(not(test), no_std)]
// `Error` inlines a 192-byte IREE status message buffer, so `Result` Err
// variants are large. Deliberate: this is `no_std` with no global allocator,
// and the most important error to report is allocator exhaustion, so the
// message must not itself allocate. The fallible calls here are millisecond
// FFI operations; a ~200-byte move on the error path is noise.
#![allow(clippy::result_large_err)]

/// Embed a compiled `.vmfb` as a 64-byte-aligned `&'static [u8]`.
///
/// IREE's FlatBuffer verifier requires the module header to be aligned, and —
/// critically — the rodata segments (model weights) inside are only used
/// *in place* when they meet HAL buffer alignment (64 bytes). An underaligned
/// module silently falls back to staging copies through the device queue,
/// which costs RAM and deadlocks the bare-metal single-threaded HAL. A plain
/// `include_bytes!` (1-byte aligned) guarantees neither; use this for any
/// embedded model.
#[macro_export]
macro_rules! include_vmfb {
    ($path:expr) => {{
        #[repr(C, align(64))]
        struct Aligned<T: ?Sized>(T);
        static ALIGNED: &Aligned<[u8]> = &Aligned(*include_bytes!($path));
        &ALIGNED.0
    }};
}

mod arena;
mod context;
mod device;
mod instance;
mod status;
mod tensor;

pub use arena::{Arena, LAST_ALLOC_FAIL_SIZE};
pub use context::{Context, Function};
pub use device::{Device, LibraryQueryFn};
pub use instance::Instance;
pub(crate) use status::check;
pub use status::{Error, Result, StatusCode};
pub use tensor::Tensor;
