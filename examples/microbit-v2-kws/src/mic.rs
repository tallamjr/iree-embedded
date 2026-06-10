//! Live microphone capture for the micro:bit v2.
//!
//! The onboard Knowles SPU0410LR5H is an **analog** MEMS mic: its output sits
//! on a ~VDD/2 bias on P0.05 (AIN3) and the mic is powered by driving P0.20
//! ("RUN_MIC") high. The SAADC samples it at 16 kHz using its local timer.
//!
//! Capture is **continuous**: EasyDMA fills 250 ms chunks of a ring buffer,
//! and a PPI channel chains the SAADC END event to its own START task, so
//! buffer switching is hardware-timed and capture never pauses while the CPU
//! classifies. Software only has to repoint `RESULT.PTR` once per chunk,
//! within the 250 ms it takes the next chunk to fill.

use nrf52833_pac as pac;

/// 16 kHz mono, matching the front end's `FrontendPopulateState(.., 16000)`.
pub const SAMPLE_RATE: usize = 16_000;
/// Capture chunk: 250 ms. Per-chunk compute must stay under this.
pub const CHUNK: usize = SAMPLE_RATE / 4;
/// Double buffer: DMA fills one slot while the CPU consumes the other.
pub const RING: usize = 2;

/// Digital gain after DC removal (applied downstream, in the front end). The
/// mic swings only tens of millivolts around its bias; the model expects
/// speech to span a good part of the int16 range.
pub const GAIN: i32 = 16;

pub struct Mic {
    saadc: pac::SAADC,
    ring: *mut [i16; CHUNK],
    /// Slot EasyDMA is filling right now.
    filling: usize,
    /// Slot the captured `RESULT.PTR` points at (DMA switches to it on END).
    armed: usize,
}

impl Mic {
    /// Start free-running capture into `ring`. The buffer must live (and not
    /// be otherwise touched) for as long as the `Mic` exists.
    pub fn start(
        p0: &pac::P0,
        ppi: &pac::PPI,
        saadc: pac::SAADC,
        ring: &'static mut [[i16; CHUNK]; RING],
    ) -> Self {
        // Power the mic and give it time to settle (~100 ms). RUN_MIC is not
        // a control line: the GPIO itself supplies the mic and its LED, so it
        // must be high-drive (CODAL does the same) or the rail sags to
        // millivolts and the ADC sees only noise.
        p0.pin_cnf[20].write(|w| {
            w.dir().output();
            w.input().disconnect();
            w.drive().h0h1()
        });
        p0.outset.write(|w| unsafe { w.bits(1 << 20) });
        cortex_m::asm::delay(6_400_000);

        // AIN3 (P0.05) single-ended. The mic delivers only millivolts (bias
        // ~80 mV, speech a few mV), so measure 0..150 mV: internal 0.6 V
        // reference with gain 4 (37 uV/LSB) — the same operating point CODAL
        // uses (`setGain(7)` = GAIN4). A wider range would bury speech in
        // quantization noise.
        saadc.ch[0].pselp.write(|w| w.pselp().analog_input3());
        saadc.ch[0].pseln.write(|w| w.pseln().nc());
        saadc.ch[0].config.write(|w| {
            w.refsel().internal();
            w.gain().gain4();
            w.tacq()._10us();
            w.mode().se();
            w.resp().bypass();
            w.resn().bypass();
            w.burst().disabled()
        });
        saadc.resolution.write(|w| w.val()._12bit());
        // Local timer mode: 16 MHz / 1000 = 16 kHz; MODE=Timers is bit 12.
        saadc
            .samplerate
            .write(|w| unsafe { w.bits(1000 | (1 << 12)) });
        saadc.enable.write(|w| w.enable().enabled());

        // PPI: END -> START. START re-captures RESULT.PTR, so DMA hops to the
        // armed slot the moment a chunk completes, with zero sample loss.
        ppi.ch[0]
            .eep
            .write(|w| unsafe { w.bits(saadc.events_end.as_ptr() as u32) });
        ppi.ch[0]
            .tep
            .write(|w| unsafe { w.bits(saadc.tasks_start.as_ptr() as u32) });
        ppi.chenset.write(|w| unsafe { w.bits(1) });

        let ring = ring.as_mut_ptr();
        // Arm slot 0, start, then pre-arm slot 1 and kick the sample timer.
        saadc.result.ptr.write(|w| unsafe { w.bits(ring as u32) });
        saadc
            .result
            .maxcnt
            .write(|w| unsafe { w.bits(CHUNK as u32) });
        saadc.events_started.write(|w| unsafe { w.bits(0) });
        saadc.tasks_start.write(|w| unsafe { w.bits(1) });
        while saadc.events_started.read().bits() == 0 {}
        saadc.events_started.write(|w| unsafe { w.bits(0) });
        // SAFETY: slot 1 is in bounds of the ring.
        saadc
            .result
            .ptr
            .write(|w| unsafe { w.bits(ring.add(1) as u32) });
        saadc.tasks_sample.write(|w| unsafe { w.bits(1) });

        Self {
            saadc,
            ring,
            filling: 0,
            armed: 1,
        }
    }

    /// Block until the chunk currently filling completes; returns it and
    /// re-arms the slot DMA just vacated. The returned slice is safe to read
    /// until the *next* `wait_chunk` call (DMA is filling the other slot).
    pub fn wait_chunk(&mut self) -> &[i16] {
        let s = &self.saadc;
        while s.events_end.read().bits() == 0 {}
        s.events_end.write(|w| unsafe { w.bits(0) });
        cortex_m::asm::dmb(); // EasyDMA wrote the chunk behind the compiler

        let completed = self.filling;
        self.filling = self.armed;
        self.armed = (self.armed + 1) % RING;
        // SAFETY: armed is in bounds; DMA is not using this register's value
        // until the next END.
        s.result
            .ptr
            .write(|w| unsafe { w.bits(self.ring.wrapping_add(self.armed) as u32) });
        // SAFETY: DMA finished this slot and is now filling the other one.
        unsafe { core::slice::from_raw_parts(self.ring.add(completed) as *const i16, CHUNK) }
    }
}
