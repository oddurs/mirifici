//! A log-space number type: products become sums, and magnitudes far beyond `f64` stay
//! representable.
//!
//! # The problem logarithms solved
//!
//! At the end of the sixteenth century, astronomers and navigators spent much of their
//! working lives multiplying long numbers by hand. In 1614 John Napier published *Mirifici
//! Logarithmorum Canonis Descriptio* (“A Description of the Wonderful Canon of
//! Logarithms”): a table that paired numbers with “logarithms” (his coinage, from *logos*,
//! ratio, and *arithmos*, number) so that multiplying two numbers became adding two
//! entries. Division became subtraction, and roots became halving. Within a decade, Henry
//! Briggs had recast the table in base 10 and Kepler was using it to compute the
//! Rudolphine Tables.
//!
//! Floating-point numbers have the same problem in a different form. An `f64` cannot go
//! below about 10⁻³⁰⁸, so the probability of a long sequence (a product of a thousand
//! factors of 0.4) simply becomes 0, and the information is gone. The cure is Napier's:
//! store the logarithm and add. [`Log<F>`](Log) does that and keeps the bookkeeping out of
//! sight:
//!
//! ```
//! use mirifici::Log;
//!
//! let p = Log::from_value(0.4_f64);
//! let sequence: Log = std::iter::repeat_n(p, 1000).product();
//!
//! assert_eq!(0.4_f64.powi(1000), 0.0);                   // the float underflows
//! assert_eq!(format!("{sequence:.4}"), "1.1481e-398");    // Log does not
//! assert!(sequence > Log::ZERO);
//!
//! // Values add and compare as values, not as logarithms:
//! let total = sequence + sequence;
//! assert!(total > sequence);
//! assert_eq!(format!("{total:.3}"), "2.296e-398");
//! ```
//!
//! # What it guarantees
//!
//! - **Domain.** A `Log` is a real number x ≥ 0, stored as ln(x) in one `F` (`f32` or `f64`):
//!   [`Log::ZERO`] stores −∞, [`Log::ONE`] stores 0, [`Log::INFINITY`] stores +∞. The type
//!   is `#[repr(transparent)]`, so it is the size of `F`. For signed values use
//!   [`SignedLog`].
//! - **No NaN.** Constructing from a negative value or NaN is an error
//!   ([`Log::try_from_value`]) or a panic ([`Log::from_value`]). Indeterminate forms
//!   (0·∞, 0/0, ∞/∞, ∞−∞) return `None` from the `checked_*` methods and panic in the
//!   operators. No operation on valid inputs produces NaN.
//! - **Semantics of the operators.** `*`, `/`, `+` act on values. There is no `-`, because
//!   x − y leaves the domain when y > x: use [`Log::checked_sub`] or [`Log::abs_diff`].
//! - **Documented accuracy.** Every bound is an absolute error in the stored log, which is
//!   a relative error in the value; multiplication, division and powers are correctly
//!   rounded, addition and subtraction are within about one ulp. The bounds are tested
//!   against 60-digit references; see `docs/accuracy.md` in the repository.
//! - **Range.** `Log<f64>` covers e^(±1.8·10³⁰⁸). The price is precision far from 1: the
//!   stored log of 10⁻³⁰⁰ has about 13 significant digits of relative precision rather than
//!   16, because ulp(ln x) grows with |ln x|.
//!
//! # Overview
//!
//! | | |
//! |---|---|
//! | [`Log`] | the number type: construction, accessors, `*`, `/`, `+`, powers, ordering, `Display`, `FromStr` |
//! | [`SignedLog`] | sign plus `Log` magnitude, with `+`, `-`, `*`, `/`, unary `-` |
//! | [`slice`](mod@slice) | log-sum-exp, softmax, normalisation and means over slices |
//! | [`log1mexp`] | ln(1 − e^(−d)), the kernel of subtraction |
//! | [`LogDisplay`] | prints a `Log` as a power of any base: `2^-1024.5` |
//! | [`LogFloat`] | the sealed trait for `f32` and `f64` |
//!
//! # Features
//!
//! | feature | default | |
//! |---|---|---|
//! | `std` | yes | use the platform math library; without it the crate is `no_std` and uses `libm` |
//! | `serde` | no | `Serialize`/`Deserialize` for `Log` and `SignedLog`, and [`serde_as_value`] |
//! | `num-traits` | no | `Zero`, `One`, `Pow`, `Bounded` |
//! | `approx` | no | `AbsDiffEq`, `RelativeEq`, `UlpsEq` on the stored log |
//! | `bytemuck` | no | `Pod`, `Zeroable` for `Log<f32>` and `Log<f64>` |
//!
//! The crate never allocates.
//!
//! # A note on the name
//!
//! *Mirifici* is the first word of Napier's title: the canon of logarithms was
//! “wonderful”. The history notes in this documentation (Napier, Bürgi, Briggs,
//! Saint-Vincent and de Sarasa, Mercator) sit next to the parts of the API whose idea they
//! first described: [`Log`], [`Log::powi`], [`Log::log10`], [`Log::ln`], and `Log`'s `Add`.
//!
//! Sources: J. Napier, *Mirifici Logarithmorum Canonis Descriptio* (Edinburgh, 1614);
//! J. Bürgi, *Arithmetische und Geometrische Progress Tabulen* (Prague, 1620), edited by
//! K. M. Clark (Birkhäuser, 2015); H. Briggs, *Arithmetica Logarithmica* (London, 1624),
//! reconstructed by D. Roegel for the LOCOMAT project (2010); G. de Saint-Vincent, *Opus
//! geometricum quadraturae circuli et sectionum coni* (Antwerp, 1647); A. A. de Sarasa,
//! *Solutio problematis a R. P. Marino Mersenne propositi* (Antwerp, 1649); N. Mercator,
//! *Logarithmotechnia* (London, 1668). Secondary: J. Havil, *John Napier: Life,
//! Logarithms, and Legacy* (Princeton, 2014); E. Maor, *e: The Story of a Number*
//! (Princeton, 1994).

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;

mod add;
mod aggregate;
mod dd;
mod error;
mod float;
mod fmt;
mod integrations;
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

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub use crate::integrations::serde::serde_as_value;

/// The README's examples, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
