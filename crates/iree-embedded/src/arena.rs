//! A fixed-buffer allocator exposed to IREE through its `iree_allocator_t`
//! vtable. The buffer is supplied by the caller (a `Vec` on the host, a
//! `static mut` on the board), so there is no global heap and memory use is
//! bounded and known.

use core::alloc::Layout;
use core::ffi::c_void;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use iree_embedded_sys as sys;
use spin::Mutex;
use talc::{ClaimOnOom, Span, Talc};

/// Byte length of the most recent allocation the arena could not satisfy (0 if
/// none). Useful for diagnosing on-device out-of-memory failures.
pub static LAST_ALLOC_FAIL_SIZE: AtomicUsize = AtomicUsize::new(0);

/// Bytes reserved before each allocation to store its size (IREE's `free` does
/// not pass the size back, so we record it). Also keeps user data 16-aligned.
const HEADER: usize = 16;
/// IREE's default allocation alignment (`iree_max_align_t` on 64-bit targets).
const ALIGN: usize = 16;

pub struct Arena {
    talc: Mutex<Talc<ClaimOnOom>>,
}

impl Arena {
    /// Build an arena over `buffer`. The arena (and every IREE object created
    /// with it) must not outlive the buffer; `'static` enforces that here.
    pub fn new(buffer: &'static mut [u8]) -> Self {
        let span = Span::from_base_size(buffer.as_mut_ptr(), buffer.len());
        // SAFETY: the buffer is exclusively owned by this Talc for its life.
        let talc = Talc::new(unsafe { ClaimOnOom::new(span) });
        Arena {
            talc: Mutex::new(talc),
        }
    }

    /// An `iree_allocator_t` backed by this arena. The arena must outlive it.
    pub fn as_iree_allocator(&self) -> sys::iree_allocator_t {
        sys::iree_allocator_t {
            self_: self as *const Arena as *mut c_void,
            ctl: Some(arena_ctl),
        }
    }

    fn alloc(&self, byte_length: usize, zero: bool) -> *mut c_void {
        let Ok(layout) = Layout::from_size_align(byte_length + HEADER, ALIGN) else {
            return core::ptr::null_mut();
        };
        let mut talc = self.talc.lock();
        // SAFETY: layout has non-zero size (HEADER > 0).
        let base = match unsafe { talc.malloc(layout) } {
            Ok(p) => p.as_ptr(),
            Err(_) => {
                LAST_ALLOC_FAIL_SIZE.store(byte_length, Ordering::Relaxed);
                return core::ptr::null_mut();
            }
        };
        // SAFETY: base points to a fresh block of `byte_length + HEADER` bytes.
        unsafe {
            (base as *mut usize).write(byte_length);
            let user = base.add(HEADER);
            if zero {
                core::ptr::write_bytes(user, 0, byte_length);
            }
            user as *mut c_void
        }
    }

    /// SAFETY: `user` must be null or a pointer previously returned by `alloc`.
    unsafe fn free(&self, user: *mut c_void) {
        if user.is_null() {
            return;
        }
        // SAFETY: per the caller contract, a length header written by `alloc`
        // sits HEADER bytes below `user`.
        unsafe {
            let base = (user as *mut u8).sub(HEADER);
            let byte_length = (base as *const usize).read();
            let layout = Layout::from_size_align_unchecked(byte_length + HEADER, ALIGN);
            let mut talc = self.talc.lock();
            talc.free(NonNull::new_unchecked(base), layout);
        }
    }

    /// SAFETY: `user` must be null or a pointer previously returned by `alloc`.
    unsafe fn realloc(&self, user: *mut c_void, new_len: usize) -> *mut c_void {
        if user.is_null() {
            return self.alloc(new_len, false);
        }
        // SAFETY: per the caller contract, a length header written by `alloc`
        // sits HEADER bytes below `user`; both blocks are at least
        // `old_len.min(new_len)` bytes.
        unsafe {
            let base = (user as *mut u8).sub(HEADER);
            let old_len = (base as *const usize).read();
            let new_ptr = self.alloc(new_len, false);
            if new_ptr.is_null() {
                return core::ptr::null_mut();
            }
            core::ptr::copy_nonoverlapping(
                user as *const u8,
                new_ptr as *mut u8,
                old_len.min(new_len),
            );
            self.free(user);
            new_ptr
        }
    }
}

