//! Multiplicative arithmetic: `Mul`, `Div`, powers.

use core::ops::{Div, Mul};

use crate::float::LogFloat;
use crate::log::Log;

impl<F: LogFloat> Log<F> {
    /// a · b, or `None` if the product is indeterminate.
    ///
    /// Computes `la + lb`, which is correctly rounded: the stored log of the result is off by
    /// at most half an ulp. The only indeterminate form is 0 · ∞ (`−∞ + ∞`), which returns
    /// `None`. A product of finite values whose log overflows becomes [`Log::INFINITY`] (or
    /// [`Log::ZERO`] if it underflows); this happens only beyond e^(±1.8·10³⁰⁸) for `f64`.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::<f64>::ZERO.checked_mul(Log::INFINITY), None);
    /// assert_eq!(Log::ZERO.checked_mul(Log::from_value(5.0)), Some(Log::ZERO));
    /// ```
    #[inline]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let ln = self.0 + rhs.0;
        if ln.is_nan() { None } else { Some(Self(ln)) }
    }

    /// a / b, or `None` if the quotient is indeterminate.
    ///
    /// Computes `la − lb`, correctly rounded. Division by zero gives [`Log::INFINITY`] for a
    /// non-zero numerator. The indeterminate forms 0/0 and ∞/∞ return `None`.
    ///
    /// ```
    /// use mirifici::Log;
    /// let two = Log::from_value(2.0);
    /// assert_eq!(two.checked_div(Log::ZERO), Some(Log::INFINITY));
    /// assert_eq!(Log::<f64>::ZERO.checked_div(Log::ZERO), None);
    /// assert_eq!(Log::<f64>::INFINITY.checked_div(Log::INFINITY), None);
    /// ```
    #[inline]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        let ln = self.0 - rhs.0;
        if ln.is_nan() { None } else { Some(Self(ln)) }
    }

    /// xᵖ, computed as `p · ln`.
    ///
    /// The result is correctly rounded (half an ulp in the stored log). Edge cases follow the
    /// IEEE 754 `pow` function:
    ///
    /// - x⁰ = 1 for every x, including 0⁰ = 1 and ∞⁰ = 1;
    /// - 1ᵖ = 1 for every p, including ±∞;
    /// - 0ᵖ = ∞ and ∞ᵖ = 0 for p < 0.
    ///
    /// # Panics
    ///
    /// If `p` is NaN.
    #[inline]
    #[track_caller]
    #[must_use]
    pub fn powf(self, p: F) -> Self {
        assert!(!p.is_nan(), "Log::powf: exponent is NaN");
        if p == F::ZERO || self.0 == F::ZERO {
            Self::ONE
        } else {
            Self(p * self.0)
        }
    }

    /// xⁿ, computed as `n · ln`. Same conventions as [`powf`](Log::powf), including 0⁰ = 1.
    ///
    /// For `f32`, `n` is first rounded to `f32`, which is exact for |n| ≤ 2²⁴.
    ///
    /// Integer powers are where logarithms began for Jost Bürgi, the Swiss clockmaker who
    /// built them independently of Napier. His *Arithmetische und Geometrische
    /// Progress-Tabulen* (Prague, 1620) set an arithmetic progression of “red” numbers
    /// beside the geometric progression of “black” numbers 10⁸·1.0001ⁿ, so that a black
    /// number's power could be read off by multiplying its red number. `powi(n)` is one step
    /// along such a table: the stored log is multiplied by n.
    ///
    /// ```
    /// use mirifici::Log;
    /// // The last entry of Bürgi's table: 1.0001^23027 ≈ 10 (his black number 10^9).
    /// let x = Log::from_value(1.0001_f64).powi(23027);
    /// assert!((x.to_value() - 9.9999).abs() < 1e-3);
    /// ```
    #[inline]
    #[must_use]
    pub fn powi(self, n: i32) -> Self {
        if n == 0 || self.0 == F::ZERO {
            Self::ONE
        } else {
            Self(F::from_i32(n) * self.0)
        }
    }

    /// √x, computed as `ln / 2`. Exact (halving a float does not round, except in the
    /// subnormal range of the stored log, i.e. for x within ~10⁻³⁰⁸ of 1).
    #[inline]
    #[must_use]
    pub fn sqrt(self) -> Self {
        Self(self.0 * F::HALF)
    }

    /// 1/x, computed as `−ln`. Exact. `ZERO.recip()` is `INFINITY` and vice versa.
    #[inline]
    #[must_use]
    pub fn recip(self) -> Self {
        // `0 - ln` rather than `-ln` so that ONE.recip() stores +0, not -0.
        Self(F::ZERO - self.0)
    }
}

impl<F: LogFloat> Mul for Log<F> {
    type Output = Self;

    /// a · b. See [`Log::checked_mul`] for the formula and edge cases.
    ///
    /// # Panics
    ///
    /// On 0 · ∞, which is indeterminate.
    #[inline]
    #[track_caller]
    fn mul(self, rhs: Self) -> Self {
        match self.checked_mul(rhs) {
            Some(v) => v,
            None => panic!("Log multiplication: 0 * inf is indeterminate"),
        }
    }
}

impl<F: LogFloat> Div for Log<F> {
    type Output = Self;

    /// a / b. See [`Log::checked_div`] for the formula and edge cases.
    ///
    /// # Panics
    ///
    /// On 0/0 and ∞/∞, which are indeterminate.
    #[inline]
    #[track_caller]
    fn div(self, rhs: Self) -> Self {
        match self.checked_div(rhs) {
            Some(v) => v,
            None => panic!("Log division: 0/0 and inf/inf are indeterminate"),
        }
    }
}

/// Implements the `&a op b`, `a op &b`, `&a op &b` and `a op= b`, `a op= &b` forms of a binary
/// operator in terms of the by-value form.
macro_rules! forward_binop {
    ($ty:ident, $Op:ident, $op:ident, $OpAssign:ident, $op_assign:ident) => {
        impl<F: LogFloat> core::ops::$Op<&$ty<F>> for $ty<F> {
            type Output = $ty<F>;
            #[inline]
            #[track_caller]
            fn $op(self, rhs: &$ty<F>) -> $ty<F> {
                core::ops::$Op::$op(self, *rhs)
            }
        }
        impl<F: LogFloat> core::ops::$Op<$ty<F>> for &$ty<F> {
            type Output = $ty<F>;
            #[inline]
            #[track_caller]
            fn $op(self, rhs: $ty<F>) -> $ty<F> {
                core::ops::$Op::$op(*self, rhs)
            }
        }
        impl<F: LogFloat> core::ops::$Op<&$ty<F>> for &$ty<F> {
            type Output = $ty<F>;
            #[inline]
            #[track_caller]
            fn $op(self, rhs: &$ty<F>) -> $ty<F> {
                core::ops::$Op::$op(*self, *rhs)
            }
        }
        impl<F: LogFloat> core::ops::$OpAssign for $ty<F> {
            #[inline]
            #[track_caller]
            fn $op_assign(&mut self, rhs: $ty<F>) {
                *self = core::ops::$Op::$op(*self, rhs);
            }
        }
        impl<F: LogFloat> core::ops::$OpAssign<&$ty<F>> for $ty<F> {
            #[inline]
            #[track_caller]
            fn $op_assign(&mut self, rhs: &$ty<F>) {
                *self = core::ops::$Op::$op(*self, *rhs);
            }
        }
    };
}
pub(crate) use forward_binop;

forward_binop!(Log, Mul, mul, MulAssign, mul_assign);
forward_binop!(Log, Div, div, DivAssign, div_assign);
