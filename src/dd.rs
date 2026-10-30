//! Double-double helpers for converting between ln(x) and decimal scientific notation.
//!
//! Formatting needs log10(x) = ln(x)·log10(e) to more than 53 bits: the integer part is the
//! decimal exponent and the fraction determines the mantissa, so for |log10 x| ≈ 5000 a
//! plain product would leave only ~40 bits for the mantissa. Carrying the product as an
//! unevaluated sum `hi + lo` keeps the full precision of the stored log.

use core::f64::consts::{
    LN_2 as LN_2_HI, LN_10 as LN_10_HI, LOG2_E as LOG2_E_HI, LOG10_E as LOG10_E_HI,
};

// The high parts are the standard library's correctly rounded constants; the low parts
// were computed with mpmath at 50 digits.

/// log10(e) = hi + lo to ~106 bits.
pub(crate) const LOG10_E: (f64, f64) = (LOG10_E_HI, 1.098319650216765e-17);
/// ln(10) = hi + lo to ~106 bits.
pub(crate) const LN_10: (f64, f64) = (LN_10_HI, -2.1707562233822494e-16);
/// log2(e) = hi + lo to ~106 bits.
pub(crate) const LOG2_E: (f64, f64) = (LOG2_E_HI, 2.0355273740931033e-17);
/// ln(2) = hi + lo to ~106 bits.
pub(crate) const LN_2: (f64, f64) = (LN_2_HI, 2.3190468138462996e-17);

#[inline]
fn fma(a: f64, b: f64, c: f64) -> f64 {
    #[cfg(feature = "std")]
    return a.mul_add(b, c);
    #[cfg(not(feature = "std"))]
    return libm::fma(a, b, c);
}

#[inline]
pub(crate) fn exp(x: f64) -> f64 {
    #[cfg(feature = "std")]
    return x.exp();
    #[cfg(not(feature = "std"))]
    return libm::exp(x);
}

#[inline]
pub(crate) fn ln(x: f64) -> f64 {
    #[cfg(feature = "std")]
    return x.ln();
    #[cfg(not(feature = "std"))]
    return libm::log(x);
}

#[inline]
pub(crate) fn floor(x: f64) -> f64 {
    #[cfg(feature = "std")]
    return x.floor();
    #[cfg(not(feature = "std"))]
    return libm::floor(x);
}

/// `a · (hi + lo)` rounded to the nearest f64 (up to a tiny fraction of an ulp).
#[inline]
pub(crate) fn mul_rounded(a: f64, c: (f64, f64)) -> f64 {
    mul(a, c).0
}

/// `a · (hi + lo)` as a normalised double-double.
#[inline]
fn mul(a: f64, (hi, lo): (f64, f64)) -> (f64, f64) {
    let p = a * hi;
    if !p.is_finite() {
        return (p, 0.0);
    }
    let e = fma(a, hi, -p) + a * lo;
    let s = p + e;
    (s, e - (s - p))
}

/// Splits a finite ln(x) into a decimal exponent `k` (an integer-valued float) and a
/// mantissa `m ∈ [1, 10)` with x ≈ m · 10^k.
pub(crate) fn to_decimal(ln: f64) -> (f64, f64) {
    let (th, tl) = mul(ln, LOG10_E);
    if th.abs() >= 4503599627370496.0 {
        // |log10 x| ≥ 2^52: the stored log has no fractional precision left, so the
        // mantissa is meaningless; report a power of ten.
        return (1.0, th);
    }
    let mut k = floor(th);
    // th − k is exact (it is the fractional part of th).
    let mut f = (th - k) + tl;
    if f < 0.0 {
        f += 1.0;
        k -= 1.0;
    } else if f >= 1.0 {
        f -= 1.0;
        k += 1.0;
    }
    let (ph, pl) = mul(f, LN_10);
    let e = exp(ph);
    let m = e + e * pl;
    // Rounding can land a hair outside [1, 10).
    if m >= 10.0 {
        (1.0, k + 1.0)
    } else {
        (m.max(1.0), k)
    }
}

/// ln(m · 10^k) for a mantissa `m > 0` and an integer-valued exponent `k`.
pub(crate) fn from_decimal(m: f64, k: f64) -> f64 {
    let (kh, kl) = mul(k, LN_10);
    if !kh.is_finite() {
        return kh;
    }
    let lm = ln(m);
    // Two-sum of kh and lm, then fold in the low parts.
    let s = kh + lm;
    let bb = s - kh;
    let err = (kh - (s - bb)) + (lm - bb);
    s + (err + kl)
}
