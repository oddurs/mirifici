//! [`SignedLog`]: a sign and a log-space magnitude.

use core::cmp::Ordering;
use core::fmt;
use core::iter::{Product, Sum};
use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::aggregate::Streaming;
use crate::error::DomainError;
use crate::float::LogFloat;
use crate::log::Log;

/// A real number of either sign, stored as a sign and a [`Log`] magnitude.
///
/// `Log` covers x ≥ 0 and so has no subtraction operator. `SignedLog` covers the whole real
/// line, with the same range: −1e-5000 and 1e+90000 are both representable. It supports
/// `+`, `-`, `*`, `/` and unary `-`. Zero is always stored with a positive sign, so there is
/// no negative zero.
///
/// Arithmetic follows `Log`: multiplication and division add and subtract magnitudes'
/// logs (correctly rounded); addition of like signs is the log-sum-exp of `Log`'s `Add`;
/// addition of unlike signs is `Log::checked_sub` of the larger magnitude minus the
/// smaller. That last case is subject to cancellation exactly as in ordinary floating point:
/// the relative error of a − b grows like |a|/|a − b|.
///
/// The indeterminate forms 0·∞, 0/0, ∞/∞ and ∞ − ∞ return `None` from the `checked_*`
/// methods and panic in the operators.
///
/// The type is larger than `F` (a magnitude plus a sign flag, padded).
///
/// ```
/// use mirifici::{Log, SignedLog};
/// let a = SignedLog::from_value(2.0_f64);
/// let b = SignedLog::from_value(5.0);
/// let d = a - b;
/// assert!(d.is_negative());
/// assert!((d.to_value() + 3.0).abs() < 1e-15);
/// // Back to Log only when non-negative:
/// assert!(Log::try_from(d).is_err());
/// assert!(Log::try_from(-d).is_ok());
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct SignedLog<F: LogFloat = f64> {
    magnitude: Log<F>,
    negative: bool,
}

impl<F: LogFloat> SignedLog<F> {
    /// Zero.
    pub const ZERO: Self = Self {
        magnitude: Log::ZERO,
        negative: false,
    };
    /// One.
    pub const ONE: Self = Self {
        magnitude: Log::ONE,
        negative: false,
    };
    /// Minus one.
    pub const NEG_ONE: Self = Self {
        magnitude: Log::ONE,
        negative: true,
    };
    /// Positive infinity.
    pub const INFINITY: Self = Self {
        magnitude: Log::INFINITY,
        negative: false,
    };
    /// Negative infinity.
    pub const NEG_INFINITY: Self = Self {
        magnitude: Log::INFINITY,
        negative: true,
    };

    /// A value with the given sign and magnitude. A zero magnitude gives [`SignedLog::ZERO`]
    /// whatever the sign.
    #[inline]
    pub fn new(negative: bool, magnitude: Log<F>) -> Self {
        Self {
            magnitude,
            negative: negative && magnitude != Log::ZERO,
        }
    }

    /// Creates a `SignedLog` from any non-NaN float. `-0.0` gives `ZERO`.
    ///
    /// # Panics
    ///
    /// If `x` is NaN.
    #[inline]
    #[track_caller]
    pub fn from_value(x: F) -> Self {
        match Self::try_from_value(x) {
            Ok(v) => v,
            Err(e) => panic!("SignedLog::from_value({x:?}): {e}"),
        }
    }

    /// Creates a `SignedLog` from a float, or returns [`DomainError::NaN`].
    ///
    /// # Errors
    ///
    /// [`DomainError::NaN`] if `x` is NaN.
    #[inline]
    pub fn try_from_value(x: F) -> Result<Self, DomainError> {
        if x.is_nan() {
            Err(DomainError::NaN)
        } else if x < F::ZERO {
            Ok(Self::new(true, Log((-x).ln())))
        } else {
            Ok(Self::new(false, Log(x.ln())))
        }
    }

    /// The magnitude |x|.
    #[inline]
    pub fn magnitude(self) -> Log<F> {
        self.magnitude
    }

    /// Whether x < 0. False for zero.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.negative
    }

    /// The value as a float, saturating to ±0 or ±∞ outside the float's range like
    /// [`Log::to_value`].
    #[inline]
    pub fn to_value(self) -> F {
        let v = self.magnitude.to_value();
        if self.negative { -v } else { v }
    }

    /// a + b, or `None` for ∞ − ∞.
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        if self.negative == rhs.negative {
            return Some(Self::new(self.negative, self.magnitude + rhs.magnitude));
        }
        let (big, small) = if self.magnitude >= rhs.magnitude {
            (self, rhs)
        } else {
            (rhs, self)
        };
        let m = big.magnitude.checked_sub(small.magnitude)?;
        Some(Self::new(big.negative, m))
    }

    /// a − b, or `None` for ∞ − ∞.
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_add(-rhs)
    }

    /// a · b, or `None` for 0 · ∞.
    #[inline]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let m = self.magnitude.checked_mul(rhs.magnitude)?;
        Some(Self::new(self.negative != rhs.negative, m))
    }

    /// a / b, or `None` for 0/0 and ∞/∞. Division of a non-zero value by zero gives an
    /// infinity whose sign is the numerator's (zero has no sign).
    #[inline]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        let m = self.magnitude.checked_div(rhs.magnitude)?;
        Some(Self::new(self.negative != rhs.negative, m))
    }

    /// A total order consistent with `PartialOrd` (which never fails, as there is no NaN).
    #[inline]
    pub fn total_cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, false) => self.magnitude.total_cmp(&other.magnitude),
            (true, true) => other.magnitude.total_cmp(&self.magnitude),
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
        }
    }
}

