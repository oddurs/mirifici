//! `serde`: `Log` and `SignedLog` serialize as their stored logarithm.
//!
//! The stored log is exact and compact, so it is the default representation:
//!
//! - In binary formats (bincode, postcard, …) a `Log<F>` is exactly one `F`.
//! - In human-readable formats (JSON, TOML, YAML, …) a finite stored log is a number. The
//!   infinite logs of `Log::ZERO` and `Log::INFINITY` are written as the strings `"-inf"`
//!   and `"inf"`, because JSON has no infinities and would otherwise write `null` for
//!   zero, the most common probability of all.
//! - A `SignedLog` is a pair `(negative, ln)`: a bool and the stored log as above.
//!
//! Deserialization accepts the same forms and rejects NaN. Round trips are bit-exact when
//! the format parses floats exactly; for `serde_json` that requires its `float_roundtrip`
//! feature (its default parser can be off by an ulp). To exchange the represented
//! *value* instead (`0.25`, `"1e-5000"`), use [`serde_as_value`](crate::serde_as_value).

use core::fmt;
use core::marker::PhantomData;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, SerializeTuple, Serializer};

use crate::float::LogFloat;
use crate::log::Log;
use crate::signed::SignedLog;

impl<F: LogFloat> Serialize for Log<F> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            if self.0 == F::NEG_INFINITY {
                return s.serialize_str("-inf");
            }
            if self.0 == F::INFINITY {
                return s.serialize_str("inf");
            }
        }
        self.0.serialize(s)
    }
}

/// Accepts a stored log as a number, or as `"-inf"` / `"inf"` / `"+inf"`.
struct LnVisitor<F>(PhantomData<F>);

impl<F: LogFloat> LnVisitor<F> {
    fn finish<E: de::Error>(ln: f64) -> Result<Log<F>, E> {
        Log::try_from_ln(F::from_f64(ln)).map_err(E::custom)
    }
}

impl<F: LogFloat> Visitor<'_> for LnVisitor<F> {
    type Value = Log<F>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a natural logarithm (a number, \"-inf\" or \"inf\")")
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Log<F>, E> {
        Self::finish(v)
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Log<F>, E> {
        Self::finish(f64::from(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Log<F>, E> {
        Self::finish(v as f64)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Log<F>, E> {
        Self::finish(v as f64)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Log<F>, E> {
        match v {
            "-inf" => Ok(Log::ZERO),
            "inf" | "+inf" => Ok(Log::INFINITY),
            _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
        }
    }
}

impl<'de, F: LogFloat> Deserialize<'de> for Log<F> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            d.deserialize_any(LnVisitor(PhantomData))
        } else {
            let ln = F::deserialize(d)?;
            Log::try_from_ln(ln).map_err(de::Error::custom)
        }
    }
}

impl<F: LogFloat> Serialize for SignedLog<F> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(2)?;
        t.serialize_element(&self.is_negative())?;
        t.serialize_element(&self.magnitude())?;
        t.end()
    }
}

impl<'de, F: LogFloat> Deserialize<'de> for SignedLog<F> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (negative, magnitude) = <(bool, Log<F>)>::deserialize(d)?;
        Ok(SignedLog::new(negative, magnitude))
    }
}

/// Serialize a `Log` as its value rather than its logarithm, for human-readable output.
///
/// Use with `#[serde(with = "mirifici::serde_as_value")]`. A value that is a normal float
/// (or exactly 0) is written as a number; anything else, a value beyond the float's range
/// or infinity, is written as a string in scientific notation (`"1e-5000"`, `"inf"`), which
/// round-trips exactly through [`FromStr`](core::str::FromStr). Deserialization accepts
/// numbers and such strings, and rejects negative values and NaN.
///
/// In range, a number round-trips through one rounding of the value (the float closest to
/// x, then its log), so the stored log may change by about an ulp. Use the default
/// representation when bit-exactness matters.
///
/// ```
/// # #[cfg(feature = "serde")] {
/// use mirifici::Log;
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct Model {
///     #[serde(with = "mirifici::serde_as_value")]
///     likelihood: Log,
/// }
/// let m = Model { likelihood: Log::from_log10(-5000.0) };
/// assert_eq!(serde_json::to_string(&m).unwrap(), r#"{"likelihood":"1e-5000"}"#);
/// # }
/// ```
pub mod serde_as_value {
    use super::{Deserializer, Log, LogFloat, PhantomData, Serializer, Visitor, de, fmt};

    /// Serializes the value of `x`: a number in range, a scientific-notation string beyond.
    ///
    /// # Errors
    ///
    /// Only those of the serializer.
    pub fn serialize<F: LogFloat, S: Serializer>(x: &Log<F>, s: S) -> Result<S::Ok, S::Error> {
        match x.try_to_value() {
            Some(v) if v == F::ZERO || (v >= F::MIN_POSITIVE && v < F::INFINITY) => v.serialize(s),
            _ => s.collect_str(&format_args!("{x:e}")),
        }
    }

    /// Deserializes a value written by [`serialize`] (a non-negative number or a decimal
    /// string).
    ///
    /// # Errors
    ///
    /// Those of the deserializer, and negative, NaN or unparsable input.
    pub fn deserialize<'de, F: LogFloat, D: Deserializer<'de>>(d: D) -> Result<Log<F>, D::Error> {
        d.deserialize_any(ValueVisitor(PhantomData))
    }

    struct ValueVisitor<F>(PhantomData<F>);

    impl<F: LogFloat> Visitor<'_> for ValueVisitor<F> {
        type Value = Log<F>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a non-negative number or a decimal string such as \"1e-5000\"")
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Log<F>, E> {
            // Take the log in f64 before narrowing to F, so a `Log<f32>` keeps values such
            // as 1e-50 that an f32 cannot hold.
            let x = Log::<f64>::try_from_value(v).map_err(E::custom)?;
            Ok(Log::from_ln(F::from_f64(x.ln())))
        }

        fn visit_f32<E: de::Error>(self, v: f32) -> Result<Log<F>, E> {
            self.visit_f64(f64::from(v))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Log<F>, E> {
            self.visit_f64(v as f64)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Log<F>, E> {
            self.visit_f64(v as f64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Log<F>, E> {
            v.parse().map_err(E::custom)
        }
    }
}
