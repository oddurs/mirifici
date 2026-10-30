//! Sums and products of many values: `Sum`, `Product`, `sum_two_pass`,
//! `product_compensated`.

use core::iter::{Product, Sum};

use crate::float::LogFloat;
use crate::log::Log;

/// One-pass log-sum-exp accumulator over stored logs.
///
/// Keeps a running maximum `m` and `s = Σ exp(lᵢ − m)` over every term except one occurrence
/// of the maximum, so the result is `m + ln_1p(s)`. Leaving the maximum out of `s` keeps the
/// full precision of small corrections. When a new maximum arrives, `s` is rescaled by
/// `exp(m_old − m_new)` and the old maximum joins it.
#[derive(Clone, Copy)]
pub(crate) struct Streaming<F> {
    m: F,
    s: F,
}

impl<F: LogFloat> Streaming<F> {
    pub(crate) const NEW: Self = Self {
        m: F::NEG_INFINITY,
        s: F::ZERO,
    };

    #[inline]
    pub(crate) fn push(&mut self, l: F) {
        if l <= self.m {
            // Skips −∞ terms, and keeps +∞ absorbing (once m = +∞ nothing changes it).
            if l != F::NEG_INFINITY && self.m != F::INFINITY {
                self.s += (l - self.m).exp();
            }
        } else if l == F::INFINITY {
            self.m = l;
        } else {
            // When m is −∞ (first finite term) exp(−∞) = 0 resets s. A NaN `l` lands here
            // too and propagates through `s` and `m`.
            self.s = (self.s + F::ONE) * (self.m - l).exp();
            self.m = l;
        }
    }

    #[inline]
    pub(crate) fn finish(self) -> F {
        if self.m == F::NEG_INFINITY || self.m == F::INFINITY {
            self.m
        } else {
            self.m + self.s.ln_1p()
        }
    }
}

#[inline]
pub(crate) fn sum_streaming<F: LogFloat>(iter: impl Iterator<Item = F>) -> F {
    let mut acc = Streaming::NEW;
    for l in iter {
        acc.push(l);
    }
    acc.finish()
}

/// Two-pass log-sum-exp: find the maximum, then sum the other terms relative to it.
#[inline]
#[allow(clippy::neg_cmp_op_on_partial_ord)] // deliberate: also true for NaN
pub(crate) fn sum_two_pass_by<T, F: LogFloat>(xs: &[T], ln: impl Fn(&T) -> F) -> F {
    let mut m = F::NEG_INFINITY;
    let mut at = 0;
    for (i, x) in xs.iter().enumerate() {
        let l = ln(x);
        // `!(l <= m)` is "l > m or l is NaN"; the NaN test stays off the common path.
        if !(l <= m) {
            if l.is_nan() {
                return l;
            }
            m = l;
            at = i;
        }
    }
    if m == F::NEG_INFINITY || m == F::INFINITY {
        return m;
    }
    let mut s = F::ZERO;
    for x in &xs[..at] {
        s += (ln(x) - m).exp();
    }
    for x in &xs[at + 1..] {
        s += (ln(x) - m).exp();
    }
    m + s.ln_1p()
}

impl<F: LogFloat> Log<F> {
    /// Σ xᵢ over a slice, in two passes: find the maximum stored log `m`, then compute
    /// `m + ln_1p(Σ_{i≠max} exp(lᵢ − m))`.
    ///
    /// Prefer this over the streaming [`Sum`] impl when the values are already in a slice:
    /// it never rescales the accumulator (the streaming sum multiplies it by
    /// `exp(m_old − m_new)` every time a new maximum appears, which costs one rounding each
    /// time and is worst for increasing sequences), and its inner loop has no branches, so it
    /// is faster. Prefer the streaming `Sum` for iterators that are not materialised or are
    /// read only once.
    ///
    /// The empty sum is `ZERO`. If any term is `INFINITY`, the sum is `INFINITY`.
    ///
    /// Error: the accumulation has the usual floating-point bound of about n·ε relative
    /// error in `s`, which is an absolute error of at most n·ε in the stored log (typically
    /// √n·ε), plus half an ulp for the final addition. Measured: at most
    /// 1.07·ε·max(1, |result|) for n ≤ 1000 (`docs/accuracy.md`).
    pub fn sum_two_pass(xs: &[Self]) -> Self {
        Self(sum_two_pass_by(xs, |x| x.0))
    }

