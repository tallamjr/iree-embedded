//! Fixed-point (int16) real FFT, a faithful port of kissfft with
//! `FIXED_POINT == 16` as vendored by TFLite-Micro, restricted to the one
//! shape this front end uses: a 512-point real FFT computed as a 256-point
//! complex FFT (factors 4,4,4,4) plus the real post-processing pass.

use libm::{cos, floor, sin};

/// Real FFT length (the 480-sample window zero-padded to the next power of 2).
pub const FFT_SIZE: usize = 512;
/// Complex sub-FFT length.
const NCFFT: usize = FFT_SIZE / 2;

const SAMP_MAX: i32 = i16::MAX as i32;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Cpx {
    pub r: i16,
    pub i: i16,
}

/// `sround(smul(a, b))`: Q15 multiply with round-half-up, truncated to i16
/// exactly as the C does (wrapping, no saturation).
#[inline]
fn s_mul(a: i16, b: i16) -> i16 {
    (((a as i32).wrapping_mul(b as i32).wrapping_add(1 << 14)) >> 15) as i16
}

/// `C_FIXDIV(c, div)`: scale by SAMP_MAX/div in Q15 (note: integer division,
/// so /2 multiplies by 16383/32768, not exactly half).
#[inline]
fn fixdiv(c: Cpx, div: i32) -> Cpx {
    let k = (SAMP_MAX / div) as i16;
    Cpx {
        r: s_mul(c.r, k),
        i: s_mul(c.i, k),
    }
}

#[inline]
fn c_mul(a: Cpx, b: Cpx) -> Cpx {
    let rr = (a.r as i32).wrapping_mul(b.r as i32);
    let ii = (a.i as i32).wrapping_mul(b.i as i32);
    let ri = (a.r as i32).wrapping_mul(b.i as i32);
    let ir = (a.i as i32).wrapping_mul(b.r as i32);
    Cpx {
        r: ((rr.wrapping_sub(ii).wrapping_add(1 << 14)) >> 15) as i16,
        i: ((ri.wrapping_add(ir).wrapping_add(1 << 14)) >> 15) as i16,
    }
}

#[inline]
fn c_add(a: Cpx, b: Cpx) -> Cpx {
    Cpx {
        r: a.r.wrapping_add(b.r),
        i: a.i.wrapping_add(b.i),
    }
}

#[inline]
fn c_sub(a: Cpx, b: Cpx) -> Cpx {
    Cpx {
        r: a.r.wrapping_sub(b.r),
        i: a.i.wrapping_sub(b.i),
    }
}

/// `kf_cexp` for FIXED_POINT: `floor(0.5 + 32767 * cos/sin(phase))`, computed
/// in f64 like the C (math.h `cos`/`sin` on doubles).
fn cexp(phase: f64) -> Cpx {
    Cpx {
        r: floor(0.5 + SAMP_MAX as f64 * cos(phase)) as i16,
        i: floor(0.5 + SAMP_MAX as f64 * sin(phase)) as i16,
    }
}

pub struct Fft {
    twiddles: [Cpx; NCFFT],
    super_twiddles: [Cpx; NCFFT / 2],
    tmpbuf: [Cpx; NCFFT],
}

impl Fft {
    pub const fn new() -> Self {
        Self {
            twiddles: [Cpx { r: 0, i: 0 }; NCFFT],
            super_twiddles: [Cpx { r: 0, i: 0 }; NCFFT / 2],
            tmpbuf: [Cpx { r: 0, i: 0 }; NCFFT],
        }
    }

    pub fn init(&mut self) {
        // The C uses a 60-digit literal; it rounds to the same f64 as this
        // constant, so the twiddle factors are bit-identical.
        const PI: f64 = core::f64::consts::PI;
        for (i, t) in self.twiddles.iter_mut().enumerate() {
            *t = cexp(-2.0 * PI * i as f64 / NCFFT as f64);
        }
        for (i, t) in self.super_twiddles.iter_mut().enumerate() {
            // kiss_fftr uses a slightly shorter PI literal here; the doubles
            // are identical at f64 precision.
            *t = cexp(-PI * ((i + 1) as f64 / NCFFT as f64 + 0.5));
        }
    }

    /// `kf_bfly2`.
    fn bfly2(fout: &mut [Cpx], twiddles: &[Cpx; NCFFT], fstride: usize, m: usize) {
        for k in 0..m {
            let a = fixdiv(fout[k], 2);
            let b = fixdiv(fout[k + m], 2);
            let t = c_mul(b, twiddles[k * fstride]);
            fout[k + m] = c_sub(a, t);
            fout[k] = c_add(a, t);
        }
    }

