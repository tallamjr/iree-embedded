//! Integer natural-log scaling, port of TFLM log_scale.c / log_lut.c.

use crate::bits::most_significant_bit_32;

pub const SCALE_SHIFT: u32 = 6;

const LOG_SCALE_LOG2: u32 = 16;
const LOG_SEGMENTS_LOG2: u32 = 7;
const LOG_SCALE: u32 = 65536;
const LOG_COEFF: u64 = 45426;

#[rustfmt::skip]
const LOG_LUT: [u16; 130] = [
    0,    224,  442,  654,  861,  1063, 1259, 1450, 1636, 1817, 1992, 2163,
    2329, 2490, 2646, 2797, 2944, 3087, 3224, 3358, 3487, 3611, 3732, 3848,
    3960, 4068, 4172, 4272, 4368, 4460, 4549, 4633, 4714, 4791, 4864, 4934,
    5001, 5063, 5123, 5178, 5231, 5280, 5326, 5368, 5408, 5444, 5477, 5507,
    5533, 5557, 5578, 5595, 5610, 5622, 5631, 5637, 5640, 5641, 5638, 5633,
    5626, 5615, 5602, 5586, 5568, 5547, 5524, 5498, 5470, 5439, 5406, 5370,
    5332, 5291, 5249, 5203, 5156, 5106, 5054, 5000, 4944, 4885, 4825, 4762,
    4697, 4630, 4561, 4490, 4416, 4341, 4264, 4184, 4103, 4020, 3935, 3848,
    3759, 3668, 3575, 3481, 3384, 3286, 3186, 3084, 2981, 2875, 2768, 2659,
    2549, 2437, 2323, 2207, 2090, 1971, 1851, 1729, 1605, 1480, 1353, 1224,
    1094, 963,  830,  695,  559,  421,  282,  142,  0,    0,
];

fn log2_fraction_part(x: u32, log2x: u32) -> u32 {
    let mut frac = (x as i64 - (1i64 << log2x)) as i32;
    if log2x < LOG_SCALE_LOG2 {
        frac <<= LOG_SCALE_LOG2 - log2x;
    } else {
        frac >>= log2x - LOG_SCALE_LOG2;
    }
    let base_seg = (frac >> (LOG_SCALE_LOG2 - LOG_SEGMENTS_LOG2)) as u32;
    let seg_unit = (1u32 << LOG_SCALE_LOG2) >> LOG_SEGMENTS_LOG2;

    let c0 = LOG_LUT[base_seg as usize] as i32;
    let c1 = LOG_LUT[base_seg as usize + 1] as i32;
    let seg_base = (seg_unit * base_seg) as i32;
    let rel_pos = ((c1 - c0) * (frac - seg_base)) >> LOG_SCALE_LOG2;
    (frac + c0 + rel_pos) as u32
}

fn log_natural(x: u32, scale_shift: u32) -> u32 {
    let integer = (most_significant_bit_32(x) - 1) as u32;
    let fraction = log2_fraction_part(x, integer);
    let log2 = (integer << LOG_SCALE_LOG2).wrapping_add(fraction);
    let round = LOG_SCALE / 2;
    let loge = ((LOG_COEFF * log2 as u64 + round as u64) >> LOG_SCALE_LOG2) as u32;
    ((loge << scale_shift).wrapping_add(round)) >> LOG_SCALE_LOG2
}

/// LogScaleApply for one value; `correction_bits` is non-negative here.
pub fn scale_value(v: u32, correction_bits: u32) -> u16 {
    let mut value = v << correction_bits;
    if value > 1 {
        value = log_natural(value, SCALE_SHIFT);
    } else {
        value = 0;
    }
    value.min(u16::MAX as u32) as u16
}
