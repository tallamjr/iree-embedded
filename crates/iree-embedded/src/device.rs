//! A synchronous, single-threaded CPU device using the embedded-ELF loader.
//!
//! On the host milestone the model `.vmfb` carries its kernels as embedded ELF,
//! so this loader needs no per-model state. (The MCU target will swap in the
//! static-library loader.)

use crate::{check, Arena, Result};
use iree_embedded_sys as sys;

pub struct Device {
    raw: *mut sys::iree_hal_device_t,
}

impl Device {
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

            let id = sys::iree_make_cstring_view(b"local-sync\0".as_ptr() as *const _);

            let mut device_allocator = core::ptr::null_mut();
            let status =
                sys::iree_hal_allocator_create_heap(id, alloc, alloc, &mut device_allocator);

            let mut params: sys::iree_hal_sync_device_params_t = core::mem::zeroed();
            sys::iree_hal_sync_device_params_initialize(&mut params);

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
