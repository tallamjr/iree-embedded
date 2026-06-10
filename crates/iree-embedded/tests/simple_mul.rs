//! End-to-end: load simple_mul.vmfb, invoke it, and check 4 * 2 == 8.

use iree_embedded::{Arena, Context, Device, Instance, Tensor, include_vmfb};

// Embedded-ELF kernels are architecture-specific; pick the host's build.
#[cfg(target_arch = "aarch64")]
static VMFB: &[u8] = include_vmfb!("fixtures/simple_mul-aarch64.vmfb");
#[cfg(target_arch = "x86_64")]
static VMFB: &[u8] = include_vmfb!("fixtures/simple_mul-x86_64.vmfb");

#[test]
fn simple_mul_returns_eight() {
    static mut BUF: [u8; 4 * 1024 * 1024] = [0; 4 * 1024 * 1024];
    // SAFETY: single-threaded test with exclusive access to BUF.
    let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(BUF)) };

    let instance = Instance::new(&arena).expect("instance");
    let device = Device::local_sync(&arena).expect("device");
    let ctx = Context::new(&instance, &device, VMFB, &arena).expect("context");
    let main = ctx.resolve("module.simple_mul").expect("resolve");

    let a = Tensor::from_f32(&device, &[4], &[4.0; 4]).expect("a");
    let b = Tensor::from_f32(&device, &[4], &[2.0; 4]).expect("b");

    let outputs = ctx.invoke(main, &[&a, &b], &arena).expect("invoke");
    assert_eq!(outputs.len(), 1, "expected one output tensor");

    let mut out = [0.0f32; 4];
    outputs[0].read_into_f32(&device, &mut out).expect("read");
    assert_eq!(out, [8.0, 8.0, 8.0, 8.0]);
}