    /// `kf_bfly4`, forward direction only.
    fn bfly4(fout: &mut [Cpx], twiddles: &[Cpx; NCFFT], fstride: usize, m: usize) {
        let m2 = 2 * m;
        let m3 = 3 * m;
        for k in 0..m {
            let f0 = fixdiv(fout[k], 4);
            let f1 = fixdiv(fout[k + m], 4);
            let f2 = fixdiv(fout[k + m2], 4);
            let f3 = fixdiv(fout[k + m3], 4);

            let s0 = c_mul(f1, twiddles[k * fstride]);
            let s1 = c_mul(f2, twiddles[k * fstride * 2]);
            let s2 = c_mul(f3, twiddles[k * fstride * 3]);

            let s5 = c_sub(f0, s1);
            let f0 = c_add(f0, s1);
            let s3 = c_add(s0, s2);
            let s4 = c_sub(s0, s2);

            fout[k + m2] = c_sub(f0, s3);
            fout[k] = c_add(f0, s3);
            fout[k + m] = Cpx {
                r: s5.r.wrapping_add(s4.i),
                i: s5.i.wrapping_sub(s4.r),
            };
            fout[k + m3] = Cpx {
                r: s5.r.wrapping_sub(s4.i),
                i: s5.i.wrapping_add(s4.r),
            };
        }
    }

    /// `kf_work` for nfft = 256 = 4 * 4 * 4 * 4 (factor list fixed at init in
    /// the C; constant here). `fin` is the strided input view.
    fn work(
        twiddles: &[Cpx; NCFFT],
        fout: &mut [Cpx],
        fin: &[Cpx],
        fin_offset: usize,
        fstride: usize,
        factors: &[(usize, usize)],
    ) {
        let (p, m) = factors[0];
        if m == 1 {
            for (i, slot) in fout.iter_mut().enumerate().take(p * m) {
                *slot = fin[fin_offset + i * fstride];
            }
        } else {
            for q in 0..p {
                Self::work(
                    twiddles,
                    &mut fout[q * m..(q + 1) * m],
                    fin,
                    fin_offset + fstride * q,
                    fstride * p,
                    &factors[1..],
                );
            }
        }
        match p {
            2 => Self::bfly2(fout, twiddles, fstride, m),
            4 => Self::bfly4(fout, twiddles, fstride, m),
            _ => unreachable!("256-point FFT only factors into 4s and 2s"),
        }
    }

    /// `kiss_fftr`: forward real FFT of `input` (FFT_SIZE i16 samples) into
    /// `FFT_SIZE / 2 + 1` complex bins.
    pub fn real_fft(&mut self, input: &[i16; FFT_SIZE], output: &mut [Cpx; NCFFT + 1]) {
        // The C reinterprets the i16 buffer as NCFFT complex pairs.
        let mut fin = [Cpx { r: 0, i: 0 }; NCFFT];
        for (i, c) in fin.iter_mut().enumerate() {
            c.r = input[2 * i];
            c.i = input[2 * i + 1];
        }

        // kf_factor(256) yields (4,64) (4,16) (4,4) (4,1).
        const FACTORS: [(usize, usize); 4] = [(4, 64), (4, 16), (4, 4), (4, 1)];
        Self::work(&self.twiddles, &mut self.tmpbuf, &fin, 0, 1, &FACTORS);

        let tdc = fixdiv(self.tmpbuf[0], 2);
        output[0] = Cpx {
            r: tdc.r.wrapping_add(tdc.i),
            i: 0,
        };
        output[NCFFT] = Cpx {
            r: tdc.r.wrapping_sub(tdc.i),
            i: 0,
        };

        for k in 1..=NCFFT / 2 {
            let fpk = fixdiv(self.tmpbuf[k], 2);
            let fpnk = fixdiv(
                Cpx {
                    r: self.tmpbuf[NCFFT - k].r,
                    i: self.tmpbuf[NCFFT - k].i.wrapping_neg(),
                },
                2,
            );
            let f1k = c_add(fpk, fpnk);
            let f2k = c_sub(fpk, fpnk);
            let tw = c_mul(f2k, self.super_twiddles[k - 1]);

            // HALF_OF on int-promoted sums: the C adds the i16s in 32 bits
            // before shifting, so the sum must not wrap at 16 bits here.
            output[k] = Cpx {
                r: ((f1k.r as i32 + tw.r as i32) >> 1) as i16,
                i: ((f1k.i as i32 + tw.i as i32) >> 1) as i16,
            };
            output[NCFFT - k] = Cpx {
                r: ((f1k.r as i32 - tw.r as i32) >> 1) as i16,
                i: ((tw.i as i32 - f1k.i as i32) >> 1) as i16,
            };
        }
    }
}
