//! `Float512` / `Complex512` — 512-bit fixed-precision wrappers replacing
//! `rug::Float`/`rug::Complex` from `sys1own/shbt-precision`.
//!
//! Two's-complement Q256x256 fixed point over eight u64 limbs: 256 integer
//! bits (with sign) + 256 fractional bits (~77 decimal digits), well above
//! the ~35-digit accuracy the 512-bit audit checks require.  All arithmetic
//! is deterministic, branch-bounded, and allocation-free.
#![allow(clippy::inconsistent_digit_grouping)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::suspicious_arithmetic_impl)]

use core::cmp::Ordering;
use core::ops::{Add, Neg, Sub};

/// Fractional bits of the Q256x256 layout.
pub const FRAC_BITS: u32 = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Float512 {
    /// Two's-complement limbs, little-endian; bit 255 of limb 3 is 2^0.
    limbs: [u64; 8],
}

impl Float512 {
    pub const ZERO: Self = Self { limbs: [0; 8] };
    pub const ONE: Self = Self::from_u64(1);

    pub const fn from_limbs(limbs: [u64; 8]) -> Self {
        Self { limbs }
    }
    pub const fn limbs(&self) -> &[u64; 8] {
        &self.limbs
    }

    #[inline]
    pub const fn from_u64(v: u64) -> Self {
        let mut l = [0u64; 8];
        l[4] = v; // 2^256 limb index = 4 (64*4 = 256)
        Self { limbs: l }
    }

    #[inline]
    pub const fn from_i64(v: i64) -> Self {
        if v >= 0 {
            Self::from_u64(v as u64)
        } else {
            Self::from_u64((-v) as u64).neg()
        }
    }

    /// Exact rational `num/den` (both fit u64) in Q256x256.
    pub fn from_ratio(num: u64, den: u64) -> Self {
        assert!(den != 0);
        // Integer part into limbs 4.., then long division: rem * 2 / den
        // emits fraction bits of (num mod den)/den from the MSB down, landing
        // at fixed-point bit index 255-i.
        let ipart = num / den;
        let mut rem: u128 = (num % den) as u128;
        let mut out = Self::from_u64(ipart);
        for i in 0..256u32 {
            rem <<= 1;
            if rem >= den as u128 {
                rem -= den as u128;
                let bit = 255 - i;
                out.limbs[(bit / 64) as usize] |= 1u64 << (bit % 64);
            }
        }
        out
    }

    /// Best binary approximation of an `f64` (exact for dyadics).
    pub fn from_f64(v: f64) -> Self {
        if !v.is_finite() || v == 0.0 {
            return Self::ZERO;
        }
        let neg = v < 0.0;
        let a = v.abs();
        // a = m * 2^e, m in [2^52, 2^53)
        let e = a.abs().log2().floor() as i64;
        let m = (a / 2f64.powi(e as i32) * 2f64.powi(52)) as u64;
        // Place m << (e - 52 - (-256)) → m scaled to Q256: value = m * 2^(e-52),
        // fixed-point stores value * 2^256 → limbs value = m * 2^(e - 52 + 256).
        let shift = e + 204; // e - 52 + 256
        let mut l = [0u64; 8];
        if shift >= 0 {
            let bit = shift as u32;
            let li = (bit / 64) as usize;
            let sh = bit % 64;
            l[li] |= m << sh;
            if sh > 0 && li + 1 < 8 {
                l[li + 1] |= m >> (64 - sh);
            }
        } else {
            let rshift = (-shift) as u32;
            if rshift < 64 {
                let li = 0;
                l[li] = m >> rshift;
            }
        }
        let out = Self { limbs: l };
        if neg {
            -out
        } else {
            out
        }
    }

    pub fn to_f64(self) -> f64 {
        let neg = self.limbs[7] >> 63 == 1;
        let mag = if neg { self.neg() } else { self };
        // Find most-significant limb.
        let mut top = 7i32;
        while top >= 0 && mag.limbs[top as usize] == 0 {
            top -= 1;
        }
        if top < 0 {
            return 0.0;
        }
        let li = top as usize;
        let lz = mag.limbs[li].leading_zeros() as i32;
        let msb = 64 * li as i32 + 63 - lz; // bit index of MSB
        // Collect top 53 bits for the mantissa.
        let mut mant: u64 = 0;
        for k in 0..53 {
            let bit = msb - k;
            if bit < 0 {
                break;
            }
            let b = (mag.limbs[(bit / 64) as usize] >> (bit % 64)) & 1;
            mant |= b << (52 - k);
        }
        let val = (mant as f64) * 2f64.powi(msb - 52 - 256);
        if neg {
            -val
        } else {
            val
        }
    }

    #[inline]
    fn add_raw(a: [u64; 8], b: [u64; 8]) -> [u64; 8] {
        let mut out = [0u64; 8];
        let mut carry = 0u128;
        for i in 0..8 {
            let s = a[i] as u128 + b[i] as u128 + carry;
            out[i] = s as u64;
            carry = s >> 64;
        }
        out
    }

    #[inline]
    pub const fn neg(self) -> Self {
        let mut l = [!self.limbs[0]; 8];
        let mut i = 1;
        while i < 8 {
            l[i] = !self.limbs[i];
            i += 1;
        }
        // +1 with carry
        let mut carry = 1u64;
        let mut j = 0;
        while j < 8 && carry != 0 {
            let (s, c) = l[j].overflowing_add(carry);
            l[j] = s;
            carry = c as u64;
            j += 1;
        }
        Self { limbs: l }
    }

