//! The [`Log`] type: construction, constants, accessors and ordering.

use core::cmp::Ordering;
use core::fmt;

use crate::error::DomainError;
use crate::float::LogFloat;

/// A non-negative real number x, stored as its natural logarithm ln(x).
///
/// `Log<F>` is `#[repr(transparent)]` over `F`, so it has the size and alignment of the float
/// it wraps. The stored float is the *only* state: `Log::ZERO` stores −∞, `Log::ONE` stores
/// 0, and `Log::INFINITY` stores +∞.
///
/// Operators act on the represented value x, not on the stored logarithm:
///
/// | expression | value | computed on the stored logs |
/// |---|---|---|
/// | `a * b` | a·b | `la + lb` |
/// | `a / b` | a/b | `la − lb` |
/// | `a + b` | a+b | `max + ln_1p(exp(−\|la − lb\|))` |
/// | `a.checked_sub(b)` | a−b | `la + log1mexp(la − lb)` |
/// | `a.powf(p)` | aᵖ | `p · la` |
///
/// There is deliberately no `Sub` operator, because a − b leaves the domain when b > a. Use
/// [`checked_sub`](Log::checked_sub), [`abs_diff`](Log::abs_diff), or
/// `SignedLog` (milestone 4).
///
/// # NaN policy
///
/// A `Log` never stores NaN, and no operation on valid inputs produces one:
///
/// - constructing from a negative value or NaN is an error ([`DomainError`]);
/// - indeterminate forms (0·∞, 0/0, ∞/∞, ∞−∞) return `None` from the `checked_*` methods
///   and panic in the operators.
///
/// [`from_ln_unchecked`](Log::from_ln_unchecked) skips the NaN check; if you pass it NaN, the
/// results of later operations are unspecified (but never undefined behaviour).
///
/// # Accuracy
///
/// Every error bound in this crate is stated as **absolute error in the stored logarithm**.
/// If the stored log is off by δ, the value is off by a factor e^δ ≈ 1 + δ, so an absolute
/// error of δ in ln(x) *is* a relative error of δ in x. The representation itself carries
/// such an error: rounding ln(x) to the nearest float costs up to half an ulp of ln(x), i.e.
/// a relative error of about ε·|ln x|/2 in x (ε is the machine epsilon of `F`). Values near 1
/// are therefore stored with full relative precision, while 1e-300 (|ln x| ≈ 690) carries a
/// relative error of up to ~8·10⁻¹⁴ in `f64`. This is the price of range; see
/// `docs/accuracy.md` for the measured error of every operation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Log<F: LogFloat = f64>(pub(crate) F);

impl<F: LogFloat> Log<F> {
    /// Zero. Stores ln(0) = −∞.
    pub const ZERO: Self = Self(F::NEG_INFINITY);
    /// One. Stores ln(1) = 0.
    pub const ONE: Self = Self(F::ZERO);
    /// Positive infinity. Stores +∞.
    pub const INFINITY: Self = Self(F::INFINITY);

    /// Creates a `Log` from a value x ≥ 0, storing ln(x).
    ///
    /// `0.0` and `-0.0` give [`Log::ZERO`]; `+∞` gives [`Log::INFINITY`].
    ///
    /// # Panics
    ///
    /// If `x` is negative or NaN. Use [`try_from_value`](Log::try_from_value) to handle
    /// these as errors.
    ///
    /// ```
    /// use mirifici::Log;
    /// let half = Log::from_value(0.5);
    /// assert_eq!(half.ln(), 0.5f64.ln());
    /// ```
    #[inline]
    #[track_caller]
    pub fn from_value(x: F) -> Self {
        match Self::try_from_value(x) {
            Ok(v) => v,
            Err(e) => panic!("Log::from_value({x:?}): {e}"),
        }
    }

    /// Creates a `Log` from a value x ≥ 0, or returns why it cannot.
    ///
    /// # Errors
    ///
    /// [`DomainError::Negative`] if `x < 0`, [`DomainError::NaN`] if `x` is NaN.
    ///
    /// ```
    /// use mirifici::{DomainError, Log};
    /// assert_eq!(Log::try_from_value(-1.0), Err(DomainError::Negative));
    /// assert_eq!(Log::try_from_value(0.0), Ok(Log::ZERO));
    /// ```
    #[inline]
    pub fn try_from_value(x: F) -> Result<Self, DomainError> {
        if x >= F::ZERO {
            Ok(Self(x.ln()))
        } else if x.is_nan() {
            Err(DomainError::NaN)
        } else {
            Err(DomainError::Negative)
        }
    }

    /// Creates a `Log` from its natural logarithm. Any non-NaN `ln` is valid: −∞ is zero and
    /// +∞ is infinity. `-0.0` is stored as `0.0`.
    ///
    /// # Panics
    ///
    /// If `ln` is NaN.
    #[inline]
    #[track_caller]
    pub fn from_ln(ln: F) -> Self {
        match Self::try_from_ln(ln) {
            Ok(v) => v,
            Err(e) => panic!("Log::from_ln({ln:?}): {e}"),
        }
    }

    /// Creates a `Log` from its natural logarithm, or returns [`DomainError::NaN`].
    ///
    /// # Errors
    ///
    /// [`DomainError::NaN`] if `ln` is NaN.
    #[inline]
    pub fn try_from_ln(ln: F) -> Result<Self, DomainError> {
        if ln.is_nan() {
            Err(DomainError::NaN)
        } else {
            // Adding +0 turns -0 into +0 and leaves everything else unchanged.
            Ok(Self(ln + F::ZERO))
        }
    }

