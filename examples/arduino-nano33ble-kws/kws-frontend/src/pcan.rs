//! Per-channel amplitude normalisation, port of TFLM pcan_gain_control.c and
//! its LUT construction (https://research.google/pubs/pub45911.pdf).

use crate::bits::most_significant_bit_32;
use crate::filterbank::NUM_CHANNELS;
use libm::powf;

const SNR_BITS: i32 = 12;
const OUTPUT_BITS: i32 = 6;
const WIDE_DYNAMIC_FUNCTION_BITS: i32 = 32;
const LUT_SIZE: usize = (4 * WIDE_DYNAMIC_FUNCTION_BITS - 3) as usize;

const STRENGTH: f32 = 0.95;
const OFFSET: f32 = 80.0;
const GAIN_BITS: i32 = 21;
const I16_MAX_F: f32 = 0x7FFF as f32;

pub struct Pcan {
    gain_lut: [i16; LUT_SIZE],
    snr_shift: i32,
}

impl Pcan {
    pub const fn new() -> Self {
        Self {
            gain_lut: [0; LUT_SIZE],
            snr_shift: 0,
        }
    }

    fn lookup(input_bits: i32, x: u32) -> i16 {
        let x_as_float = x as f32 / (1u32 << input_bits) as f32;
        let gain_as_float = (1u32 << GAIN_BITS) as f32 * powf(x_as_float + OFFSET, -STRENGTH);
        if gain_as_float > I16_MAX_F {
            return 0x7FFF;
        }
        (gain_as_float + 0.5) as i16
    }

    pub fn init(&mut self, smoothing_bits: u32, input_correction_bits: i32) {
        self.snr_shift = GAIN_BITS - input_correction_bits - SNR_BITS;
        let input_bits = smoothing_bits as i32 - input_correction_bits;

        self.gain_lut[0] = Self::lookup(input_bits, 0);
        self.gain_lut[1] = Self::lookup(input_bits, 1);
        // The C shifts the lut pointer by -6 and writes lut[4 * interval ..];
        // the equivalent flat index is 4 * interval - 6.
        for interval in 2..=WIDE_DYNAMIC_FUNCTION_BITS {
            let x0: u32 = 1 << (interval - 1);
            let x1: u32 = x0 + (x0 >> 1);
            let x2: u32 = if interval == WIDE_DYNAMIC_FUNCTION_BITS {
                x0 + (x0 - 1)
            } else {
                2 * x0
            };

            let y0 = Self::lookup(input_bits, x0) as i32;
            let y1 = Self::lookup(input_bits, x1) as i32;
            let y2 = Self::lookup(input_bits, x2) as i32;

            let diff1 = y1 - y0;
            let diff2 = y2 - y0;
            let a1 = 4 * diff1 - diff2;
            let a2 = diff2 - a1;

            let base = (4 * interval - 6) as usize;
            self.gain_lut[base] = y0 as i16;
            self.gain_lut[base + 1] = a1 as i16;
            self.gain_lut[base + 2] = a2 as i16;
        }
    }

    fn wide_dynamic_function(&self, x: u32) -> i16 {
        if x <= 2 {
            return self.gain_lut[x as usize];
        }
        let interval = most_significant_bit_32(x);
        let base = (4 * interval - 6) as usize;

        let frac = (if interval < 11 {
            x << (11 - interval)
        } else {
            x >> (interval - 11)
        }) & 0x3FF;

        let mut result = ((self.gain_lut[base + 2] as i32) * frac as i32) >> 5;
        result += (self.gain_lut[base + 1] as i32) << 5;
        result = result.wrapping_mul(frac as i32);
        result = (result + (1 << 14)) >> 15;
        result += self.gain_lut[base] as i32;
        result as i16
    }

    fn shrink(x: u32) -> u32 {
        if x < (2 << SNR_BITS) {
            (x * x) >> (2 + 2 * SNR_BITS - OUTPUT_BITS)
        } else {
            (x >> (SNR_BITS - OUTPUT_BITS)) - (1 << OUTPUT_BITS)
        }
    }

    pub fn apply(&self, noise_estimate: &[u32; NUM_CHANNELS], signal: &mut [u32; NUM_CHANNELS]) {
        for (s, n) in signal.iter_mut().zip(noise_estimate.iter()) {
            let gain = self.wide_dynamic_function(*n) as u32;
            let snr = (((*s as u64) * (gain as u64)) >> self.snr_shift) as u32;
            *s = Self::shrink(snr);
        }
    }
}
