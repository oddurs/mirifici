//! The sealed float trait that `Log` is generic over.

use core::cmp::Ordering;
use core::fmt::{Debug, Display, LowerExp, UpperExp};
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A floating-point type that can be the storage of a [`Log`](crate::Log).
///
/// This trait is sealed: it is implemented for [`f32`] and [`f64`] only, and cannot be
/// implemented outside this crate. It deliberately exposes no methods of its own, so the
/// crate can route math through the platform library (feature `std`) or through
/// [`libm`](https://docs.rs/libm) (without `std`) without that choice leaking into the
/// public API. Write `F: LogFloat` in your own generic code that wraps `Log<F>`.
pub trait LogFloat: sealed::Float {}

pub(crate) mod sealed {
    use super::*;

    /// Everything the crate needs from a float. Not nameable outside the crate.
    pub trait Float:
        Copy
        + PartialOrd
        + Debug
        + Display
        + LowerExp
        + UpperExp
        + Default
        + Send
        + Sync
        + 'static
        + Add<Output = Self>
        + Sub<Output = Self>
        + Mul<Output = Self>
        + Div<Output = Self>
        + Neg<Output = Self>
        + AddAssign
        + SubAssign
    {
        const ZERO: Self;
        const ONE: Self;
        const HALF: Self;
        const INFINITY: Self;
        const NEG_INFINITY: Self;
        const EPSILON: Self;
        const MAX: Self;
        const MIN_POSITIVE: Self;
        const E: Self;
        const LN_2: Self;
        const LN_10: Self;
        const LOG2_E: Self;
        const LOG10_E: Self;

        fn exp(self) -> Self;
        fn ln(self) -> Self;
        fn ln_1p(self) -> Self;
        fn exp_m1(self) -> Self;
        fn floor(self) -> Self;
        fn abs(self) -> Self;
        fn is_nan(self) -> bool;
        fn is_finite(self) -> bool;
        fn total_cmp(&self, other: &Self) -> Ordering;
        /// Rounds to nearest.
        fn from_f64(x: f64) -> Self;
        /// Exact.
        fn to_f64(self) -> f64;
        /// Exact for `f64`; rounds for `f32` when `|n| > 2^24`.
        fn from_i32(n: i32) -> Self;
    }
}

macro_rules! impl_float {
    (
        $t:ident,
        exp = $exp:ident,
        ln = $ln:ident,
        ln_1p = $ln_1p:ident,
        exp_m1 = $exp_m1:ident,
        floor = $floor:ident
    ) => {
        impl LogFloat for $t {}

        impl sealed::Float for $t {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const HALF: Self = 0.5;
            const INFINITY: Self = $t::INFINITY;
            const NEG_INFINITY: Self = $t::NEG_INFINITY;
            const EPSILON: Self = $t::EPSILON;
            const MAX: Self = $t::MAX;
            const MIN_POSITIVE: Self = $t::MIN_POSITIVE;
            const E: Self = core::$t::consts::E;
            const LN_2: Self = core::$t::consts::LN_2;
            const LN_10: Self = core::$t::consts::LN_10;
            const LOG2_E: Self = core::$t::consts::LOG2_E;
            const LOG10_E: Self = core::$t::consts::LOG10_E;

            #[inline]
            fn exp(self) -> Self {
                #[cfg(feature = "std")]
                return $t::exp(self);
                #[cfg(not(feature = "std"))]
                return libm::$exp(self);
            }
            #[inline]
            fn ln(self) -> Self {
                #[cfg(feature = "std")]
                return $t::ln(self);
                #[cfg(not(feature = "std"))]
                return libm::$ln(self);
            }
            #[inline]
            fn ln_1p(self) -> Self {
                #[cfg(feature = "std")]
                return $t::ln_1p(self);
                #[cfg(not(feature = "std"))]
                return libm::$ln_1p(self);
            }
            #[inline]
            fn exp_m1(self) -> Self {
                #[cfg(feature = "std")]
                return $t::exp_m1(self);
                #[cfg(not(feature = "std"))]
                return libm::$exp_m1(self);
            }
            #[inline]
            fn floor(self) -> Self {
                #[cfg(feature = "std")]
                return $t::floor(self);
                #[cfg(not(feature = "std"))]
                return libm::$floor(self);
            }
            #[inline]
            fn abs(self) -> Self {
                $t::abs(self)
            }
            #[inline]
            fn is_nan(self) -> bool {
                $t::is_nan(self)
            }
            #[inline]
            fn is_finite(self) -> bool {
                $t::is_finite(self)
            }
            #[inline]
            fn total_cmp(&self, other: &Self) -> Ordering {
                $t::total_cmp(self, other)
            }
            #[inline]
            fn from_f64(x: f64) -> Self {
                x as $t
            }
            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }
            #[inline]
            fn from_i32(n: i32) -> Self {
                n as $t
            }
        }
    };
}

impl_float!(
    f64,
    exp = exp,
    ln = log,
    ln_1p = log1p,
    exp_m1 = expm1,
    floor = floor
);
impl_float!(
    f32,
    exp = expf,
    ln = logf,
    ln_1p = log1pf,
    exp_m1 = expm1f,
    floor = floorf
);
