//! Pure-Rust port of the TFLite-Micro audio front end, fixed to the
//! micro_speech configuration (16 kHz mono, 30 ms window / 20 ms step,
//! 40 mel channels in 125..7500 Hz, noise reduction, PCAN, log scale),
//! byte-exact against the vendored C reference (see tests/golden.rs).
//!
//! `no_std`, no heap, no `unsafe`: all state lives inline in [`Frontend`]
//! (~13 KiB), so firmware can keep it in a static.

#![cfg_attr(not(test), no_std)]

mod bits;
mod fft;
mod filterbank;
mod log_scale;
mod noise_reduction;
mod pcan;
mod window;

use bits::most_significant_bit_32;
use fft::{Cpx, FFT_SIZE, Fft};
use filterbank::{FILTERBANK_BITS, Filterbank, SPECTRUM_SIZE};
use noise_reduction::{NoiseReduction, SMOOTHING_BITS};
use pcan::Pcan;
use window::{WINDOW_SIZE, Window};

pub use filterbank::NUM_CHANNELS;

/// Frames in the model's input spectrogram (1 s of audio at a 20 ms step).
pub const NUM_FRAMES: usize = 49;
/// Bytes in one full feature window.
pub const FEATURE_BYTES: usize = NUM_FRAMES * NUM_CHANNELS;

/// `MostSignificantBit32(fft_size) - 1 - kFilterbankBits / 2` for a 512-point
/// FFT: the fixed log-stage correction.
const CORRECTION_BITS: u32 = (10 - 1 - FILTERBANK_BITS / 2) as u32;

pub struct Frontend {
    window: Window,
    fft: Fft,
    fft_input: [i16; FFT_SIZE],
    fft_output: [Cpx; SPECTRUM_SIZE],
    filterbank: Filterbank,
    noise: NoiseReduction,
    pcan: Pcan,
    /// Rolling spectrogram: the most recent `spec_frames` frames, newest last.
    spec: [u8; FEATURE_BYTES],
    spec_frames: usize,
}

impl Default for Frontend {
    fn default() -> Self {
        Self::new()
    }
}

impl Frontend {
    pub const fn new() -> Self {
        Self {
            window: Window::new(),
            fft: Fft::new(),
            fft_input: [0; FFT_SIZE],
            fft_output: [Cpx { r: 0, i: 0 }; SPECTRUM_SIZE],
            filterbank: Filterbank::new(),
            noise: NoiseReduction::new(),
            pcan: Pcan::new(),
            spec: [0; FEATURE_BYTES],
            spec_frames: 0,
        }
    }

    /// Build the fixed-point tables. Call once before use (separate from
    /// `new` so the state can live in a `static`).
    pub fn init(&mut self) {
        self.window.init();
        self.fft.init();
        self.filterbank.init(16_000);
        self.noise.init();
        self.pcan.init(SMOOTHING_BITS, CORRECTION_BITS as i32);
        self.reset();
    }

    /// FrontendReset plus clearing the rolling spectrogram.
    pub fn reset(&mut self) {
        self.window.reset();
        self.fft_input = [0; FFT_SIZE];
        self.fft_output = [Cpx { r: 0, i: 0 }; SPECTRUM_SIZE];
        self.filterbank.reset();
        self.noise.reset();
        self.spec = [0; FEATURE_BYTES];
        self.spec_frames = 0;
    }

    /// FrontendProcessSamples: consume input until a full window is available
    /// and produce one frame of channel values. Returns (samples_read, frame).
    fn process(&mut self, samples: &[i16]) -> (usize, Option<[u16; NUM_CHANNELS]>) {
        let (read, ready) = self.window.process_samples(samples);
        if !ready {
            return (read, None);
        }

        let input_shift = 15 - most_significant_bit_32(self.window.max_abs_output_value as u32);
        for i in 0..WINDOW_SIZE {
            self.fft_input[i] = ((self.window.output[i] as u16) << input_shift) as i16;
        }
        self.fft_input[WINDOW_SIZE..].fill(0);
        self.fft.real_fft(&self.fft_input, &mut self.fft_output);

        self.filterbank.accumulate(&self.fft_output);
        let mut signal = [0u32; NUM_CHANNELS];
        self.filterbank.sqrt(input_shift, &mut signal);

        self.noise.apply(&mut signal);
        self.pcan.apply(&self.noise.estimate, &mut signal);

        let mut frame = [0u16; NUM_CHANNELS];
        for (f, &s) in frame.iter_mut().zip(signal.iter()) {
            *f = log_scale::scale_value(s, CORRECTION_BITS);
        }
        (read, Some(frame))
    }

    /// Append one frame to the rolling spectrogram, clamping channel values
    /// to u8 exactly as the firmware shim always has.
    fn push_frame(&mut self, frame: &[u16; NUM_CHANNELS]) {
        if self.spec_frames == NUM_FRAMES {
            self.spec.copy_within(NUM_CHANNELS.., 0);
            self.spec_frames = NUM_FRAMES - 1;
        }
        let base = self.spec_frames * NUM_CHANNELS;
        for (i, &v) in frame.iter().enumerate() {
            self.spec[base + i] = v.min(255) as u8;
        }
        self.spec_frames += 1;
    }

    /// Streaming entry point: feed live samples, applying DC removal (`mean`)
    /// and digital `gain` on the fly. Returns frames produced.
    pub fn push(&mut self, samples: &[i16], mean: i32, gain: i32) -> usize {
        let mut scratch = [0i16; WINDOW_SIZE];
        let mut produced = 0;
        let mut remaining = samples;
        while !remaining.is_empty() {
            let chunk = remaining.len().min(scratch.len());
            for (dst, &src) in scratch.iter_mut().zip(remaining[..chunk].iter()) {
                let v = ((src as i32) - mean) * gain;
                *dst = v.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            let mut consumed = 0;
            while consumed < chunk {
                let (read, frame) = self.process(&scratch[consumed..chunk]);
                consumed += read;
                if let Some(f) = frame {
                    self.push_frame(&f);
                    produced += 1;
                }
                if read == 0 {
                    break;
                }
            }
            remaining = &remaining[chunk..];
        }
        produced
    }

    /// Copy the rolling spectrogram into `out`, zero-padding the oldest
    /// frames while warming up. Returns the valid frame count.
    pub fn window(&self, out: &mut [u8; FEATURE_BYTES]) -> usize {
        let pad = (NUM_FRAMES - self.spec_frames) * NUM_CHANNELS;
        out[..pad].fill(0);
        out[pad..].copy_from_slice(&self.spec[..self.spec_frames * NUM_CHANNELS]);
        self.spec_frames
    }

    /// One-shot mode mirroring the C shim's `kws_features`: reset, process
    /// the whole clip, emit up to 49 frames. Returns frames produced.
    pub fn features_oneshot(&mut self, samples: &[i16], out: &mut [u8; FEATURE_BYTES]) -> usize {
        self.reset();
        let mut frames = 0;
        let mut i = 0;
        while i < samples.len() && frames < NUM_FRAMES {
            let (read, frame) = self.process(&samples[i..]);
            i += read;
            if let Some(f) = frame {
                for (c, &v) in f.iter().enumerate() {
                    out[frames * NUM_CHANNELS + c] = v.min(255) as u8;
                }
                frames += 1;
            }
            if read == 0 {
                break;
            }
        }
        frames
    }
}
