# Public API (0.1)

This page started as the API sketch agreed before implementation (M0) and has been kept in
step with the code; the differences from the original sketch are listed at the end.
Signatures are abbreviated; every item is documented in rustdoc with its exact formula,
edge cases and error bound.

```rust
// ---- the float trait ------------------------------------------------------------
pub trait LogFloat: sealed::Float {}          // implemented for f32 and f64 only

// ---- the number type ------------------------------------------------------------
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Log<F: LogFloat = f64>(/* ln(x) */ F);

impl<F: LogFloat> Log<F> {
    pub const ZERO: Self;       // ln = -inf
    pub const ONE: Self;        // ln = 0
    pub const INFINITY: Self;   // ln = +inf

    // construction (x ≥ 0; NaN and negatives rejected)
    pub fn from_value(x: F) -> Self;                          // panics on x < 0 or NaN
    pub fn try_from_value(x: F) -> Result<Self, DomainError>;
    pub fn from_ln(ln: F) -> Self;                            // panics on NaN
    pub fn try_from_ln(ln: F) -> Result<Self, DomainError>;
    pub const fn from_ln_unchecked(ln: F) -> Self;            // hot paths; NaN not checked
    pub fn from_log2(log2: F) -> Self;                        // panics on NaN
    pub fn from_log10(log10: F) -> Self;                      // panics on NaN

    // accessors
    pub const fn ln(self) -> F;
    pub fn log2(self) -> F;
    pub fn log10(self) -> F;
    pub fn log(self, base: F) -> F;                           // panics unless base > 0, finite, ≠ 1
    pub fn to_value(self) -> F;                               // saturates to 0 / inf
    pub fn try_to_value(self) -> Option<F>;                   // None on underflow / overflow

    // multiplicative (exact up to one rounding of the stored log)
    pub fn checked_mul(self, rhs: Self) -> Option<Self>;      // None for 0 · inf
    pub fn checked_div(self, rhs: Self) -> Option<Self>;      // None for 0/0 and inf/inf
    pub fn powf(self, p: F) -> Self;                          // 0^0 = 1; panics on NaN p
    pub fn powi(self, n: i32) -> Self;
    pub fn sqrt(self) -> Self;
    pub fn recip(self) -> Self;

    // additive
    pub fn checked_sub(self, rhs: Self) -> Option<Self>;      // None if rhs > self or inf - inf
    pub fn abs_diff(self, rhs: Self) -> Self;                 // panics on inf, inf

    // aggregation
    pub fn sum_two_pass(xs: &[Self]) -> Self;
    pub fn product_compensated<I: IntoIterator<Item = Self>>(iter: I) -> Self;

    // ordering
    pub fn total_cmp(&self, other: &Self) -> core::cmp::Ordering;
    pub fn max(self, other: Self) -> Self;
    pub fn min(self, other: Self) -> Self;

    // formatting
    pub fn display_base(self, base: F) -> LogDisplay<F>;      // "2^-1024.5"
}

pub fn log1mexp<F: LogFloat>(d: F) -> F;                     // ln(1 - e^-d), d ≥ 0 (Mächler 2012)

// Operators on Log: Mul, Div, Add (+ the *Assign forms, and by-reference forms),
// Sum, Product, Default (= ZERO), Debug ("Log(ln = -11512.925464970229)"),
// Display, LowerExp, UpperExp, FromStr. No Sub, no Neg, no Ord, no Hash.

#[non_exhaustive]
pub enum DomainError { Negative, NaN }                       // Display + core::error::Error
pub struct ParseLogError { /* opaque */ }                    // Display + core::error::Error
pub struct LogDisplay<F> { /* adapter */ }                   // Display

// ---- signed values ----------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct SignedLog<F: LogFloat = f64> { /* sign bit + Log magnitude */ }

impl<F: LogFloat> SignedLog<F> {
    pub const ZERO, ONE, NEG_ONE, INFINITY, NEG_INFINITY: Self;
    pub fn new(negative: bool, magnitude: Log<F>) -> Self;
    pub fn from_value(x: F) -> Self;                          // panics on NaN
    pub fn try_from_value(x: F) -> Result<Self, DomainError>;
    pub fn magnitude(self) -> Log<F>;
    pub fn is_negative(self) -> bool;                         // false for zero
    pub fn to_value(self) -> F;
    pub fn checked_add / checked_sub / checked_mul / checked_div(self, rhs) -> Option<Self>;
    pub fn total_cmp(&self, other: &Self) -> Ordering;
}
// Add, Sub, Mul, Div, Neg (+ assign forms), Sum, Product, PartialEq, PartialOrd,
// Debug, Display, LowerExp, UpperExp.
// From<Log<F>> for SignedLog<F> (lossless); TryFrom<SignedLog<F>> for Log<F> (DomainError).

// ---- slices (no allocation needed, so no feature gate) ----------------------------
pub mod slice {
    pub fn logsumexp<F: LogFloat>(ln: &[F]) -> F;
    pub fn log_softmax_in_place<F: LogFloat>(ln: &mut [F]) -> Option<F>;
    pub fn softmax_in_place<F: LogFloat>(ln: &mut [F]) -> Option<F>;
    pub fn log_mean_exp<F: LogFloat>(ln: &[F]) -> Option<F>;
    pub fn log_mean_exp_weighted<F: LogFloat>(ln: &[F], ln_weights: &[F]) -> Option<F>;
    pub fn normalize_in_place<F: LogFloat>(xs: &mut [Log<F>]) -> Option<Log<F>>;
    pub fn mean<F: LogFloat>(xs: &[Log<F>]) -> Option<Log<F>>;
    pub fn mean_weighted<F: LogFloat>(xs: &[Log<F>], weights: &[Log<F>]) -> Option<Log<F>>;
}

// ---- integrations (feature-gated) --------------------------------------------------
// serde:      Serialize/Deserialize as the stored log ("-inf"/"inf" strings in
//             human-readable formats); SignedLog as (negative, ln).
pub mod serde_as_value {                                     // #[serde(with = "...")]
    pub fn serialize<F, S>(x: &Log<F>, s: S) -> Result<S::Ok, S::Error>;
    pub fn deserialize<'de, F, D>(d: D) -> Result<Log<F>, D::Error>;
}
// num-traits: Zero, One, Pow<F>, Pow<i32>, Bounded for Log; Zero, One for SignedLog.
// approx:     AbsDiffEq, RelativeEq, UlpsEq on the stored log.
// bytemuck:   Pod, Zeroable (all-zero bits = ONE).
```

