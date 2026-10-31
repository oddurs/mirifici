//! `Display`, `LowerExp`, `UpperExp` and [`LogDisplay`]: printing values beyond the float's
//! range.

use core::fmt::{self, Write};

use crate::dd;
use crate::float::LogFloat;
use crate::log::Log;
use crate::signed::SignedLog;

/// A fixed-capacity, stack-allocated string.
pub(crate) struct Buf<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Buf<N> {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        // Only whole `&str`s are ever appended.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    pub(crate) fn push(&mut self, b: u8) -> fmt::Result {
        if self.len == N {
            return Err(fmt::Error);
        }
        self.bytes[self.len] = b;
        self.len += 1;
        Ok(())
    }
}

impl<const N: usize> Write for Buf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Display,
    LowerExp,
    UpperExp,
}

/// Mantissa digits `m` rounded to `p` decimals, with the carry from 9.99… to 10 folded into
/// the exponent. Returns `None` if the digits do not fit the buffer.
fn mantissa(m: f64, k: f64, p: usize) -> Option<(Buf<128>, f64)> {
    let mut buf = Buf::new();
    write!(buf, "{m:.p$}").ok()?;
    if buf.as_str().starts_with("10") {
        let mut carried = Buf::new();
        write!(carried, "{:.p$}", 1.0).ok()?;
        return Some((carried, k + 1.0));
    }
    Some((buf, k))
}

/// Formats a finite stored log `ln` in scientific notation, choosing the shortest mantissa
/// for which `roundtrips` accepts the parsed-back log when no precision is given.
fn write_sci(
    ln: f64,
    negative: bool,
    f: &mut fmt::Formatter<'_>,
    upper: bool,
    roundtrips: &dyn Fn(f64) -> bool,
) -> fmt::Result {
    let (m, k) = dd::to_decimal(ln);
    let digits = match f.precision() {
        Some(p) => mantissa(m, k, p),
        // |log10 x| ≥ 2^52: the stored log cannot resolve a mantissa (see `to_decimal`).
        None if k.abs() >= 4503599627370496.0 => mantissa(m, k, 0),
        None => {
            let mut found = None;
            for p in 0..=17 {
                let Some((d, kk)) = mantissa(m, k, p) else {
                    break;
                };
                let back: f64 = d.as_str().parse().unwrap_or(f64::NAN);
                let done = p == 17 || roundtrips(dd::from_decimal(back, kk));
                if done {
                    found = Some((d, kk));
                    break;
                }
            }
            found
        }
    };
    let e = if upper { "E" } else { "e" };
    let Some((digits, k)) = digits else {
        // Only reachable for precisions above ~100 digits: write without padding.
        let p = f.precision().unwrap_or(0);
        if negative {
            f.write_str("-")?;
        }
        write!(f, "{m:.p$}{e}")?;
        return write_exponent(f, k);
    };
    let mut out = Buf::<512>::new();
    out.write_str(digits.as_str())?;
    out.write_str(e)?;
    write_exponent(&mut out, k)?;
    f.pad_integral(!negative, "", out.as_str())
}

/// Writes an integer-valued float exponent without a fractional part.
fn write_exponent(w: &mut dyn Write, k: f64) -> fmt::Result {
    if k.abs() < 1e18 {
        write!(w, "{}", k as i64)
    } else {
        // Display of an integer-valued f64 prints its exact integer digits.
        write!(w, "{k}")
    }
}

/// Shared implementation of the three formatting traits for `Log` and `SignedLog`.
fn fmt_value<F: LogFloat>(
    ln: F,
    negative: bool,
    f: &mut fmt::Formatter<'_>,
    style: Style,
) -> fmt::Result {
    let x = Log(ln);
    if let Some(v) = x.try_to_value() {
        if v == F::ZERO || v == F::INFINITY || v >= F::MIN_POSITIVE {
            let v = if negative { -v } else { v };
            return match style {
                Style::Display => fmt::Display::fmt(&v, f),
                Style::LowerExp => fmt::LowerExp::fmt(&v, f),
                Style::UpperExp => fmt::UpperExp::fmt(&v, f),
            };
        }
    }
    write_sci(
        ln.to_f64(),
        negative,
        f,
        style == Style::UpperExp,
        &|back| F::from_f64(back) == ln,
    )
}

