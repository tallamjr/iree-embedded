//! A safe, `no_std` Rust API for machine-learning inference on Cortex-M
//! microcontrollers, built on [IREE](https://iree.dev)'s bare-metal C runtime.
//!
//! The runtime half of IREE (loading a compiled model and invoking it) is
//! wrapped in six RAII types ([`Arena`], [`Instance`], [`Device`], [`Context`],
//! [`Tensor`], [`Error`]) so leaks and double-frees are compile-time
//! impossibilities and every fallible call returns a [`Result`] carrying the
//! real IREE status message. See the repository for a complete firmware
//! example and the model-compilation workflow.
#![cfg_attr(not(test), no_std)]
#![deny(missing_docs)]
// `Error` inlines a 192-byte IREE status message buffer, so `Result` Err
// variants are large. Deliberate: this is `no_std` with no global allocator,
// and the most important error to report is allocator exhaustion, so the
// message must not itself allocate. The fallible calls here are millisecond
// FFI operations; a ~200-byte move on the error path is noise.
#![allow(clippy::result_large_err)]

/// Embed a compiled `.vmfb` as a 64-byte-aligned `&'static [u8]`.
///
/// IREE's FlatBuffer verifier requires the module header to be aligned, and,
/// critically, the rodata segments (model weights) inside are only used
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

/// Hand out a unique `&'static mut` to a static, at most once per call site.
///
/// The initialiser must be a const expression: the value is a real `static`,
/// living in `.bss`/`.data` like any other, so a multi-kilobyte arena costs
/// no stack to create (passing such a buffer *by value* through an
/// initialiser, as cell-based abstractions do, can overflow a small MCU
/// stack before the move is elided). A second take of the same call site
/// panics rather than aliasing the `&mut`.
///
/// # Panics
///
/// Panics on a second take of the same call site, and in any concurrent race
/// all but one taker panics.
///
/// # Target requirements
///
/// The guard uses an atomic swap, available on Cortex-M3 and above
/// (thumbv7/thumbv8); it is not available on thumbv6m (Cortex-M0/M0+).
///
/// ```
/// let heap: &'static mut [u8; 1024] = iree_embedded::singleton!([u8; 1024] = [0; 1024]);
/// heap[0] = 1;
/// ```
#[macro_export]
macro_rules! singleton {
    ($t:ty = $init:expr) => {{
        static TAKEN: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);
        static mut SLOT: $t = $init;
        assert!(
            // The swap is an atomic read-modify-write, so exactly one caller
            // can ever observe `false`; all concurrent racers observe `true`
            // and hit the assert. `AcqRel` is deliberately conservative; the
            // uniqueness argument needs only the RMW's atomicity.
            !TAKEN.swap(true, ::core::sync::atomic::Ordering::AcqRel),
            "iree_embedded::singleton! taken more than once"
        );
        // SAFETY: the TAKEN swap lets this expression complete at most once,
        // so the returned &mut is the only reference to SLOT for the life of
        // the program.
        unsafe { &mut *::core::ptr::addr_of_mut!(SLOT) }
    }};
}

/// Declare the query entry point of a statically linked IREE executable
/// library and yield it as a [`LibraryQueryFn`].
///
/// `iree-compile --iree-hal-target-backends=llvm-cpu` with static-library
/// output produces an object file plus a header naming its query function
/// (for example `my_model_linked_library_query`). Link the object into the
/// firmware and pass the symbol here; give the result to
/// [`Device::local_sync_static`](crate::Device::local_sync_static).
///
/// # Contract
///
/// `$sym` must name the query function of an IREE static library, emitted by
/// `iree-compile` alongside the object file (the `*_library_query` symbol in
/// its generated header). The macro declares, it cannot verify: naming any
/// other symbol misdeclares its ABI, and invoking the device on it is
/// undefined behaviour.
///
/// ```ignore
/// let device = Device::local_sync_static(
///     &arena,
///     &[iree_embedded::link_kernels!(my_model_linked_library_query)],
/// )?;
/// ```
#[macro_export]
macro_rules! link_kernels {
    ($sym:ident) => {{
        unsafe extern "C" {
            fn $sym(
                max_version: u32,
                environment: *const ::core::ffi::c_void,
            ) -> *const ::core::ffi::c_void;
        }
        $sym as $crate::LibraryQueryFn
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
