//! `FromStr`: parsing decimal scientific notation with exponents of any size.

use core::fmt::{self, Write};
use core::str::FromStr;

use crate::dd;
use crate::float::LogFloat;
use crate::fmt::Buf;
use crate::log::Log;
use crate::signed::SignedLog;

/// An error from parsing a [`Log`] or [`SignedLog`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseLogError {
    kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Empty,
    Invalid,
    Negative,
    NaN,
}

impl fmt::Display for ParseLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind {
            Kind::Empty => "cannot parse a number from an empty string",
            Kind::Invalid => "invalid number syntax",
            Kind::Negative => "negative value; Log represents only x >= 0",
            Kind::NaN => "NaN is not a valid value",
        })
    }
}

impl core::error::Error for ParseLogError {}

const fn err(kind: Kind) -> ParseLogError {
    ParseLogError { kind }
}

/// Mantissa digits kept for the out-of-range path. Digits beyond these change the value by
/// a relative 10⁻⁴⁰, far below the precision of any stored log.
const MAX_DIGITS: usize = 40;

/// Parses an unsigned decimal (or `inf`) into its natural log. Returns `-inf` for zero.
fn parse_ln<F: LogFloat>(s: &str) -> Result<F, ParseLogError> {
    if s.is_empty() {
        return Err(err(Kind::Empty));
    }
    if s.eq_ignore_ascii_case("inf") || s.eq_ignore_ascii_case("infinity") {
        return Ok(F::INFINITY);
    }
    if s.eq_ignore_ascii_case("nan") {
        return Err(err(Kind::NaN));
    }

    let bytes = s.as_bytes();
    let mut i = 0;
    let mut digits = Buf::<{ MAX_DIGITS + 1 }>::new(); // d1 '.' d2 d3 …, see below
    let mut n_kept = 0;
    let mut seen_digit = false;
    let mut seen_nonzero = false;
    let mut after_point = false;
    // Significant digits before the point, and zeros between the point and the first
    // significant digit. Together they give the decimal exponent of the leading digit.
    let mut int_sig: i64 = 0;
    let mut frac_zeros: i64 = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => {
                seen_digit = true;
                let d = bytes[i];
                if !seen_nonzero && d == b'0' {
                    if after_point {
                        frac_zeros += 1;
                    }
                } else {
                    seen_nonzero = true;
                    if !after_point {
                        int_sig += 1;
                    }
                    if n_kept < MAX_DIGITS {
                        digits.push(d).map_err(|_| err(Kind::Invalid))?;
                        n_kept += 1;
                    }
                }
            }
            b'.' if !after_point => after_point = true,
            b'e' | b'E' => break,
            _ => return Err(err(Kind::Invalid)),
        }
        i += 1;
    }
    if !seen_digit {
        return Err(err(Kind::Invalid));
    }

    // Exponent: an optional sign and at least one digit, parsed exactly (as an f64, so
    // exponents of any length are accepted).
    let mut exp10 = 0.0f64;
    if i < bytes.len() {
        i += 1; // the 'e'
        let start = i;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let digits_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == digits_start || i != bytes.len() {
            return Err(err(Kind::Invalid));
        }
        exp10 = s[start..].parse().map_err(|_| err(Kind::Invalid))?;
    }

    if !seen_nonzero {
        return Ok(F::NEG_INFINITY);
    }

    // value = d1.d2d3… × 10^k
    let lead = if int_sig > 0 {
        int_sig - 1
    } else {
        -(frac_zeros + 1)
    };
    let k = lead as f64 + exp10;

    if k.abs() <= F::MAX_SAFE_EXP10 {
        // Comfortably inside the normal range: parse with the float's own correctly rounded
        // parser, so that `"0.1".parse::<Log>()` equals `Log::from_value(0.1)` exactly.
        let v: F = s.parse().map_err(|_| err(Kind::Invalid))?;
        return Ok(v.ln());
    }

    // Beyond the float's range: ln(m) + k·ln(10) in double-double.
    let mut m = Buf::<{ MAX_DIGITS + 2 }>::new();
    let d = digits.as_str().as_bytes();
    m.push(d[0])
        .and_then(|()| m.push(b'.'))
        .and_then(|()| m.write_str(&digits.as_str()[1..]))
        .map_err(|_| err(Kind::Invalid))?;
    let m: f64 = m.as_str().parse().map_err(|_| err(Kind::Invalid))?;
    Ok(F::from_f64(dd::from_decimal(m, k)))
}

/// Splits an optional leading sign.
fn split_sign(s: &str) -> (bool, &str) {
    match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    }
}

impl<F: LogFloat> FromStr for Log<F> {
    type Err = ParseLogError;

    /// Parses a non-negative decimal number, with exponents of any size.
    ///
    /// Accepts the syntax of `F`'s own `FromStr` for finite numbers (`2.5`, `.5`, `1e-3`,
    /// `+4E+7`), plus `inf` / `infinity` in any case. Exponents are not limited to the
    /// float's range: `"2.5e-90210"` parses to a `Log` whose stored log is the correctly
    /// rounded ln(2.5·10⁻⁹⁰²¹⁰), up to a few units in the last place of `ln(10)·exponent`.
    /// Inside the float's normal range the string is parsed by `F` itself and then
    /// converted, so `s.parse::<Log>()` equals `Log::from_value(s.parse::<f64>())` exactly.
    ///
    /// `-0` parses to zero. Magnitudes beyond the range of `Log<F>` itself (|ln x| above the
    /// largest finite `F`; for `f64`, decimal exponents beyond about ±7.8·10³⁰⁷) saturate to
    /// `ZERO` or `INFINITY`, as float parsing saturates.
    ///
    /// # Errors
    ///
    /// On empty input, invalid syntax, a negative non-zero value, or `NaN`.
    ///
    /// ```
    /// use mirifici::Log;
    /// let x: Log = "2.5e-90210".parse().unwrap();
    /// assert!((x.log10() - (-90210.0 + 2.5f64.log10())).abs() < 1e-9);
    /// assert_eq!(x.to_string(), "2.5e-90210");
    /// assert!("-1".parse::<Log>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, ParseLogError> {
        let (negative, rest) = split_sign(s);
        let ln = parse_ln::<F>(rest)?;
        if negative && ln != F::NEG_INFINITY {
            return Err(err(Kind::Negative));
        }
        Ok(Log(ln + F::ZERO))
    }
}

impl<F: LogFloat> FromStr for SignedLog<F> {
    type Err = ParseLogError;

    /// Parses a decimal number of either sign, with exponents of any size. Same syntax and
    /// accuracy as `Log`'s `FromStr`.
    ///
    /// # Errors
    ///
    /// On empty input, invalid syntax, or `NaN`.
    fn from_str(s: &str) -> Result<Self, ParseLogError> {
        let (negative, rest) = split_sign(s);
        let ln = parse_ln::<F>(rest)?;
        Ok(SignedLog::new(negative, Log(ln + F::ZERO)))
    }
}