    /// Schoolbook product `(a * b) >> 256`, retaining the middle 512 bits.
    pub fn mul(self, rhs: Self) -> Self {
        let neg = (self.limbs[7] >> 63) ^ (rhs.limbs[7] >> 63) == 1;
        let a = if self.is_neg() { self.neg().limbs } else { self.limbs };
        let b = if rhs.is_neg() { rhs.neg().limbs } else { rhs.limbs };
        let mut wide = [0u64; 16];
        for i in 0..8 {
            let mut carry = 0u128;
            for j in 0..8 {
                let s = wide[i + j] as u128 + a[i] as u128 * b[j] as u128 + carry;
                wide[i + j] = s as u64;
                carry = s >> 64;
            }
            wide[i + 8] = wide[i + 8].wrapping_add(carry as u64);
        }
        // >> 256: take limbs 4..12 of the 16-limb product.
        let mut out = [0u64; 8];
        out.copy_from_slice(&wide[4..12]);
        let r = Self { limbs: out };
        if neg {
            -r
        } else {
            r
        }
    }

    /// General division `a / b` computed as `(a << 256) / b` over 768 bits.
    pub fn div(self, rhs: Self) -> Self {
        assert!(rhs != Self::ZERO);
        let neg = (self.limbs[7] >> 63) ^ (rhs.limbs[7] >> 63) == 1;
        let a = if self.is_neg() { self.neg() } else { self };
        let d = if rhs.is_neg() { rhs.neg() } else { rhs };

        // Dividend: a.limbs << 256 → 12-limb array.
        let mut num = [0u64; 12];
        num[4..12].copy_from_slice(&a.limbs);

        // Long division producing 512 quotient bits.
        let mut rem = [0u64; 9];
        let mut quot = [0u64; 8];
        for i in (0..768u32).rev() {
            // rem <<= 1
            let mut carry = 0u64;
            for r in rem.iter_mut() {
                let nc = *r >> 63;
                *r = (*r << 1) | carry;
                carry = nc;
            }
            // bring down dividend bit i
            rem[0] |= (num[(i / 64) as usize] >> (i % 64)) & 1;
            // if rem >= d (8-limb), subtract, set quotient bit i
            if cmp9(&rem, &d.limbs) != Ordering::Less {
                sub9(&mut rem, &d.limbs);
                quot[(i / 64) as usize] |= 1u64 << (i % 64);
            }
        }
        let r = Self { limbs: quot };
        if neg {
            -r
        } else {
            r
        }
    }

    #[inline]
    pub fn is_neg(&self) -> bool {
        self.limbs[7] >> 63 == 1
    }

    #[inline]
    pub fn abs(self) -> Self {
        if self.is_neg() {
            -self
        } else {
            self
        }
    }
}

fn cmp9(a: &[u64; 9], b: &[u64; 8]) -> Ordering {
    if a[8] != 0 {
        return Ordering::Greater;
    }
    for i in (0..8).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    Ordering::Equal
}

fn sub9(a: &mut [u64; 9], b: &[u64; 8]) {
    let mut borrow = false;
    for i in 0..8 {
        let (d1, o1) = a[i].overflowing_sub(b[i]);
        let (d2, o2) = d1.overflowing_sub(borrow as u64);
        a[i] = d2;
        borrow = o1 | o2;
    }
    a[8] = a[8].wrapping_sub(borrow as u64);
}

impl Add for Float512 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            limbs: Self::add_raw(self.limbs, rhs.limbs),
        }
    }
}
impl Sub for Float512 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + rhs.neg()
    }
}
impl Neg for Float512 {
    type Output = Self;
    fn neg(self) -> Self {
        self.neg()
    }
}
impl PartialOrd for Float512 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp_total(other))
    }
}
impl Float512 {
    pub fn cmp_total(&self, other: &Self) -> Ordering {
        let s = self.is_neg();
        let o = other.is_neg();
        if s != o {
            return if s { Ordering::Less } else { Ordering::Greater };
        }
        for i in (0..8).rev() {
            match self.limbs[i].cmp(&other.limbs[i]) {
                Ordering::Equal => continue,
                ord => return if s { ord.reverse() } else { ord },
            }
        }
        Ordering::Equal
    }
}

/// Complex number over `Float512` — the `rug::Complex` analogue.
#[derive(Clone, Copy, Debug, Default)]
pub struct Complex512 {
    pub re: Float512,
    pub im: Float512,
}

impl Complex512 {
    pub const fn new(re: Float512, im: Float512) -> Self {
        Self { re, im }
    }
    /// |z|^2 as a `Float512`.
    pub fn norm2(&self) -> Float512 {
        self.re.mul(self.re) + self.im.mul(self.im)
    }
    pub fn mul(&self, rhs: &Self) -> Self {
        Self {
            re: self.re.mul(rhs.re) - self.im.mul(rhs.im),
            im: self.re.mul(rhs.im) + self.im.mul(rhs.re),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_dark_ledger() {
        let eta = Float512::from_ratio(23, 33);
        assert!((eta.to_f64() - 23.0 / 33.0).abs() < 1e-15, "{:?}", eta.to_f64());
    }

    #[test]
    fn mul_roundtrip() {
        let a = Float512::from_ratio(10, 33);
        let sq = a.mul(a);
        assert!((sq.to_f64() - 100.0 / 1089.0).abs() < 1e-15);
    }

    #[test]
    fn from_f64_roundtrip() {
        for v in [0.5, 8750.0, 1.0e-12, 352.48, 0.0918] {
            let f = Float512::from_f64(v);
            assert!((f.to_f64() - v).abs() / v.abs().max(1.0) < 1e-12, "{v}");
        }
    }

    #[test]
    fn div_and_ordering() {
        let a = Float512::from_u64(7);
        let b = Float512::from_u64(2);
        assert!((a.div(b).to_f64() - 3.5).abs() < 1e-15);
        assert!(b > Float512::ONE);
        assert!(Float512::from_i64(-3) < Float512::ZERO);
    }
}
