//! The safe-buffer macro works from a crate that forbids unsafe code.
//!
//! The `#![forbid(unsafe_code)]` below is the point of the test: the
//! `unsafe_code` lint is evaluated at a macro's definition site, so the
//! unsafe inside `singleton!`'s expansion (defined in `iree-embedded`)
//! must not trip this crate's forbid. If that premise ever breaks, this
//! file stops compiling.
#![forbid(unsafe_code)]

#[test]
fn singleton_returns_writable_buffer() {
    let buf: &'static mut [u8; 32] = iree_embedded::singleton!([u8; 32] = [0; 32]);
    buf[0] = 0xAA;
    buf[31] = 0x55;
    assert_eq!(buf[0], 0xAA);
    assert_eq!(buf[31], 0x55);
}

#[test]
fn singleton_supports_non_array_types() {
    let value: &'static mut u32 = iree_embedded::singleton!(u32 = 7);
    *value += 1;
    assert_eq!(*value, 8);
}

#[test]
#[should_panic(expected = "taken more than once")]
fn second_take_of_the_same_site_panics() {
    fn take() -> &'static mut [u8; 16] {
        iree_embedded::singleton!([u8; 16] = [0; 16])
    }
    let first = take();
    first[0] = 1;
    let _second = take();
}
