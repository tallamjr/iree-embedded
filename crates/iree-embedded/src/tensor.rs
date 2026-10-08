//! Device buffers with shape and dtype, wrapping `iree_hal_buffer_view_t`.

use core::ffi::c_void;

use crate::{Device, Result, check};
use iree_embedded_sys as sys;

/// A device buffer with shape and element type: an input to or output from
/// [`Context::invoke`](crate::Context::invoke).
pub struct Tensor {
    raw: *mut sys::iree_hal_buffer_view_t,
}

impl Tensor {
    /// Allocate a device-local f32 buffer view, copying `data` in.
    pub fn from_f32(device: &Device, shape: &[usize], data: &[f32]) -> Result<Self> {
        Self::from_bytes(
            device,
            shape,
            sys::IREE_HAL_ELEMENT_TYPE_FLOAT_32,
            data.as_ptr() as *const c_void,
            core::mem::size_of_val(data),
        )
    }

    /// Allocate a device-local i8 buffer view, copying `data` in (e.g. an
    /// int8 quantized model input).
    pub fn from_i8(device: &Device, shape: &[usize], data: &[i8]) -> Result<Self> {
        Self::from_bytes(
            device,
            shape,
            sys::IREE_HAL_ELEMENT_TYPE_SINT_8,
            data.as_ptr() as *const c_void,
            data.len(),
        )
    }

    /// Allocate a device-local u8 buffer view, copying `data` in (e.g. a
    /// uint8 quantized model input).
    pub fn from_u8(device: &Device, shape: &[usize], data: &[u8]) -> Result<Self> {
        Self::from_bytes(
            device,
            shape,
            sys::IREE_HAL_ELEMENT_TYPE_UINT_8,
            data.as_ptr() as *const c_void,
            data.len(),
        )
    }

    /// Copy the buffer contents back to the host as f32.
    pub fn read_into_f32(&self, device: &Device, out: &mut [f32]) -> Result<()> {
        self.read_bytes(
            device,
            out.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(out),
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Arena;

    #[test]
    fn from_i8_makes_an_int8_buffer_with_the_given_bytes() {
        static mut BUF: [u8; 1024 * 1024] = [0; 1024 * 1024];
        // SAFETY: single-threaded test with exclusive access to BUF.
        let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(BUF)) };
        let device = Device::local_sync(&arena).expect("device");

        let values: [i8; 6] = [-128, -50, -1, 0, 1, 127];
        let tensor = Tensor::from_i8(&device, &[2, 3], &values).expect("tensor");

        // SAFETY: raw is a valid buffer view owned by tensor.
        let (element_type, rank) = unsafe {
            (
                sys::iree_hal_buffer_view_element_type(tensor.raw()),
                sys::iree_hal_buffer_view_shape_rank(tensor.raw()),
            )
        };
        assert_eq!(element_type, sys::IREE_HAL_ELEMENT_TYPE_SINT_8);
        assert_eq!(rank, 2);

        let mut bytes = [0u8; 6];
        tensor.read_into_u8(&device, &mut bytes).expect("read");
        assert_eq!(bytes, values.map(|value| value as u8));
    }
}
