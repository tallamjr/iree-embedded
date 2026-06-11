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

#[test]
fn concurrent_takes_yield_exactly_one_winner() {
    use std::sync::{Arc, Barrier};

    fn take() -> &'static mut [u8; 8] {
        iree_embedded::singleton!([u8; 8] = [0; 8])
    }

    let threads = 8;
    let barrier = Arc::new(Barrier::new(threads));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                std::panic::catch_unwind(|| take().len()).is_ok()
            })
        })
        .collect();
    let winners = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .filter(|succeeded| *succeeded)
        .count();
    assert_eq!(winners, 1, "exactly one take must succeed under contention");
}

#[test]
fn distinct_call_sites_are_independent() {
    let a: &'static mut u8 = iree_embedded::singleton!(u8 = 1);
    let b: &'static mut u8 = iree_embedded::singleton!(u8 = 2);
    *a += 10;
    assert_eq!(*a, 11);
    assert_eq!(*b, 2);
}