## Semantics checklist

| Brief | Sketch |
|---|---|
| single float holding ln(x), same size as `F` | `#[repr(transparent)] struct Log<F>(F)` |
| ZERO = -inf, ONE = 0, INFINITY = +inf | constants |
| negative / NaN construction is an error | `try_from_value` → `DomainError`, `from_value` panics |
| `from_ln_unchecked` for hot paths | yes, `const fn` |
| no `Sub`; `checked_sub`, `abs_diff` | yes |
| `ZERO * INFINITY` indeterminate | `checked_mul` → `None`, `Mul` panics |
| `ZERO / ZERO` indeterminate, x/0 = inf | `checked_div` → `None`, `Div` panics; also inf/inf (flagged in decisions.md) |
| 0^0 = 1 | `powf`, `powi` |
| log-sum-exp addition with ±inf special cases | `Add` |
| Mächler `log1mexp` | `log1mexp` |
| streaming `Sum`, `sum_two_pass` | yes |
| `Product`, `product_compensated` (Neumaier) | yes |
| PartialEq/PartialOrd on the log, `total_cmp`, Ord only on a wrapper if at all | no wrapper; see decisions.md |
| ln/log2/log10/log, saturating `to_value`, `try_to_value` | yes |
| sealed `LogFloat` for f32/f64, no `num_traits::Float` bound | yes |

## Changes from the M0 sketch

- `log2`, `log10`, `from_log2` and `from_log10` use double-double constants (M5), which
  changed their accuracy, not their signatures.
- `SignedLog` gained `total_cmp`, and `num-traits` `Zero`/`One` (M4, M6).
- No item was removed.
