//! A synchronous, single-threaded CPU device.
//!
//! Two loaders are offered: the embedded-ELF loader (host milestone; the
//! `.vmfb` carries position-independent ELF kernels that are mapped into RAM)
//! and the static-library loader (MCU; the kernels are compiled into the
//! firmware image and execute in place from flash, costing no RAM).

use crate::{check, Arena, Result};
use iree_embedded_sys as sys;

/// The `*_library_query` entry point emitted by
/// `iree-compile --iree-llvmcpu-static-library-output-path=` (declared in the
/// generated header alongside the `.o`). Declared with opaque pointers so
/// firmware can `extern "C"` it without naming generated binding types; the
/// shape is ABI-identical to `iree_hal_executable_library_query_fn_t`
/// (`uint32_t`, pointer) -> pointer.
pub type LibraryQueryFn = unsafe extern "C" fn(
    max_version: u32,
    environment: *const core::ffi::c_void,
) -> *const core::ffi::c_void;

pub struct Device {
    raw: *mut sys::iree_hal_device_t,
}

impl Device {
    /// Device whose executables are statically linked into this binary.
    ///
    /// `libraries` are the query functions of every model linked into the
    /// firmware; the loader resolves a `.vmfb`'s library reference by name.
    pub fn local_sync_static(arena: &Arena, libraries: &[LibraryQueryFn]) -> Result<Self> {
        let alloc = arena.as_iree_allocator();
        // SAFETY: every out-pointer is valid; the arena outlives the device.
        unsafe {
            let mut loader = core::ptr::null_mut();
            check(sys::iree_hal_static_library_loader_create(
                libraries.len() as _,
                // A non-null `LibraryQueryFn` has the same layout as the
                // `Option`-wrapped bindgen fn pointer (niche optimization).
                libraries.as_ptr() as *const sys::iree_hal_executable_library_query_fn_t,
                sys::iree_hal_executable_import_provider_null(),
                alloc,
                &mut loader,
            ))?;
            Self::from_loader(loader, alloc)
        }
    }

    pub fn local_sync(arena: &Arena) -> Result<Self> {
        let alloc = arena.as_iree_allocator();
        // SAFETY: every out-pointer is valid; the arena outlives the device.
        unsafe {
            let mut loader = core::ptr::null_mut();
            check(sys::iree_hal_embedded_elf_loader_create(
                core::ptr::null_mut(), // plugin_manager
                alloc,
                &mut loader,
            ))?;
            Self::from_loader(loader, alloc)
        }
    }

    /// Build the local-sync device around `loader`, consuming one reference to
    /// it (released here whether or not creation succeeds).
    ///
    /// # Safety
    /// `loader` must be a valid executable loader and `alloc` must outlive the
    /// returned device.
    unsafe fn from_loader(
        mut loader: *mut sys::iree_hal_executable_loader_t,
        alloc: sys::iree_allocator_t,
    ) -> Result<Self> {
        unsafe {
            let id = sys::iree_make_cstring_view(b"local-sync\0".as_ptr() as *const _);

            let mut device_allocator = core::ptr::null_mut();
            let status =
                sys::iree_hal_allocator_create_heap(id, alloc, alloc, &mut device_allocator);

            let mut params: sys::iree_hal_sync_device_params_t = core::mem::zeroed();
            sys::iree_hal_sync_device_params_initialize(&mut params);
            // The default 32 KiB transient block is host-sized; on an MCU it
            // starves the arena. 4 KiB is the device's documented minimum and
            // blocks chain on demand.
            params.arena_block_size = 4096;

            let mut raw = core::ptr::null_mut();
            let status = if status.is_null() {
                sys::iree_hal_sync_device_create(
                    id,
                    &params,
                    1, // loader_count
                    &mut loader,
                    device_allocator,
                    alloc,
                    &mut raw,
                )
            } else {
                status
            };

            sys::iree_hal_allocator_release(device_allocator);
            sys::iree_hal_executable_loader_release(loader);
            check(status)?;
            Ok(Device { raw })
        }
    }
}

impl Device {

    pub(crate) fn raw(&self) -> *mut sys::iree_hal_device_t {
        self.raw
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: raw was created by iree_hal_sync_device_create.
        unsafe { sys::iree_hal_device_release(self.raw) };
    }
}