    /// Creates a `Log` from its natural logarithm without checking for NaN.
    ///
    /// This is for hot paths where the logarithm is known to be valid, for example values
    /// read back from a buffer of stored logs. If `ln` is NaN, the invariant that a `Log`
    /// never holds NaN is broken, and the results of later operations on it (comparisons,
    /// arithmetic, formatting) are unspecified. This can never cause undefined behaviour.
    #[inline]
    pub const fn from_ln_unchecked(ln: F) -> Self {
        Self(ln)
    }

    /// Creates a `Log` from its base-2 logarithm: stores `log2 · ln 2`.
    ///
    /// The multiplication adds at most half an ulp of rounding to the stored log.
    ///
    /// # Panics
    ///
    /// If `log2` is NaN.
    #[inline]
    #[track_caller]
    pub fn from_log2(log2: F) -> Self {
        Self::from_ln(log2 * F::LN_2)
    }

    /// Creates a `Log` from its base-10 logarithm: stores `log10 · ln 10`.
    ///
    /// The multiplication adds at most half an ulp of rounding to the stored log.
    ///
    /// # Panics
    ///
    /// If `log10` is NaN.
    ///
    /// ```
    /// use mirifici::Log;
    /// let tiny = Log::from_log10(-5000.0); // 1e-5000, far below f64's range
    /// assert!(tiny > Log::ZERO);
    /// assert_eq!(tiny.to_value(), 0.0);
    /// ```
    #[inline]
    #[track_caller]
    pub fn from_log10(log10: F) -> Self {
        Self::from_ln(log10 * F::LN_10)
    }

    /// The stored natural logarithm, ln(x). Exact.
    #[inline]
    pub fn ln(self) -> F {
        self.0
    }

    /// The base-2 logarithm, `ln · log2(e)`. Adds at most half an ulp of rounding.
    #[inline]
    pub fn log2(self) -> F {
        self.0 * F::LOG2_E
    }

    /// The base-10 logarithm, `ln · log10(e)`. Adds at most half an ulp of rounding.
    #[inline]
    pub fn log10(self) -> F {
        self.0 * F::LOG10_E
    }

    /// The logarithm in an arbitrary base, `ln / ln(base)`.
    ///
    /// # Panics
    ///
    /// Unless `base` is finite, positive and not 1.
    #[inline]
    #[track_caller]
    pub fn log(self, base: F) -> F {
        assert!(
            base > F::ZERO && base.is_finite() && base != F::ONE,
            "Log::log: base must be finite, positive and not 1, got {base:?}"
        );
        self.0 / base.ln()
    }

    /// The represented value as a float, `exp(ln)`.
    ///
    /// This **loses information** outside the float's range: values too small for `F`
    /// become 0 and values too large become ∞ (and subnormal results keep only a few
    /// significant bits). Use [`try_to_value`](Log::try_to_value) to detect this, or
    /// `Display` to print the value without converting it.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::from_value(0.25).to_value(), 0.25);
    /// assert_eq!(Log::from_log10(-400.0).to_value(), 0.0); // saturates
    /// ```
    #[inline]
    pub fn to_value(self) -> F {
        self.0.exp()
    }

    /// The represented value as a float, or `None` if it would round to 0 or ∞.
    ///
    /// [`Log::ZERO`] and [`Log::INFINITY`] convert exactly to `0.0` and `∞`. A non-zero,
    /// finite value that rounds to `0.0` (underflow) or `∞` (overflow) gives `None`. Values in
    /// the subnormal range are returned, with reduced precision.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::<f64>::ZERO.try_to_value(), Some(0.0));
    /// assert_eq!(Log::from_log10(-400.0).try_to_value(), None);
    /// assert_eq!(Log::from_log10(400.0).try_to_value(), None);
    /// ```
    #[inline]
    pub fn try_to_value(self) -> Option<F> {
        let v = self.0.exp();
        let underflow = v == F::ZERO && self.0 != F::NEG_INFINITY;
        let overflow = v == F::INFINITY && self.0 != F::INFINITY;
        if underflow || overflow { None } else { Some(v) }
    }

    /// A total order consistent with `PartialOrd`.
    ///
    /// Because a `Log` never holds NaN, `partial_cmp` always succeeds; `total_cmp` returns
    /// the same answer without the `Option`, which is convenient for
    /// `slice.sort_by(Log::total_cmp)` and `iter.max_by(Log::total_cmp)`. Unlike
    /// [`f64::total_cmp`], it treats the stored `-0.0` and `0.0` as equal, so it agrees with
    /// `==`.
    #[inline]
    pub fn total_cmp(&self, other: &Self) -> Ordering {
        match self.0.partial_cmp(&other.0) {
            Some(ord) => ord,
            // Only reachable through `from_ln_unchecked(NaN)`.
            None => self.0.total_cmp(&other.0),
        }
    }

    /// The larger of two values.
    #[inline]
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        if other > self { other } else { self }
    }

    /// The smaller of two values.
    #[inline]
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        if other < self { other } else { self }
    }
}

impl<F: LogFloat> Default for Log<F> {
    /// [`Log::ZERO`], the additive identity.
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: LogFloat> fmt::Debug for Log<F> {
    /// Shows the stored logarithm, e.g. `Log(ln = -11512.925464970229)`. Format flags such
    /// as precision apply to the logarithm.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Log(ln = ")?;
        fmt::Debug::fmt(&self.0, f)?;
        f.write_str(")")
    }
}
