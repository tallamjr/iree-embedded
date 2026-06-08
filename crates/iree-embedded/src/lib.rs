#![cfg_attr(not(test), no_std)]

/// Embed a compiled `.vmfb` as a 16-byte-aligned `&'static [u8]`.
///
/// IREE's FlatBuffer verifier requires the module header to be aligned, which a
/// plain `include_bytes!` (1-byte aligned) does not guarantee. Use this for any
/// embedded model.
#[macro_export]
macro_rules! include_vmfb {
    ($path:expr) => {{
        #[repr(C, align(16))]
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

pub use arena::Arena;
pub use context::{Context, Function};
pub use device::Device;
pub use instance::Instance;
pub(crate) use status::check;
pub use status::{Error, Result, StatusCode};
pub use tensor::Tensor;
