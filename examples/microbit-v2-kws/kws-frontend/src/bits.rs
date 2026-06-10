//! Bit helpers mirroring TFLM's bits.h.

#[inline]
pub fn most_significant_bit_32(n: u32) -> i32 {
    32 - n.leading_zeros() as i32
}

#[inline]
pub fn most_significant_bit_64(n: u64) -> i32 {
    64 - n.leading_zeros() as i32
}
