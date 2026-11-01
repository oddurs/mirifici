//! Probability helpers: complements and the logistic function, without leaving log space.

use crate::add::log1mexp_unchecked;
use crate::float::LogFloat;
use crate::log::Log;

/// softplus(y) = ln(1 + eʸ), accurate for every y (never overflows, never rounds a small
/// eʸ away).
fn softplus<F: LogFloat>(y: F) -> F {
    if y > F::ZERO {
        y + (-y).exp().ln_1p()
    } else {
        y.exp().ln_1p()
    }
}

impl<F: LogFloat> Log<F> {
    /// 1 − p, for a probability p ≤ 1; `None` if p > 1.
    ///
    /// This is `Log::ONE.checked_sub(p)`, named for what it is in probability code: the
    /// chance that something does *not* happen. It is accurate at both ends: for p near 0,
    /// 1 − p is computed as ln(1 − p) = log1mexp(−ln p) rather than by forming 1 − p, and for
    /// p near 1 the tiny result keeps its full relative precision.
    ///
    /// ```
    /// use mirifici::Log;
    /// let survive = Log::from_value(0.999_999_f64);
    /// let fail = survive.complement().unwrap();
    /// assert!((fail.to_value() / 1e-6 - 1.0).abs() < 1e-9);
    /// // a thousand independent survivals, then their complement
    /// let all = survive.powi(1000);
    /// assert!((all.complement().unwrap().to_value() - 9.995e-4).abs() < 1e-6);
    /// assert_eq!(Log::from_value(1.5_f64).complement(), None);
    /// ```
    #[must_use]
    pub fn complement(self) -> Option<Self> {
        Self::ONE.checked_sub(self)
    }

    /// σ(x) = 1/(1 + e⁻ˣ), the logistic function, as a `Log`.
    ///
    /// Computed as ln σ(x) = −softplus(−x), so it never overflows and keeps full precision
    /// in both tails: σ(−800) is about 3.67·10⁻³⁴⁸, not 0, and σ(40) is 1 − 4.2·10⁻¹⁸ with that
    /// difference intact in the stored log. `σ(−∞) = 0`, `σ(∞) = 1`.
    ///
    /// # Panics
    ///
    /// If `x` is NaN.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::from_logit(0.0_f64).to_value(), 0.5);
    /// assert_eq!(Log::from_logit(-800.0_f64).to_string(), "3.6678745841777e-348");
    /// ```
    #[must_use]
    #[track_caller]
    pub fn from_logit(x: F) -> Self {
        assert!(!x.is_nan(), "Log::from_logit: argument is NaN");
        Self::from_ln(F::ZERO - softplus(F::ZERO - x))
    }

    /// logit(p) = ln(p / (1 − p)), the inverse of [`from_logit`](Log::from_logit), for a
    /// probability p ≤ 1; `None` if p > 1.
    ///
    /// Computed as ln p − log1mexp(−ln p), so probabilities near 1 (whose complement is tiny)
    /// keep their precision. `logit(0) = −∞`, `logit(1) = ∞`.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::from_value(0.5_f64).logit(), Some(0.0));
    /// let p = Log::from_logit(37.5_f64);
    /// assert!((p.logit().unwrap() - 37.5).abs() < 1e-12);
    /// assert_eq!(Log::from_value(2.0_f64).logit(), None);
    /// ```
    #[must_use]
    pub fn logit(self) -> Option<F> {
        let l = self.0;
        if l > F::ZERO {
            return None;
        }
        if l == F::NEG_INFINITY {
            return Some(l);
        }
        // −ln p ≥ 0; ln(1 − p) = log1mexp(−ln p), which is −∞ for p = 1.
        Some(l - log1mexp_unchecked(F::ZERO - l))
    }
}
