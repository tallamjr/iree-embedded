//! Device buffers with shape and dtype, wrapping `iree_hal_buffer_view_t`.

use core::ffi::c_void;

use crate::{check, Device, Result};
use iree_embedded_sys as sys;

pub struct Tensor {
    raw: *mut sys::iree_hal_buffer_view_t,
}

impl Tensor {
    /// Allocate a device-local f32 buffer view, copying `data` in.
    pub fn from_f32(device: &Device, shape: &[usize], data: &[f32]) -> Result<Self> {
        Self::from_bytes(
            device,
            shape,
            sys::IREE_HAL_ELEMENT_TYPE_FLOAT_32 as u32,
            data.as_ptr() as *const c_void,
            core::mem::size_of_val(data),
        )
    }

    /// Allocate a device-local u8 buffer view, copying `data` in (e.g. an
    /// int8/uint8 quantized model input).
    pub fn from_u8(device: &Device, shape: &[usize], data: &[u8]) -> Result<Self> {
        Self::from_bytes(
            device,
            shape,
            sys::IREE_HAL_ELEMENT_TYPE_UINT_8 as u32,
            data.as_ptr() as *const c_void,
            data.len(),
        )
    }

    /// Copy the buffer contents back to the host as f32.
    pub fn read_into_f32(&self, device: &Device, out: &mut [f32]) -> Result<()> {
        self.read_bytes(device, out.as_mut_ptr() as *mut c_void, core::mem::size_of_val(out))
    }

    /// Copy the buffer contents back to the host as u8.
    pub fn read_into_u8(&self, device: &Device, out: &mut [u8]) -> Result<()> {
        self.read_bytes(device, out.as_mut_ptr() as *mut c_void, out.len())
    }

    fn from_bytes(
        device: &Device,
        shape: &[usize],
        element_type: u32,
        data: *const c_void,
        len: usize,
    ) -> Result<Self> {
        let dims: heapless::Vec<sys::iree_hal_dim_t, 8> =
            shape.iter().map(|&d| d as sys::iree_hal_dim_t).collect();
        let params = sys::iree_hal_buffer_params_t {
            usage: sys::IREE_HAL_BUFFER_USAGE_DEFAULT as _,
            type_: sys::IREE_HAL_MEMORY_TYPE_DEVICE_LOCAL as _,
            ..unsafe { core::mem::zeroed() }
        };
        let mut raw = core::ptr::null_mut();
        // SAFETY: data/len describe a valid initial-contents span; dims is valid.
        unsafe {
            check(sys::iree_hal_buffer_view_allocate_buffer_copy(
                device.raw(),
                sys::iree_hal_device_allocator(device.raw()),
                dims.len() as sys::iree_host_size_t,
                dims.as_ptr(),
                element_type as _,
                sys::IREE_HAL_ENCODING_TYPE_DENSE_ROW_MAJOR as _,
                params,
                sys::iree_make_const_byte_span(data, len),
                &mut raw,
            ))?;
        }
        Ok(Tensor { raw })
    }

    fn read_bytes(&self, device: &Device, out: *mut c_void, len: usize) -> Result<()> {
        // SAFETY: out/len describe a valid mutable span; the buffer outlives the call.
        unsafe {
            check(sys::iree_hal_device_transfer_d2h(
                device.raw(),
                sys::iree_hal_buffer_view_buffer(self.raw),
                0,
                out,
                len as sys::iree_device_size_t,
                sys::IREE_HAL_TRANSFER_BUFFER_FLAG_DEFAULT as _,
                sys::iree_infinite_timeout(),
            ))?;
        }
        Ok(())
    }

    pub(crate) fn raw(&self) -> *mut sys::iree_hal_buffer_view_t {
        self.raw
    }

    /// Wrap a buffer view whose reference this `Tensor` now owns.
    pub(crate) fn from_raw(raw: *mut sys::iree_hal_buffer_view_t) -> Self {
        Tensor { raw }
    }
}

impl Drop for Tensor {
    fn drop(&mut self) {
        // SAFETY: raw is an owned buffer-view reference.
        unsafe { sys::iree_hal_buffer_view_release(self.raw) };
    }
}
