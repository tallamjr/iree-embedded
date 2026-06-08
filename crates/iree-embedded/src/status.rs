//! Mapping IREE's `iree_status_t` to a Rust `Result`.
//!
//! A status is a tagged pointer: the low 5 bits hold the code, and the OK
//! status is the all-zero (null) value. A non-OK status may own a heap message,
//! so we always free it after reading the code to avoid leaks.

use iree_embedded_sys as sys;

const CODE_MASK: usize = 0x1F;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Aborted,
    OutOfMemory,
    NotFound,
    InvalidArgument,
    Unimplemented,
    Internal,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    code: StatusCode,
}

impl Error {
    pub fn code(&self) -> StatusCode {
        self.code
    }
}

pub type Result<T> = core::result::Result<T, Error>;

/// Consume an `iree_status_t`: `Ok` for the OK (null) status, otherwise map the
/// code and free the status so any message buffer is released.
pub(crate) fn check(status: sys::iree_status_t) -> Result<()> {
    if status.is_null() {
        return Ok(());
    }
    let code = (status as usize & CODE_MASK) as u32;
    // SAFETY: `status` is a non-OK status we now own; free its message buffer.
    unsafe {
        sys::iree_status_free(status);
    }
    Err(Error { code: map(code) })
}

fn map(code: u32) -> StatusCode {
    use StatusCode::*;
    match code {
        c if c == sys::IREE_STATUS_ABORTED as u32 => Aborted,
        c if c == sys::IREE_STATUS_RESOURCE_EXHAUSTED as u32 => OutOfMemory,
        c if c == sys::IREE_STATUS_NOT_FOUND as u32 => NotFound,
        c if c == sys::IREE_STATUS_INVALID_ARGUMENT as u32 => InvalidArgument,
        c if c == sys::IREE_STATUS_UNIMPLEMENTED as u32 => Unimplemented,
        c if c == sys::IREE_STATUS_INTERNAL as u32 => Internal,
        _ => Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a code-only status (no heap message): the code in the low bits.
    fn status_of(code: u32) -> sys::iree_status_t {
        code as usize as sys::iree_status_t
    }

    #[test]
    fn ok_status_is_ok() {
        assert!(check(core::ptr::null_mut()).is_ok());
    }

    #[test]
    fn aborted_maps_to_error() {
        let st = status_of(sys::IREE_STATUS_ABORTED as u32);
        assert_eq!(check(st).unwrap_err().code(), StatusCode::Aborted);
    }

    #[test]
    fn resource_exhausted_maps_to_oom() {
        let st = status_of(sys::IREE_STATUS_RESOURCE_EXHAUSTED as u32);
        assert_eq!(check(st).unwrap_err().code(), StatusCode::OutOfMemory);
    }
}
