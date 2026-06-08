//! The VM instance: process-wide IREE state with HAL types registered.

use crate::{check, Arena, Result};
use iree_embedded_sys as sys;

pub struct Instance {
    raw: *mut sys::iree_vm_instance_t,
}

impl Instance {
    pub fn new(arena: &Arena) -> Result<Self> {
        let mut raw = core::ptr::null_mut();
        // SAFETY: out-pointer is valid; allocator outlives the instance.
        unsafe {
            check(sys::iree_vm_instance_create(
                sys::IREE_VM_TYPE_CAPACITY_DEFAULT as sys::iree_host_size_t,
                arena.as_iree_allocator(),
                &mut raw,
            ))?;
            // HAL custom types must be registered before HAL modules are used.
            check(sys::iree_hal_module_register_all_types(raw))?;
        }
        Ok(Instance { raw })
    }

    pub(crate) fn raw(&self) -> *mut sys::iree_vm_instance_t {
        self.raw
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: raw was created by iree_vm_instance_create and not released.
        unsafe { sys::iree_vm_instance_release(self.raw) };
    }
}
