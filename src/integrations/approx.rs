//! `approx`: comparisons on the stored logarithm.
//!
//! An absolute tolerance on the stored log is a relative tolerance on the value:
//! `assert_abs_diff_eq!(a, b, epsilon = 1e-12)` checks |ln a − ln b| ≤ 1e-12, i.e. that a and
//! b agree to a relative 10⁻¹². `ZERO` and `INFINITY` are equal only to themselves.

use approx::{AbsDiffEq, RelativeEq, UlpsEq};

use crate::float::LogFloat;
use crate::log::Log;

impl<F: LogFloat> AbsDiffEq for Log<F> {
    type Epsilon = F;

    #[inline]
    fn default_epsilon() -> F {
        F::default_epsilon()
    }

    #[inline]
    fn abs_diff_eq(&self, other: &Self, epsilon: F) -> bool {
        self.0 == other.0 || self.0.abs_diff_eq(&other.0, epsilon)
    }
}

impl<F: LogFloat> RelativeEq for Log<F> {
    #[inline]
    fn default_max_relative() -> F {
        F::default_max_relative()
    }

    #[inline]
    fn relative_eq(&self, other: &Self, epsilon: F, max_relative: F) -> bool {
        self.0 == other.0 || self.0.relative_eq(&other.0, epsilon, max_relative)
    }
}

impl<F: LogFloat> UlpsEq for Log<F> {
    #[inline]
    fn default_max_ulps() -> u32 {
        F::default_max_ulps()
    }

    #[inline]
    fn ulps_eq(&self, other: &Self, epsilon: F, max_ulps: u32) -> bool {
        self.0 == other.0 || self.0.ulps_eq(&other.0, epsilon, max_ulps)
    }
}
