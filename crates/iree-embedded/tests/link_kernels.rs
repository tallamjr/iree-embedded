//! `link_kernels!` declares an extern query symbol and yields it as a
//! [`iree_embedded::LibraryQueryFn`]. The test provides the symbol from
//! Rust (in firmware it comes from the model's compiled object file) and
//! calls through the returned pointer, proving both linkage and ABI.
//!
//! No `#![forbid(unsafe_code)]` here: providing the `#[unsafe(no_mangle)]`
//! test symbol needs the unsafe attribute. The forbid premise for this
//! macro is proven by the example firmware, which forbids unsafe and uses
//! it against a real model object.

use core::ffi::c_void;

#[unsafe(no_mangle)]
extern "C" fn iree_embedded_link_kernels_test_query(
    max_version: u32,
    _environment: *const c_void,
) -> *const c_void {
    max_version as usize as *const c_void
}

#[test]
fn link_kernels_resolves_and_calls_the_symbol() {
    let query: iree_embedded::LibraryQueryFn =
        iree_embedded::link_kernels!(iree_embedded_link_kernels_test_query);
    // SAFETY: the symbol is the Rust function above, which only echoes its
    // first argument back as a pointer.
    let out = unsafe { query(7, core::ptr::null()) };
    assert_eq!(out as usize, 7);
}
