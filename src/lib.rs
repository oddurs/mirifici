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
mod dd;
mod error;
mod float;
mod fmt;
mod log;
mod ops;
mod parse;
mod signed;
pub mod slice;

pub use crate::add::log1mexp;
pub use crate::error::DomainError;
pub use crate::float::LogFloat;
pub use crate::fmt::LogDisplay;
pub use crate::log::Log;
pub use crate::parse::ParseLogError;
pub use crate::signed::SignedLog;
