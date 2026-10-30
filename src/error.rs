//! Error types.

use core::fmt;

/// A float that cannot be the value (or the logarithm) of a [`Log`](crate::Log).
///
/// `Log` represents x ≥ 0, so negative values and NaN are rejected at construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DomainError {
    /// The value was negative (`-0.0` is accepted as zero).
    Negative,
    /// The value (or logarithm) was NaN.
    NaN,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DomainError::Negative => "value is negative; Log represents only x >= 0",
            DomainError::NaN => "value is NaN; Log never stores NaN",
        })
    }
}

impl core::error::Error for DomainError {}
