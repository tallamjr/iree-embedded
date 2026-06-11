//! Mel filterbank, port of TFLM filterbank.c / filterbank_util.c.

use crate::bits::{most_significant_bit_32, most_significant_bit_64};
use crate::fft::Cpx;
use libm::{floorf, log1pf};

pub const FILTERBANK_BITS: i32 = 12;
pub const NUM_CHANNELS: usize = 40;
const NUM_CHANNELS_PLUS_1: usize = NUM_CHANNELS + 1;
const INDEX_ALIGNMENT: usize = 4 / size_of::<i16>(); // kFilterbankIndexAlignment / sizeof(int16)
const CHANNEL_BLOCK_SIZE: usize = 4;
/// Spectrum bins from the 512-point real FFT.
pub const SPECTRUM_SIZE: usize = 512 / 2 + 1;
/// Upper bound on the quantized weight array length (the actual length for
/// this config is computed at init and asserted to fit).
const MAX_WEIGHTS: usize = 512;

const LOWER_BAND_LIMIT: f32 = 125.0;
const UPPER_BAND_LIMIT: f32 = 7500.0;

fn freq_to_mel(freq: f32) -> f32 {
    1127.0 * log1pf(freq / 700.0)
}

pub struct Filterbank {
    pub start_index: usize,
    pub end_index: usize,
    channel_frequency_starts: [i16; NUM_CHANNELS_PLUS_1],
    channel_weight_starts: [i16; NUM_CHANNELS_PLUS_1],
    channel_widths: [i16; NUM_CHANNELS_PLUS_1],
    weights: [i16; MAX_WEIGHTS],
    unweights: [i16; MAX_WEIGHTS],
    work: [u64; NUM_CHANNELS_PLUS_1],
    /// Energy scratch sized like the C's reuse of the FFT output allocation
    /// (padded channel reads may touch stale bins; their weights are zero).
    energy: [i32; 2 * SPECTRUM_SIZE],
}

impl Filterbank {
    pub const fn new() -> Self {
        Self {
            start_index: 0,
            end_index: 0,
            channel_frequency_starts: [0; NUM_CHANNELS_PLUS_1],
            channel_weight_starts: [0; NUM_CHANNELS_PLUS_1],
            channel_widths: [0; NUM_CHANNELS_PLUS_1],
            weights: [0; MAX_WEIGHTS],
            unweights: [0; MAX_WEIGHTS],
            work: [0; NUM_CHANNELS_PLUS_1],
            energy: [0; 2 * SPECTRUM_SIZE],
        }
    }

    pub fn init(&mut self, sample_rate: u32) {
        let mut center_mel_freqs = [0f32; NUM_CHANNELS_PLUS_1];
        let mut actual_channel_starts = [0i16; NUM_CHANNELS_PLUS_1];
        let mut actual_channel_widths = [0i16; NUM_CHANNELS_PLUS_1];

        let mel_low = freq_to_mel(LOWER_BAND_LIMIT);
        let mel_hi = freq_to_mel(UPPER_BAND_LIMIT);
        let mel_span = mel_hi - mel_low;
        let mel_spacing = mel_span / NUM_CHANNELS_PLUS_1 as f32;
        for (i, m) in center_mel_freqs.iter_mut().enumerate() {
            *m = mel_low + mel_spacing * (i + 1) as f32;
        }

        let hz_per_sbin = 0.5 * sample_rate as f32 / (SPECTRUM_SIZE as f32 - 1.0);
        self.start_index = (1.5 + LOWER_BAND_LIMIT / hz_per_sbin) as usize;
        self.end_index = 0;

        let mut chan_freq_index_start = self.start_index;
        let mut weight_index_start: usize = 0;
        let mut needs_zeros = false;

        for chan in 0..NUM_CHANNELS_PLUS_1 {
            let mut freq_index = chan_freq_index_start;
            while freq_to_mel(freq_index as f32 * hz_per_sbin) <= center_mel_freqs[chan] {
                freq_index += 1;
            }
            let width = freq_index - chan_freq_index_start;
            actual_channel_starts[chan] = chan_freq_index_start as i16;
            actual_channel_widths[chan] = width as i16;

            if width == 0 {
                self.channel_frequency_starts[chan] = 0;
                self.channel_weight_starts[chan] = 0;
                self.channel_widths[chan] = CHANNEL_BLOCK_SIZE as i16;
                if !needs_zeros {
                    needs_zeros = true;
                    for j in 0..chan {
                        self.channel_weight_starts[j] += CHANNEL_BLOCK_SIZE as i16;
                    }
                    weight_index_start += CHANNEL_BLOCK_SIZE;
                }
            } else {
                let aligned_start = (chan_freq_index_start / INDEX_ALIGNMENT) * INDEX_ALIGNMENT;
                let aligned_width = chan_freq_index_start - aligned_start + width;
                let padded_width =
                    ((aligned_width - 1) / CHANNEL_BLOCK_SIZE + 1) * CHANNEL_BLOCK_SIZE;
                self.channel_frequency_starts[chan] = aligned_start as i16;
                self.channel_weight_starts[chan] = weight_index_start as i16;
                self.channel_widths[chan] = padded_width as i16;
                weight_index_start += padded_width;
            }
            chan_freq_index_start = freq_index;
        }
        assert!(weight_index_start <= MAX_WEIGHTS);
        self.weights = [0; MAX_WEIGHTS];
        self.unweights = [0; MAX_WEIGHTS];

        for chan in 0..NUM_CHANNELS_PLUS_1 {
            let mut frequency = actual_channel_starts[chan] as usize;
            let num_frequencies = actual_channel_widths[chan] as usize;
            let frequency_offset = frequency - self.channel_frequency_starts[chan] as usize;
            let weight_start = self.channel_weight_starts[chan] as usize;
            let denom_val = if chan == 0 {
                mel_low
            } else {
                center_mel_freqs[chan - 1]
            };

            for j in 0..num_frequencies {
                let weight = (center_mel_freqs[chan] - freq_to_mel(frequency as f32 * hz_per_sbin))
                    / (center_mel_freqs[chan] - denom_val);
                let weight_index = weight_start + frequency_offset + j;
                self.weights[weight_index] =
                    floorf(weight * (1 << FILTERBANK_BITS) as f32 + 0.5) as i16;
                self.unweights[weight_index] =
                    floorf((1.0 - weight) * (1 << FILTERBANK_BITS) as f32 + 0.5) as i16;
                frequency += 1;
            }
            if frequency > self.end_index {
                self.end_index = frequency;
            }
        }
        assert!(self.end_index < SPECTRUM_SIZE);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.work = [0; NUM_CHANNELS_PLUS_1];
    }