// SAFETY: all interior mutation goes through the Mutex.
unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}

/// IREE routes malloc/calloc/realloc/free through this single control function.
/// SAFETY: invoked by IREE with a `self` pointer to a live `Arena`.
unsafe extern "C" fn arena_ctl(
    self_: *mut c_void,
    command: sys::iree_allocator_command_t,
    params: *const c_void,
    inout_ptr: *mut *mut c_void,
) -> sys::iree_status_t {
    // SAFETY: IREE invokes this with `self_` pointing at a live `Arena` and
    // `params`/`inout_ptr` valid for the given command, per the
    // `iree_allocator_ctl_fn_t` contract.
    unsafe {
        let arena = &*(self_ as *const Arena);
        let cmd = command;

        if cmd == sys::IREE_ALLOCATOR_COMMAND_FREE {
            arena.free(*inout_ptr);
            *inout_ptr = core::ptr::null_mut();
            return ok();
        }
        if cmd == sys::IREE_ALLOCATOR_COMMAND_MALLOC || cmd == sys::IREE_ALLOCATOR_COMMAND_CALLOC {
            let byte_length = (*(params as *const sys::iree_allocator_alloc_params_t)).byte_length;
            let zero = cmd == sys::IREE_ALLOCATOR_COMMAND_CALLOC;
            let p = arena.alloc(byte_length, zero);
            if p.is_null() {
                return oom();
            }
            *inout_ptr = p;
            return ok();
        }
        if cmd == sys::IREE_ALLOCATOR_COMMAND_REALLOC {
            let new_len = (*(params as *const sys::iree_allocator_alloc_params_t)).byte_length;
            let p = arena.realloc(*inout_ptr, new_len);
            if p.is_null() {
                return oom();
            }
            *inout_ptr = p;
            return ok();
        }
        unimplemented_status()
    }
}

#[inline]
fn ok() -> sys::iree_status_t {
    core::ptr::null_mut() // IREE_STATUS_OK
}
#[inline]
fn oom() -> sys::iree_status_t {
    sys::IREE_STATUS_RESOURCE_EXHAUSTED as usize as sys::iree_status_t
}
#[inline]
fn unimplemented_status() -> sys::iree_status_t {
    sys::IREE_STATUS_UNIMPLEMENTED as usize as sys::iree_status_t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_free_roundtrip() {
        static mut BUF: [u8; 64 * 1024] = [0; 64 * 1024];
        // SAFETY: single-threaded test with exclusive access to BUF.
        let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(BUF)) };
        let allocator = arena.as_iree_allocator();
        unsafe {
            let mut p: *mut c_void = core::ptr::null_mut();
            let st = sys::iree_allocator_malloc(allocator, 128, &mut p);
            assert!(st.is_null(), "malloc returned non-OK status");
            assert!(!p.is_null());
            // Write through the whole block to prove it is usable.
            core::ptr::write_bytes(p as *mut u8, 0xAB, 128);
            sys::iree_allocator_free(allocator, p);
        }
    }

    #[test]
    fn many_allocs_do_not_leak_across_reuse() {
        static mut BUF: [u8; 64 * 1024] = [0; 64 * 1024];
        // SAFETY: single-threaded test with exclusive access to BUF.
        let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(BUF)) };
        let allocator = arena.as_iree_allocator();
        // Allocate and free repeatedly; a leaking allocator would exhaust the
        // 64 KiB arena well before 10_000 iterations of 256 bytes.
        for _ in 0..10_000 {
            unsafe {
                let mut p: *mut c_void = core::ptr::null_mut();
                let st = sys::iree_allocator_malloc(allocator, 256, &mut p);
                assert!(st.is_null());
                assert!(!p.is_null());
                sys::iree_allocator_free(allocator, p);
            }
        }
    }
}
