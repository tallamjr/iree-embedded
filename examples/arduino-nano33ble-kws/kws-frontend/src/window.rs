//! Hann-style analysis window, port of TFLM window.c / window_util.c.

use libm::{cosf, floorf};

pub const WINDOW_BITS: i32 = 12;
/// 30 ms at 16 kHz.
pub const WINDOW_SIZE: usize = 480;
/// 20 ms step at 16 kHz.
pub const WINDOW_STEP: usize = 320;

pub struct Window {
    coefficients: [i16; WINDOW_SIZE],
    input: [i16; WINDOW_SIZE],
    input_used: usize,
    pub output: [i16; WINDOW_SIZE],
    pub max_abs_output_value: i16,
}

impl Window {
    pub const fn new() -> Self {
        Self {
            coefficients: [0; WINDOW_SIZE],
            input: [0; WINDOW_SIZE],
            input_used: 0,
            output: [0; WINDOW_SIZE],
            max_abs_output_value: 0,
        }
    }

    pub fn init(&mut self) {
        // float arithmetic exactly as WindowPopulateState.
        let arg = core::f32::consts::PI * 2.0 / (WINDOW_SIZE as f32);
        for (i, c) in self.coefficients.iter_mut().enumerate() {
            let float_value = 0.5 - 0.5 * cosf(arg * (i as f32 + 0.5));
            *c = floorf(float_value * (1 << WINDOW_BITS) as f32 + 0.5) as i16;
        }
        self.reset();
    }

    pub fn reset(&mut self) {
        self.input = [0; WINDOW_SIZE];
        self.output = [0; WINDOW_SIZE];
        self.input_used = 0;
        self.max_abs_output_value = 0;
    }

    /// Returns the samples consumed; `true` in the bool when a full window
    /// was produced into `self.output`.
    pub fn process_samples(&mut self, samples: &[i16]) -> (usize, bool) {
        let take = (WINDOW_SIZE - self.input_used).min(samples.len());
        self.input[self.input_used..self.input_used + take].copy_from_slice(&samples[..take]);
        self.input_used += take;
        if self.input_used < WINDOW_SIZE {
            return (take, false);
        }

        let mut max_abs: i16 = 0;
        for i in 0..WINDOW_SIZE {
            let new_value =
                (((self.input[i] as i32) * (self.coefficients[i] as i32)) >> WINDOW_BITS) as i16;
            self.output[i] = new_value;
            // The C negates an int16 in place; mirror its wrapping.
            let abs = if new_value < 0 {
                new_value.wrapping_neg()
            } else {
                new_value
            };
            if abs > max_abs {
                max_abs = abs;
            }
        }
        self.input.copy_within(WINDOW_STEP.., 0);
        self.input_used -= WINDOW_STEP;
        self.max_abs_output_value = max_abs;
        (take, true)
    }
}
