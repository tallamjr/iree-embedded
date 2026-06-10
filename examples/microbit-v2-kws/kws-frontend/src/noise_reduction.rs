//! Per-channel stationary-noise estimate and subtraction, port of TFLM
//! noise_reduction.c / noise_reduction_util.c.

use crate::filterbank::NUM_CHANNELS;

pub const NOISE_REDUCTION_BITS: i32 = 14;
pub const SMOOTHING_BITS: u32 = 10;

const EVEN_SMOOTHING: f32 = 0.025;
const ODD_SMOOTHING: f32 = 0.06;
const MIN_SIGNAL_REMAINING: f32 = 0.05;

pub struct NoiseReduction {
    even_smoothing: u16,
    odd_smoothing: u16,
    min_signal_remaining: u16,
    pub estimate: [u32; NUM_CHANNELS],
}

impl NoiseReduction {
    pub const fn new() -> Self {
        Self {
            even_smoothing: 0,
            odd_smoothing: 0,
            min_signal_remaining: 0,
            estimate: [0; NUM_CHANNELS],
        }
    }

    pub fn init(&mut self) {
        let scale = (1 << NOISE_REDUCTION_BITS) as f32;
        self.even_smoothing = (EVEN_SMOOTHING * scale) as u16;
        self.odd_smoothing = (ODD_SMOOTHING * scale) as u16;
        self.min_signal_remaining = (MIN_SIGNAL_REMAINING * scale) as u16;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.estimate = [0; NUM_CHANNELS];
    }

    pub fn apply(&mut self, signal: &mut [u32; NUM_CHANNELS]) {
        for (i, s) in signal.iter_mut().enumerate() {
            let smoothing = if i & 1 == 0 {
                self.even_smoothing as u64
            } else {
                self.odd_smoothing as u64
            } as u32;
            let one_minus_smoothing = (1u32 << NOISE_REDUCTION_BITS) - smoothing;

            let signal_scaled_up = *s << SMOOTHING_BITS;
            let mut estimate = (((signal_scaled_up as u64) * (smoothing as u64)
                + (self.estimate[i] as u64) * (one_minus_smoothing as u64))
                >> NOISE_REDUCTION_BITS) as u32;
            self.estimate[i] = estimate;

            if estimate > signal_scaled_up {
                estimate = signal_scaled_up;
            }

            let floor =
                (((*s as u64) * (self.min_signal_remaining as u64)) >> NOISE_REDUCTION_BITS) as u32;
            let subtracted = (signal_scaled_up - estimate) >> SMOOTHING_BITS;
            *s = subtracted.max(floor);
        }
    }
}
