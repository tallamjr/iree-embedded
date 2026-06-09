#![no_std]
#![no_main]

use core::ffi::c_void;
use core::sync::atomic::{AtomicUsize, Ordering};

use cortex_m_rt::entry;
use defmt_rtt as _;
use iree_embedded::{include_vmfb, Arena, Context, Device, Instance, Result, Tensor};
use panic_probe as _;

// The keyword-spotting model (TFLite-Micro micro_speech, softmax stripped),
// compiled for thumbv7em and embedded (16-byte aligned).
static VMFB: &[u8] = include_vmfb!("../models/micro_speech.vmfb");

// Real 1-second "yes" recording (16 kHz mono int16), embedded in flash and run
// through the on-device front end. Replace with live PDM-mic samples to do live
// keyword spotting (the front end and model are identical either way).
#[repr(C, align(4))]
struct Align4<T: ?Sized>(T);
static AUDIO: &Align4<[u8]> = &Align4(*include_bytes!("../models/yes_audio.bin"));

const LABELS: [&str; 4] = ["silence", "unknown", "yes", "no"];

// IREE arena (micro_speech runs in ~40 KB; 64 KB leaves margin and stack).
static mut HEAP: [u8; 64 * 1024] = [0; 64 * 1024];

// Small bump pool backing libc malloc/calloc for the front end's one-time state
// allocation (it never frees during operation).
const POOL: usize = 12 * 1024;
static mut FE_POOL: [u8; POOL] = [0; POOL];
static FE_OFF: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    let align = 8usize;
    let off = FE_OFF.load(Ordering::Relaxed);
    let start = (off + align - 1) & !(align - 1);
    let end = start + size;
    if end > POOL {
        return core::ptr::null_mut();
    }
    FE_OFF.store(end, Ordering::Relaxed);
    // SAFETY: single-threaded; [start, end) is a fresh, in-bounds region.
    unsafe { (core::ptr::addr_of_mut!(FE_POOL) as *mut u8).add(start) as *mut c_void }
}

#[no_mangle]
pub extern "C" fn calloc(n: usize, sz: usize) -> *mut c_void {
    let total = n.saturating_mul(sz);
    let p = malloc(total);
    if !p.is_null() {
        // SAFETY: malloc returned a valid `total`-byte region.
        unsafe { core::ptr::write_bytes(p as *mut u8, 0, total) };
    }
    p
}

#[no_mangle]
pub extern "C" fn free(_ptr: *mut c_void) {}

// newlib's malloc/_sbrk reference the `end` heap symbol; we use pools instead.
#[no_mangle]
pub extern "C" fn _sbrk(_incr: isize) -> *mut c_void {
    -1isize as *mut c_void
}

extern "C" {
    fn kws_frontend_init() -> i32;
    fn kws_features(samples: *const i16, n: i32, out: *mut u8) -> i32;
}

#[entry]
fn main() -> ! {
    defmt::info!("iree-embedded KWS: audio -> front end -> micro_speech on micro:bit v2");

    // SAFETY: single core; exclusive access to HEAP, taken once.
    let arena = unsafe { Arena::new(&mut *core::ptr::addr_of_mut!(HEAP)) };

    match run(&arena) {
        Ok(label) => defmt::info!("KWS prediction = {} (expected 'yes')", label),
        Err(e) => defmt::error!("inference failed: {}", defmt::Debug2Format(&e)),
    }

    loop {
        cortex_m::asm::wfi();
    }
}

fn run(arena: &Arena) -> Result<&'static str> {
    // 1. Compute the 49x40 uint8 spectrogram on-device from the embedded audio.
    let audio = &AUDIO.0;
    // SAFETY: the bytes are 4-aligned and an even length of little-endian i16.
    let samples: &[i16] =
        unsafe { core::slice::from_raw_parts(audio.as_ptr() as *const i16, audio.len() / 2) };
    let mut features = [0u8; 49 * 40];
    // SAFETY: features holds 49*40 bytes; samples is a valid slice.
    let frames = unsafe {
        if kws_frontend_init() == 0 {
            defmt::warn!("front end init failed");
        }
        kws_features(samples.as_ptr(), samples.len() as i32, features.as_mut_ptr())
    };
    defmt::info!("front end produced {} frames", frames);

    // 2. Run the model on the features.
    let instance = Instance::new(arena)?;
    let device = Device::local_sync(arena)?;
    let ctx = Context::new(&instance, &device, VMFB, arena)?;
    let infer = ctx.resolve("module.tf2onnx")?;

    let input = Tensor::from_u8(&device, &[1, 49, 40, 1], &features)?;
    let outputs = ctx.invoke(infer, &[&input], arena)?;
    let mut logits = [0.0f32; 4];
    outputs[0].read_into_f32(&device, &mut logits)?;

    let best = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .unwrap();
    Ok(LABELS[best])
}
