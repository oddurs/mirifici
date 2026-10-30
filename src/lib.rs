//! `mirifici`: a log-space number type.
//!
//! [`Log<F>`](Log) stores a non-negative real number x as ln(x). Multiplication becomes
//! addition of logarithms, division becomes subtraction, and values far outside the range of
//! `f64` (1e-5000, 1e+90000) remain representable and comparable.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod add;
mod aggregate;
mod error;
mod float;
mod log;
mod ops;

pub use crate::add::log1mexp;
pub use crate::error::DomainError;
pub use crate::float::LogFloat;
pub use crate::log::Log;
