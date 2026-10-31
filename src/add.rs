//! Additive arithmetic: `Add`, `checked_sub`, `abs_diff` and [`log1mexp`].

use core::ops::Add;

use crate::float::LogFloat;
use crate::log::Log;

/// ln(1 − e^(−d)) for d ≥ 0, computed accurately for every d.
///
/// This is the kernel of [`Log::checked_sub`]: a − b = a · (1 − b/a), and
/// ln(1 − b/a) = `log1mexp(la − lb)`.
///
/// The naive `ln(1 - exp(-d))` loses all precision for small d, where `exp(-d)` rounds to
/// one, and the alternative `ln_1p(-exp(-d))` loses it for small d as well. Following
/// Mächler (2012), the function switches between two formulas at d = ln 2:
///
/// - d ≤ ln 2: `ln(−expm1(−d))`, since `expm1` is accurate near 0;
/// - d > ln 2: `ln_1p(−exp(−d))`, since `exp(−d) ≤ 1/2` and `ln_1p` is accurate there.
///
/// Edge cases: `log1mexp(0) = −∞` and `log1mexp(∞) = 0`. Error: at most ε·max(1, |result|)
/// for `f64` and 1.5·ε·max(1, |result|) for `f32` (see `docs/accuracy.md`).
///
/// Reference: Martin Mächler, *Accurately Computing log(1 − exp(−|a|)), Assessed by the Rmpfr
/// package* (2012), <https://cran.r-project.org/package=Rmpfr/vignettes/log1mexp-note.pdf>.
///
/// # Panics
///
/// If `d` is negative or NaN (for d < 0 the logarithm's argument is negative).
///
/// ```
/// use mirifici::log1mexp;
/// // 1 - e^-1e-20 = 1e-20 (to 20 digits), so the result is ln(1e-20).
/// assert!((log1mexp(1e-20_f64) - 1e-20_f64.ln()).abs() < 1e-14);
/// assert_eq!(log1mexp(0.0_f64), f64::NEG_INFINITY);
/// ```
#[inline]
#[track_caller]
#[must_use]
pub fn log1mexp<F: LogFloat>(d: F) -> F {
    assert!(d >= F::ZERO, "log1mexp: argument must be >= 0, got {d:?}");
    log1mexp_unchecked(d)
}

/// [`log1mexp`] without the domain check. Requires `d >= 0`.
#[inline]
pub(crate) fn log1mexp_unchecked<F: LogFloat>(d: F) -> F {
    if d <= F::LN_2 {
        (-(-d).exp_m1()).ln()
    } else {
        (-(-d).exp()).ln_1p()
    }
}

/// ln(eᵃ + eᵇ) on stored logs. Never NaN for non-NaN inputs.
#[inline]
pub(crate) fn ln_add_exp<F: LogFloat>(a: F, b: F) -> F {
    let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
    if lo == F::NEG_INFINITY || hi == F::INFINITY {
        // x + 0 = x, and ∞ + anything = ∞ (including ∞ + ∞).
        return hi;
    }
    hi + (lo - hi).exp().ln_1p()
}

impl<F: LogFloat> Add for Log<F> {
    type Output = Self;

    /// a + b, by the log-sum-exp identity
    ///
    /// ```text
    /// ln(a + b) = m + ln_1p(exp(−|la − lb|)),   m = max(la, lb)
    /// ```
    ///
    /// Factoring out the larger term means `exp` never overflows and the correction
    /// `ln_1p(…)` lies in [0, ln 2]. Edge cases: `x + ZERO = x` (exactly), and
    /// `x + INFINITY = INFINITY`, including `INFINITY + INFINITY`.
    ///
    /// The correction is computed with `ln_1p(t) = ln(1 + t)`, not with `ln(1.0 + t)`: when
    /// b is much smaller than a, t = b/a is tiny and forming `1.0 + t` rounds its low digits
    /// away (for t = 10⁻²⁰ it rounds to exactly 1, and the sum would lose b entirely).
    /// `ln_1p` evaluates the function directly, in the spirit of Nicholas Mercator's
    /// *Logarithmotechnia* (London, 1668), which gave the series
    /// ln(1 + t) = t − t²/2 + t³/3 − …; the book is also generally credited with the first
    /// use of the term “natural logarithm”.
    ///
    /// Error: at most ε·max(1, |ln(a + b)|) in the stored log, i.e. a relative error of that
    /// size in the value (measured 0.75; see `docs/accuracy.md`).
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(ln_add_exp(self.0, rhs.0))
    }
}

crate::ops::forward_binop!(Log, Add, add, AddAssign, add_assign);

impl<F: LogFloat> Log<F> {
    /// a − b, or `None` if the result would be negative or is indeterminate.
    ///
    /// For a ≥ b this computes
    ///
    /// ```text
    /// ln(a − b) = la + log1mexp(la − lb)
    /// ```
    ///
    /// with [`log1mexp`] following Mächler (2012). Edge cases:
    ///
    /// - b > a: `None` (a `Log` cannot be negative; use `SignedLog` for signed results);
    /// - a == b: `ZERO`, exactly;
    /// - b == `ZERO`: `a`, exactly;
    /// - a == `INFINITY` and b finite: `INFINITY`;
    /// - ∞ − ∞: `None` (indeterminate).
    ///
    /// Subtraction of nearly equal values is ill-conditioned for any representation: the
    /// result's relative error is the inputs' relative error amplified by a/(a − b). The
    /// computation itself adds at most 2ε·max(1, |la|, |ln(a − b)|) to the stored log
    /// (measured 0.85; see `docs/accuracy.md`).
    ///
    /// ```
    /// use mirifici::Log;
    /// let a = Log::from_value(5.0_f64);
    /// let b = Log::from_value(3.0);
    /// assert!((a.checked_sub(b).unwrap().to_value() - 2.0).abs() < 1e-15);
    /// assert_eq!(b.checked_sub(a), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (a, b) = (self.0, rhs.0);
        if b > a {
            None
        } else if b == F::NEG_INFINITY {
            Some(self)
        } else if a == F::INFINITY {
            if b == F::INFINITY { None } else { Some(self) }
        } else if a == b {
            Some(Self::ZERO)
        } else {
            // a > b, both finite. `a - b` may overflow to +inf; log1mexp(inf) = 0 is then
            // the right answer.
            Some(Self(a + log1mexp_unchecked(a - b)))
        }
    }

    /// |a − b|. Same formula and error as [`checked_sub`](Log::checked_sub), applied to the
    /// larger minus the smaller.
    ///
    /// # Panics
    ///
    /// If both are `INFINITY` (∞ − ∞ is indeterminate).
    #[inline]
    #[track_caller]
    #[must_use]
    pub fn abs_diff(self, rhs: Self) -> Self {
        let (hi, lo) = if self >= rhs {
            (self, rhs)
        } else {
            (rhs, self)
        };
        match hi.checked_sub(lo) {
            Some(v) => v,
            None => panic!("Log::abs_diff: inf - inf is indeterminate"),
        }
    }
}