impl<F: LogFloat> fmt::Display for Log<F> {
    /// Prints the represented value.
    ///
    /// Inside the normal range of `F`, the output is exactly that of `F`'s own `Display`
    /// (positional notation; `{:.3}` gives three decimals). Outside it, where converting to
    /// `F` would print `0` or `inf`, the value is printed in scientific notation computed
    /// from the stored log: `1e-5000`, `2.5e-90210`. There, the precision flag sets the
    /// number of mantissa decimals, and without it the shortest mantissa that parses back
    /// to the same `Log` is chosen. Width, fill, alignment, `+` and `0` flags are honoured.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::from_value(0.25_f64).to_string(), "0.25");
    /// assert_eq!(Log::from_log10(-5000.0_f64).to_string(), "1e-5000");
    /// assert_eq!(format!("{:.3}", Log::from_log10(-5000.0_f64)), "1.000e-5000");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self.0, false, f, Style::Display)
    }
}

impl<F: LogFloat> fmt::LowerExp for Log<F> {
    /// Scientific notation with `e`. Matches `F`'s `LowerExp` in the normal range and
    /// extends it beyond: `format!("{:e}", x)` gives `1e-5000`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self.0, false, f, Style::LowerExp)
    }
}

impl<F: LogFloat> fmt::UpperExp for Log<F> {
    /// Scientific notation with `E`, as [`LowerExp`](fmt::LowerExp).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self.0, false, f, Style::UpperExp)
    }
}

impl<F: LogFloat> fmt::Display for SignedLog<F> {
    /// Prints the value with a leading `-` when negative; otherwise as `Log`'s `Display`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self.magnitude().0, self.is_negative(), f, Style::Display)
    }
}

impl<F: LogFloat> fmt::LowerExp for SignedLog<F> {
    /// As `Log`'s `LowerExp`, with a leading `-` when negative.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self.magnitude().0, self.is_negative(), f, Style::LowerExp)
    }
}

impl<F: LogFloat> fmt::UpperExp for SignedLog<F> {
    /// As `Log`'s `UpperExp`, with a leading `-` when negative.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self.magnitude().0, self.is_negative(), f, Style::UpperExp)
    }
}

/// Displays a [`Log`] as a power of a chosen base, such as `2^-1024.5` or `10^-308.2`.
///
/// Created by [`Log::display_base`]. The precision flag applies to the exponent; width and
/// fill are ignored. Zero prints as `0` and infinity as `inf`. The base is printed with
/// `F`'s `Display`, except that Euler's number prints as `e`.
///
/// ```
/// use mirifici::Log;
/// let x = Log::from_log2(-1024.5_f64);
/// assert_eq!(x.display_base(2.0).to_string(), "2^-1024.5");
/// assert_eq!(format!("{:.1}", x.display_base(10.0)), "10^-308.4");
/// assert_eq!(format!("{:.2}", x.display_base(core::f64::consts::E)), "e^-710.13");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct LogDisplay<F: LogFloat> {
    value: Log<F>,
    base: F,
}

impl<F: LogFloat> Log<F> {
    /// An adapter that displays this value as `base^exponent`. See [`LogDisplay`].
    ///
    /// # Panics
    ///
    /// Unless `base` is finite, positive and not 1 (as for [`Log::log`]).
    #[track_caller]
    #[must_use]
    pub fn display_base(self, base: F) -> LogDisplay<F> {
        // Validates the base with the same message as `log`.
        let _ = Log::<F>::ONE.log(base);
        LogDisplay { value: self, base }
    }
}

impl<F: LogFloat> fmt::Display for LogDisplay<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ln = self.value.0;
        if ln == F::NEG_INFINITY {
            return f.write_str("0");
        }
        if ln == F::INFINITY {
            return f.write_str("inf");
        }
        if self.base == F::E {
            f.write_str("e^")?;
        } else {
            write!(f, "{}^", self.base)?;
        }
        let exponent = self.value.log(self.base);
        match f.precision() {
            Some(p) => write!(f, "{exponent:.p$}"),
            None => write!(f, "{exponent}"),
        }
    }
}
