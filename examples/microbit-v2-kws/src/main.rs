#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt_rtt as _;
use iree_embedded::{include_vmfb, Arena, Context, Device, Instance, Result, Tensor};
use panic_probe as _;

// The model, compiled for thumbv7em and embedded (16-byte aligned).
static VMFB: &[u8] = include_vmfb!("../models/simple_mul.vmfb");

// Fixed arena for every IREE allocation. Sized to leave room for the stack in
// the nRF52833's 128 KB RAM.
static mut HEAP: [u8; 64 * 1024] = [0; 64 * 1024];

// IREE links newlib, whose malloc/_sbrk reference the `end` heap symbol. We
// never use the system heap (everything goes through the arena), so provide a
// failing _sbrk; this keeps newlib's real heap (and `end`) out of the image.
#[no_mangle]
pub extern "C" fn _sbrk(_incr: isize) -> *mut core::ffi::c_void {
    -1isize as *mut core::ffi::c_void
}

#[entry]
fn main() -> ! {
    defmt::info!("iree-embedded: running simple_mul on micro:bit v2");

    // SAFETY: single core; exclusive access to HEAP, taken once.
    let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(HEAP)) };

    match run(&arena) {
        Ok(out) => defmt::info!("simple_mul result = {} (expected [8, 8, 8, 8])", out),
        Err(e) => defmt::error!("inference failed: {}", defmt::Debug2Format(&e)),
    }

    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &Arena) -> Result<[f32; 4]> {
    let instance = Instance::new(arena)?;
    let device = Device::local_sync(arena)?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let main = ctx.resolve("module.simple_mul")?;

    let a = Tensor::from_f32(&device, &[4], &[4.0; 4])?;
    let b = Tensor::from_f32(&device, &[4], &[2.0; 4])?;

    let outputs = ctx.invoke(main, &[&a, &b], arena)?;
    let mut out = [0.0f32; 4];
    outputs[0].read_into_f32(&device, &mut out)?;
    Ok(out)
}