    /// Π xᵢ with compensated (Neumaier) summation of the stored logs.
    ///
    /// The plain [`Product`] impl adds the logs with ordinary floating-point addition, whose
    /// error grows with the number of terms (up to n·ε·Σ|lᵢ| in the worst case). This method
    /// carries a running compensation term (Neumaier's improvement of Kahan summation) so
    /// the error stays at about one ulp of the result independent of n, at roughly 4× the
    /// cost per term. Use it for products of millions of terms, such as long-sequence
    /// likelihoods, where the plain sum can lose several digits.
    ///
    /// Edge cases are those of [`Mul`](core::ops::Mul): the empty product is `ONE`, any
    /// `ZERO` factor gives `ZERO`, any `INFINITY` factor gives `INFINITY`.
    ///
    /// Reference: A. Neumaier, *Rundungsfehleranalyse einiger Verfahren zur Summation
    /// endlicher Summen*, ZAMM 54 (1974).
    ///
    /// # Panics
    ///
    /// If the factors include both `ZERO` and `INFINITY` (0 · ∞ is indeterminate).
    #[track_caller]
    pub fn product_compensated<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        let mut sum = F::ZERO;
        let mut c = F::ZERO;
        let mut has_zero = false;
        let mut has_inf = false;
        for x in iter {
            let l = x.0;
            if l == F::NEG_INFINITY {
                has_zero = true;
            } else if l == F::INFINITY {
                has_inf = true;
            } else {
                let t = sum + l;
                if sum.abs() >= l.abs() {
                    c += (sum - t) + l;
                } else {
                    c += (l - t) + sum;
                }
                sum = t;
            }
        }
        // The running sum itself can overflow (only beyond ±1.8e308 for f64).
        has_zero |= sum == F::NEG_INFINITY;
        has_inf |= sum == F::INFINITY;
        match (has_zero, has_inf) {
            (true, true) => panic!("Log::product_compensated: 0 * inf is indeterminate"),
            (true, false) => Self::ZERO,
            (false, true) => Self::INFINITY,
            (false, false) => Self(sum + c),
        }
    }
}

impl<F: LogFloat> Sum for Log<F> {
    /// Σ xᵢ in a single streaming pass (see [`Log::sum_two_pass`] for when to prefer the
    /// two-pass version). The empty sum is `ZERO`.
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(sum_streaming(iter.map(|x| x.0)))
    }
}

impl<'a, F: LogFloat> Sum<&'a Log<F>> for Log<F> {
    /// Σ xᵢ in a single streaming pass. The empty sum is `ZERO`.
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(sum_streaming(iter.map(|x| x.0)))
    }
}

/// Plain sum of stored logs; NaN only from −∞ + ∞, i.e. 0 · ∞.
#[track_caller]
fn product_plain<F: LogFloat>(iter: impl Iterator<Item = F>) -> Log<F> {
    let mut s = F::ZERO;
    for l in iter {
        s += l;
    }
    if s.is_nan() {
        panic!("Log product: 0 * inf is indeterminate");
    }
    Log(s)
}

impl<F: LogFloat> Product for Log<F> {
    /// Π xᵢ as a plain sum of the stored logs. The empty product is `ONE`.
    ///
    /// For very long products, [`Log::product_compensated`] is more accurate.
    ///
    /// # Panics
    ///
    /// If the factors include both `ZERO` and `INFINITY`.
    #[track_caller]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        product_plain(iter.map(|x| x.0))
    }
}

impl<'a, F: LogFloat> Product<&'a Log<F>> for Log<F> {
    /// Π xᵢ as a plain sum of the stored logs. The empty product is `ONE`.
    ///
    /// # Panics
    ///
    /// If the factors include both `ZERO` and `INFINITY`.
    #[track_caller]
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        product_plain(iter.map(|x| x.0))
    }
}
