//! `num-traits`: only the traits whose semantics `Log` genuinely has.
//!
//! Not implemented, because they would lie: `Num` (requires `Sub`, `Rem` and radix
//! parsing), `Float`/`Real` (require negative values and NaN), `Signed`, `Unsigned`
//! (`Log` is not an integer type), `ToPrimitive`/`NumCast` (lossy saturation is not a
//! conversion), `CheckedSub` (its signature is for types with `Sub`).

use num_traits::{Bounded, One, Pow, Zero};

use crate::float::LogFloat;
use crate::log::Log;
use crate::signed::SignedLog;

impl<F: LogFloat> Zero for Log<F> {
    /// [`Log::ZERO`].
    #[inline]
    fn zero() -> Self {
        Self::ZERO
    }
    #[inline]
    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

impl<F: LogFloat> One for Log<F> {
    /// [`Log::ONE`].
    #[inline]
    fn one() -> Self {
        Self::ONE
    }
    #[inline]
    fn is_one(&self) -> bool {
        *self == Self::ONE
    }
}

impl<F: LogFloat> Pow<F> for Log<F> {
    type Output = Self;
    /// [`Log::powf`].
    #[inline]
    #[track_caller]
    fn pow(self, p: F) -> Self {
        self.powf(p)
    }
}

impl<F: LogFloat> Pow<i32> for Log<F> {
    type Output = Self;
    /// [`Log::powi`].
    #[inline]
    fn pow(self, n: i32) -> Self {
        self.powi(n)
    }
}

impl<F: LogFloat> Bounded for Log<F> {
    /// [`Log::ZERO`], the least value.
    #[inline]
    fn min_value() -> Self {
        Self::ZERO
    }
    /// [`Log::INFINITY`], the greatest value. Unlike the floats' `max_value`, this is not a
    /// finite number: every `Log` is ≤ `INFINITY`, and nothing finite is an upper bound.
    #[inline]
    fn max_value() -> Self {
        Self::INFINITY
    }
}

impl<F: LogFloat> Zero for SignedLog<F> {
    /// [`SignedLog::ZERO`].
    #[inline]
    fn zero() -> Self {
        Self::ZERO
    }
    #[inline]
    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

impl<F: LogFloat> One for SignedLog<F> {
    /// [`SignedLog::ONE`].
    #[inline]
    fn one() -> Self {
        Self::ONE
    }
}