impl<F: LogFloat> Default for SignedLog<F> {
    /// [`SignedLog::ZERO`].
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: LogFloat> PartialOrd for SignedLog<F> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.total_cmp(other))
    }
}

impl<F: LogFloat> From<Log<F>> for SignedLog<F> {
    /// Lossless: the magnitude is copied and the sign is positive.
    #[inline]
    fn from(x: Log<F>) -> Self {
        Self {
            magnitude: x,
            negative: false,
        }
    }
}

impl<F: LogFloat> TryFrom<SignedLog<F>> for Log<F> {
    type Error = DomainError;

    /// The magnitude, if the value is not negative.
    ///
    /// # Errors
    ///
    /// [`DomainError::Negative`] if the value is negative.
    #[inline]
    fn try_from(x: SignedLog<F>) -> Result<Self, DomainError> {
        if x.negative {
            Err(DomainError::Negative)
        } else {
            Ok(x.magnitude)
        }
    }
}

impl<F: LogFloat> Neg for SignedLog<F> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(!self.negative, self.magnitude)
    }
}

impl<F: LogFloat> Neg for &SignedLog<F> {
    type Output = SignedLog<F>;
    #[inline]
    fn neg(self) -> SignedLog<F> {
        -*self
    }
}

macro_rules! panicking_op {
    ($Op:ident, $op:ident, $checked:ident, $msg:literal) => {
        impl<F: LogFloat> $Op for SignedLog<F> {
            type Output = Self;
            /// See the corresponding `checked_*` method for edge cases.
            ///
            /// # Panics
            ///
            /// On an indeterminate form.
            #[inline]
            #[track_caller]
            fn $op(self, rhs: Self) -> Self {
                match self.$checked(rhs) {
                    Some(v) => v,
                    None => panic!($msg),
                }
            }
        }
    };
}

panicking_op!(
    Add,
    add,
    checked_add,
    "SignedLog addition: inf - inf is indeterminate"
);
panicking_op!(
    Sub,
    sub,
    checked_sub,
    "SignedLog subtraction: inf - inf is indeterminate"
);
panicking_op!(
    Mul,
    mul,
    checked_mul,
    "SignedLog multiplication: 0 * inf is indeterminate"
);
panicking_op!(
    Div,
    div,
    checked_div,
    "SignedLog division: 0/0 and inf/inf are indeterminate"
);

crate::ops::forward_binop!(SignedLog, Add, add, AddAssign, add_assign);
crate::ops::forward_binop!(SignedLog, Sub, sub, SubAssign, sub_assign);
crate::ops::forward_binop!(SignedLog, Mul, mul, MulAssign, mul_assign);
crate::ops::forward_binop!(SignedLog, Div, div, DivAssign, div_assign);

#[track_caller]
fn sum_signed<F: LogFloat>(iter: impl Iterator<Item = SignedLog<F>>) -> SignedLog<F> {
    // Sum positive and negative parts separately, then subtract once: one cancellation
    // instead of one per sign change.
    let mut pos = Streaming::NEW;
    let mut neg = Streaming::NEW;
    for x in iter {
        if x.negative {
            neg.push(x.magnitude.0);
        } else {
            pos.push(x.magnitude.0);
        }
    }
    let p = SignedLog::from(Log(pos.finish()));
    let n = SignedLog::new(true, Log(neg.finish()));
    match p.checked_add(n) {
        Some(v) => v,
        None => panic!("SignedLog sum: inf - inf is indeterminate"),
    }
}

impl<F: LogFloat> Sum for SignedLog<F> {
    /// Σ xᵢ. Positive and negative terms are accumulated separately (streaming
    /// log-sum-exp) and subtracted once at the end. The empty sum is `ZERO`.
    ///
    /// # Panics
    ///
    /// If the terms include both `INFINITY` and `NEG_INFINITY`.
    #[track_caller]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        sum_signed(iter)
    }
}

impl<'a, F: LogFloat> Sum<&'a SignedLog<F>> for SignedLog<F> {
    /// Σ xᵢ; see the by-value impl.
    #[track_caller]
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        sum_signed(iter.copied())
    }
}

#[track_caller]
fn product_signed<F: LogFloat>(iter: impl Iterator<Item = SignedLog<F>>) -> SignedLog<F> {
    let mut negative = false;
    let magnitude: Log<F> = iter
        .map(|x| {
            negative ^= x.negative;
            x.magnitude
        })
        .product();
    SignedLog::new(negative, magnitude)
}

impl<F: LogFloat> Product for SignedLog<F> {
    /// Π xᵢ: the sign is the parity of the negative factors, the magnitude is the `Log`
    /// product. The empty product is `ONE`.
    ///
    /// # Panics
    ///
    /// If the factors include both a zero and an infinity.
    #[track_caller]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        product_signed(iter)
    }
}

impl<'a, F: LogFloat> Product<&'a SignedLog<F>> for SignedLog<F> {
    /// Π xᵢ; see the by-value impl.
    #[track_caller]
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        product_signed(iter.copied())
    }
}

impl<F: LogFloat> fmt::Debug for SignedLog<F> {
    /// Shows the sign and the stored log of the magnitude, e.g. `SignedLog(-, ln = 1.5)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.negative {
            "SignedLog(-, ln = "
        } else {
            "SignedLog(+, ln = "
        })?;
        fmt::Debug::fmt(&self.magnitude.0, f)?;
        f.write_str(")")
    }
}