    /// FilterbankConvertFftComplexToEnergy + AccumulateChannels.
    pub fn accumulate(&mut self, fft_output: &[Cpx; SPECTRUM_SIZE]) {
        let range = self.start_index..self.end_index;
        for (e, c) in self.energy[range.clone()]
            .iter_mut()
            .zip(fft_output[range].iter())
        {
            let real = c.r as i32;
            let imag = c.i as i32;
            *e = real
                .wrapping_mul(real)
                .wrapping_add(imag.wrapping_mul(imag));
        }

        let mut weight_accumulator: u64 = 0;
        let mut unweight_accumulator: u64 = 0;
        for chan in 0..NUM_CHANNELS_PLUS_1 {
            let mag_start = self.channel_frequency_starts[chan] as usize;
            let w_start = self.channel_weight_starts[chan] as usize;
            let width = self.channel_widths[chan] as usize;
            for j in 0..width {
                // The C converts int32 energy to uint64 (sign-extending).
                let magnitude = self.energy[mag_start + j] as i64 as u64;
                weight_accumulator = weight_accumulator.wrapping_add(
                    (self.weights[w_start + j] as i64 as u64).wrapping_mul(magnitude),
                );
                unweight_accumulator = unweight_accumulator.wrapping_add(
                    (self.unweights[w_start + j] as i64 as u64).wrapping_mul(magnitude),
                );
            }
            self.work[chan] = weight_accumulator;
            weight_accumulator = unweight_accumulator;
            unweight_accumulator = 0;
        }
    }

    /// FilterbankSqrt into `out[NUM_CHANNELS]`.
    pub fn sqrt(&self, scale_down_shift: i32, out: &mut [u32; NUM_CHANNELS]) {
        for (i, o) in out.iter_mut().enumerate() {
            *o = sqrt64(self.work[i + 1]) >> scale_down_shift;
        }
    }
}

fn sqrt32(num: u32) -> u16 {
    if num == 0 {
        return 0;
    }
    let mut num = num;
    let mut res: u32 = 0;
    let max_bit_number = (32 - most_significant_bit_32(num)) | 1;
    let mut bit: u32 = 1 << (31 - max_bit_number);
    let mut iterations = (31 - max_bit_number) / 2 + 1;
    while iterations > 0 {
        iterations -= 1;
        if num >= res + bit {
            num -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    if num > res && res != 0xFFFF {
        res += 1;
    }
    res as u16
}

fn sqrt64(num: u64) -> u32 {
    if (num >> 32) == 0 {
        return sqrt32(num as u32) as u32;
    }
    let mut num = num;
    let mut res: u64 = 0;
    let max_bit_number = (64 - most_significant_bit_64(num)) | 1;
    let mut bit: u64 = 1 << (63 - max_bit_number);
    let mut iterations = (63 - max_bit_number) / 2 + 1;
    while iterations > 0 {
        iterations -= 1;
        if num >= res + bit {
            num -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    if num > res && res != 0xFFFFFFFF {
        res += 1;
    }
    res as u32
}
