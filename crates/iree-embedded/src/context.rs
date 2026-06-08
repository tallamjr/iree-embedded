//! Loading a compiled module and invoking its functions.
//!
//! A `Context` binds the HAL module (wrapping the device) and the bytecode
//! module (the `.vmfb`) so functions can be resolved and invoked.

use core::marker::PhantomData;

use crate::{check, Arena, Device, Instance, Result, Tensor};
use iree_embedded_sys as sys;

/// A resolved entry-point function. `iree_vm_function_t` is a plain value
/// handle (not refcounted), so this is `Copy`.
#[derive(Clone, Copy)]
pub struct Function {
    raw: sys::iree_vm_function_t,
}

pub struct Context<'i> {
    raw: *mut sys::iree_vm_context_t,
    _instance: PhantomData<&'i Instance>,
}

impl<'i> Context<'i> {
    pub fn new(
        instance: &'i Instance,
        device: &Device,
        vmfb: &'static [u8],
        arena: &Arena,
    ) -> Result<Self> {
        let alloc = arena.as_iree_allocator();
        // SAFETY: all handles are created/owned here; out-pointers are valid.
        unsafe {
            // The HAL module is built over a device group; the group is only
            // needed during module creation and released immediately after.
            let mut group = core::ptr::null_mut();
            check(sys::iree_hal_device_group_create_from_device(
                device.raw(),
                alloc,
                &mut group,
            ))?;
            let mut hal_module = core::ptr::null_mut();
            let status = sys::iree_hal_module_create(
                instance.raw(),
                sys::iree_hal_module_device_policy_default(),
                group,
                sys::IREE_HAL_MODULE_FLAG_SYNCHRONOUS as _,
                sys::iree_hal_module_debug_sink_null(),
                alloc,
                &mut hal_module,
            );
            sys::iree_hal_device_group_release(group);
            check(status)?;

            // Bytecode module from the embedded .vmfb bytes (not copied).
            let mut bytecode = core::ptr::null_mut();
            let bc = sys::iree_vm_bytecode_module_create(
                instance.raw(),
                sys::IREE_VM_BYTECODE_MODULE_FLAG_NONE as _,
                sys::iree_make_const_byte_span(vmfb.as_ptr() as *const _, vmfb.len()),
                sys::iree_allocator_null(),
                alloc,
                &mut bytecode,
            );
            if !bc.is_null() {
                sys::iree_vm_module_release(hal_module);
                check(bc)?;
            }

            let mut modules = [hal_module, bytecode];
            let mut raw = core::ptr::null_mut();
            let ctx = sys::iree_vm_context_create_with_modules(
                instance.raw(),
                sys::IREE_VM_CONTEXT_FLAG_NONE as _,
                modules.len() as sys::iree_host_size_t,
                modules.as_mut_ptr(),
                alloc,
                &mut raw,
            );
            sys::iree_vm_module_release(hal_module);
            sys::iree_vm_module_release(bytecode);
            check(ctx)?;
            Ok(Context {
                raw,
                _instance: PhantomData,
            })
        }
    }

    pub fn resolve(&self, name: &str) -> Result<Function> {
        let mut raw: sys::iree_vm_function_t = unsafe { core::mem::zeroed() };
        // SAFETY: name is a valid UTF-8 slice; out-pointer is valid.
        unsafe {
            check(sys::iree_vm_context_resolve_function(
                self.raw,
                sys::iree_string_view_t {
                    data: name.as_ptr() as *const _,
                    size: name.len(),
                },
                &mut raw,
            ))?;
        }
        Ok(Function { raw })
    }

    /// Synchronously invoke `function` with the given tensor inputs, returning
    /// the output tensors.
    pub fn invoke(
        &self,
        function: Function,
        inputs: &[&Tensor],
        arena: &Arena,
    ) -> Result<heapless::Vec<Tensor, 8>> {
        let alloc = arena.as_iree_allocator();
        // SAFETY: lists and refs are created/owned here and released below.
        unsafe {
            let mut in_list = core::ptr::null_mut();
            check(sys::iree_vm_list_create(
                sys::iree_vm_make_undefined_type_def(),
                inputs.len() as sys::iree_host_size_t,
                alloc,
                &mut in_list,
            ))?;
            for t in inputs {
                // retain_ref takes its own reference; the Tensor keeps its own.
                let mut r = sys::iree_hal_buffer_view_retain_ref(t.raw());
                let st = sys::iree_vm_list_push_ref_move(in_list, &mut r);
                if !st.is_null() {
                    sys::iree_vm_ref_release(&mut r);
                    sys::iree_vm_list_release(in_list);
                    check(st)?;
                }
            }

            let mut out_list = core::ptr::null_mut();
            let oc = sys::iree_vm_list_create(
                sys::iree_vm_make_undefined_type_def(),
                8,
                alloc,
                &mut out_list,
            );
            if !oc.is_null() {
                sys::iree_vm_list_release(in_list);
                check(oc)?;
            }

            let status = sys::iree_vm_invoke(
                self.raw,
                function.raw,
                sys::IREE_VM_INVOCATION_FLAG_NONE as _,
                core::ptr::null(),
                in_list,
                out_list,
                alloc,
            );
            sys::iree_vm_list_release(in_list);
            if !status.is_null() {
                sys::iree_vm_list_release(out_list);
                check(status)?;
            }

            let count = sys::iree_vm_list_size(out_list);
            let mut results: heapless::Vec<Tensor, 8> = heapless::Vec::new();
            for i in 0..count {
                let mut r: sys::iree_vm_ref_t = core::mem::zeroed();
                // get_ref_retain hands us a +1 reference; deref reads the
                // pointer, and the Tensor takes ownership of that reference.
                if sys::iree_vm_list_get_ref_retain(out_list, i, &mut r).is_null() {
                    let bv = sys::iree_hal_buffer_view_deref(r);
                    let _ = results.push(Tensor::from_raw(bv));
                }
            }
            sys::iree_vm_list_release(out_list);
            Ok(results)
        }
    }
}

impl<'i> Drop for Context<'i> {
    fn drop(&mut self) {
        // SAFETY: raw was created by iree_vm_context_create_with_modules.
        unsafe { sys::iree_vm_context_release(self.raw) };
    }
}
