//! Mapping IREE's `iree_status_t` to a Rust `Result`.
//!
//! A status is a tagged pointer: the low 5 bits hold the code, and the OK
//! status is the all-zero (null) value. A non-OK status may own a heap message,
//! so we always free it after reading the code to avoid leaks.

use iree_embedded_sys as sys;

const CODE_MASK: usize = 0x1F;

/// A coarse classification of an IREE failure, mapped from the raw status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    /// The operation was aborted.
    Aborted,
    /// An allocation could not be satisfied (the arena is too small).
    OutOfMemory,
    /// A requested entity (function or symbol) was not found.
    NotFound,
    /// An argument was invalid.
    InvalidArgument,
    /// The operation is not implemented for this configuration.
    Unimplemented,
    /// An internal runtime error.
    Internal,
    /// A status code this crate does not classify.
    Unknown,
}

const MSG_CAP: usize = 192;

/// An IREE failure: a classified [`StatusCode`], the raw code, and the
/// formatted status message (source location and annotations).
#[derive(Clone, Copy)]
pub struct Error {
    code: StatusCode,
    raw: u32,
    msg: [u8; MSG_CAP],
    msg_len: usize,
}

impl Error {
    /// The classified status code.
    pub fn code(&self) -> StatusCode {
        self.code
    }

    /// The raw IREE status code (the low 5 bits of the status).
    pub fn raw_code(&self) -> u32 {
        self.raw
    }

    /// The formatted IREE status message (source location + annotations), if any.
    pub fn message(&self) -> &str {
        core::str::from_utf8(&self.msg[..self.msg_len]).unwrap_or("<non-utf8>")
    }
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Error({:?}, raw={}, {})",
            self.code,
            self.raw,
            self.message()
        )
    }
}

/// The result type returned by fallible operations in this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Consume an `iree_status_t`: `Ok` for the OK (null) status, otherwise map the
/// code and free the status so any message buffer is released.
pub(crate) fn check(status: sys::iree_status_t) -> Result<()> {
    if status.is_null() {
        return Ok(());
    }
    let code = (status as usize & CODE_MASK) as u32;
    let mut msg = [0u8; MSG_CAP];
    let mut len: sys::iree_host_size_t = 0;
    // SAFETY: format into our buffer, then free the status (we own it).
    unsafe {
        sys::iree_status_format(status, msg.len(), msg.as_mut_ptr() as *mut _, &mut len);
        sys::iree_status_free(status);
    }
    Err(Error {
        code: map(code),
        raw: code,
        msg,
        msg_len: (len as usize).min(MSG_CAP),
    })
}

fn map(code: u32) -> StatusCode {
    use StatusCode::*;
    match code {
        sys::IREE_STATUS_ABORTED => Aborted,
        sys::IREE_STATUS_RESOURCE_EXHAUSTED => OutOfMemory,
        sys::IREE_STATUS_NOT_FOUND => NotFound,
        sys::IREE_STATUS_INVALID_ARGUMENT => InvalidArgument,
        sys::IREE_STATUS_UNIMPLEMENTED => Unimplemented,
        sys::IREE_STATUS_INTERNAL => Internal,
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
