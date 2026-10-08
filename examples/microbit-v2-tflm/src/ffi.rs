//! The only module with unsafe code: calls into the C++ shim (`shim.h`).
//! Its public functions are safe. They keep the shim's rules (one interpreter,
//! 16-byte alignment, valid buffers).
#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, Ordering};

use crate::Error;

unsafe extern "C" {
    fn tflm_init(model: *const u8, arena: *mut u8, arena_len: usize) -> i32;
    fn tflm_set_input(data: *const i8, count: usize) -> i32;
    fn tflm_invoke() -> i32;
    fn tflm_read_output(data: *mut i8, count: usize) -> i32;
    fn tflm_arena_used() -> usize;
}

/// Set when the one interpreter exists. The shim keeps global state.
static CREATED: AtomicBool = AtomicBool::new(false);

const ALIGNMENT: usize = 16;

fn check(code: i32) -> Result<(), Error> {
    match code {
        0 => Ok(()),
        3 => Err(Error::ModelVersion),
        4 => Err(Error::Allocate),
        5 => Err(Error::Size),
        6 => Err(Error::Invoke),
        other => Err(Error::Shim(other)),
    }
}

pub fn init(model: &'static [u8], arena: &'static mut [u8]) -> Result<(), Error> {
    if !(model.as_ptr() as usize).is_multiple_of(ALIGNMENT)
        || !(arena.as_ptr() as usize).is_multiple_of(ALIGNMENT)
    {
        return Err(Error::Misaligned);
    }
    if CREATED.swap(true, Ordering::AcqRel) {
        return Err(Error::AlreadyCreated);
    }
    // SAFETY: both buffers are 'static and 16-byte aligned. CREATED makes
    // this the only call, so the shim owns the arena from here on.
    let code = unsafe { tflm_init(model.as_ptr(), arena.as_mut_ptr(), arena.len()) };
    if code != 0 {
        CREATED.store(false, Ordering::Release);
    }
    check(code)
}

pub fn set_input(data: &[i8]) -> Result<(), Error> {
    // SAFETY: the pointer and length come from one valid slice; the shim only reads.
    check(unsafe { tflm_set_input(data.as_ptr(), data.len()) })
}

pub fn invoke() -> Result<(), Error> {
    // SAFETY: the shim has no arguments; it checks its own state.
    check(unsafe { tflm_invoke() })
}

pub fn read_output(data: &mut [i8]) -> Result<(), Error> {
    // SAFETY: the pointer and length come from one valid slice; the shim writes at most `count` bytes.
    check(unsafe { tflm_read_output(data.as_mut_ptr(), data.len()) })
}

pub fn arena_used() -> usize {
    // SAFETY: the shim has no arguments; it checks its own state.
    unsafe { tflm_arena_used() }
}
