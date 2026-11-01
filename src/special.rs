//! Special functions whose values outgrow floats long before their logarithms do:
//! Γ(x), n! and binomial coefficients.

use crate::float::LogFloat;
use crate::float::sealed::Float;
use crate::log::Log;

/// ½ ln(2π).
const LN_SQRT_2PI: f64 = 0.9189385332046728;

/// Stirling's series: B₂ₖ / (2k(2k − 1)), k = 1..9. The first omitted term is below
/// 2·10⁻¹⁸ for x ≥ 10.
const STIRLING: [f64; 9] = [
    0.08333333333333333,
    -0.002777777777777778,
    0.0007936507936507937,
    -0.0005952380952380953,
    0.0008417508417508417,
    -0.0019175269175269176,
    0.00641025641025641,
    -0.029550653594771242,
    0.17964437236883057,
];

/// ln Γ(2 + z) = (1 − γ)z + Σₖ₌₂ (−1)ᵏ (ζ(k) − 1)/k · zᵏ, coefficients for k = 1..30 (from
/// mpmath at 50 digits). For |z| ≤ ½ the omitted tail is below 10⁻²⁰.
const SERIES_2: [f64; 30] = [
    0.42278433509846713,
    0.3224670334241132,
    -0.0673523010531981,
    0.020580808427784546,
    -0.007385551028673986,
    0.0028905103307415234,
    -0.001192753911703261,
    0.0005096695247430425,
    -0.00022315475845357939,
    9.945751278180853e-05,
    -4.492623673813314e-05,
    2.050721277567069e-05,
    -9.439488275268397e-06,
    4.374866789907488e-06,
    -2.039215753801366e-06,
    9.55141213040742e-07,
    -4.492469198764566e-07,
    2.1207184805554665e-07,
    -1.0043224823968099e-07,
    4.7698101693639804e-08,
    -2.2711094608943164e-08,
    1.0838659214896955e-08,
    -5.183475041970047e-09,
    2.4836745438024785e-09,
    -1.1921401405860912e-09,
    5.731367241678862e-10,
    -2.7595228851242334e-10,
    1.330476437424449e-10,
    -6.4229645638381e-11,
    3.1044247747322276e-11,
];

// Math goes through `Float` (not `f64`'s inherent methods) so that the `libm` feature gives
// the same bits on every platform here too.
fn ln(x: f64) -> f64 {
    Float::ln(x)
}
fn ln_1p(x: f64) -> f64 {
    Float::ln_1p(x)
}

/// ln Γ(2 + z) for |z| ≤ ½.
fn series_2(z: f64) -> f64 {
    let mut s = 0.0;
    for c in SERIES_2.iter().rev() {
        s = s * z + c;
    }
    s * z
}

/// ln Γ(x) for x > 0, in f64.
///
/// Near the zeros of ln Γ (x = 1 and x = 2) a power series about 2 keeps the absolute
/// error at a few ε; below ½ and between 2½ and 10 the recurrence Γ(x + 1) = xΓ(x) moves
/// the argument into the series' range without cancellation; from 10 up, Stirling's series.
pub(crate) fn ln_gamma(x: f64) -> f64 {
    if x == f64::INFINITY {
        return x;
    }
    if x >= 10.0 {
        let r = 1.0 / x;
        let r2 = r * r;
        let mut s = 0.0;
        for c in STIRLING.iter().rev() {
            s = s * r2 + c;
        }
        return (x - 0.5) * ln(x) - x + LN_SQRT_2PI + s * r;
    }
    if x < 0.5 {
        // Γ(x) = Γ(x + 2) / (x (x + 1))
        return series_2(x) - ln(x) - ln_1p(x);
    }
    if x < 1.5 {
        // Γ(x) = Γ(x + 1) / x, and x − 1 is exact here
        return series_2(x - 1.0) - ln_1p(x - 1.0);
    }
    if x <= 2.5 {
        return series_2(x - 2.0);
    }
    // 2.5 < x < 10: Γ(x) = Γ(y)·y(y + 1)…(x − 1) with y in (1.5, 2.5]; each y − 1 is exact.
    let mut y = x;
    let mut prod = 1.0;
    while y > 2.5 {
        y -= 1.0;
        prod *= y;
    }
    series_2(y - 2.0) + ln(prod)
}

/// ln C(n, k), exactly when it fits in a `u128` and from a sum of logs otherwise.
fn ln_binomial(n: u64, k: u64) -> f64 {
    let k = k.min(n - k);
    // Exact: C(n, i + 1) = C(n, i)·(n − i)/(i + 1), divisible at every step.
    let mut c: u128 = 1;
    let mut exact = true;
    for i in 0..k {
        let Some(p) = c.checked_mul(u128::from(n - i)) else {
            exact = false;
            break;
        };
        c = p / u128::from(i + 1);
    }
    if exact {
        return ln(c as f64);
    }
    if k <= 4096 {
        // Σ ln((n − i)/(i + 1)), every term rounded once. The running sum grows to |ln C|,
        // so plain addition would round thousands of times at that size; Neumaier's
        // compensated summation keeps the total within about one ulp of the exact sum.
        let (mut s, mut c) = (0.0f64, 0.0f64);
        for i in 0..k {
            let t = ln((n - i) as f64 / (i + 1) as f64);
            let u = s + t;
            c += if s.abs() >= t.abs() {
                (s - u) + t
            } else {
                (t - u) + s
            };
            s = u;
        }
        return s + c;
    }
    ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0)
}

impl<F: LogFloat> Log<F> {
    /// Γ(x), the gamma function, for x ≥ 0.
    ///
    /// Γ(n) = (n − 1)! for positive integers, and Γ grows so fast that Γ(172) already
    /// overflows `f64`; its logarithm does not, so `Log::gamma` covers every `x` up to about
    /// 10³⁰⁵. `Γ(0)` is the pole's limit from the right, [`Log::INFINITY`] (as `x / 0` is),
    /// and `Γ(∞) = ∞`.
    ///
    /// Computed in `f64` (also for `Log<f32>`): a power series for ln Γ about 2 on
    /// [½, 2½], the recurrence Γ(x + 1) = xΓ(x) below and up to 10, and Stirling's series
    /// from 10. Error: at most 3ε·max(1, |ln Γ(x)|) in the stored log for `f64` (measured
    /// 1.9) and ε·max(1, |ln Γ(x)|) for `f32`, against mpmath (`docs/accuracy.md`).
    ///
    /// # Panics
    ///
    /// If `x` is negative or NaN (Γ is negative for some negative arguments, and has poles
    /// at the non-positive integers).
    ///
    /// ```
    /// use mirifici::Log;
    /// assert!((Log::<f64>::gamma(5.0).to_value() - 24.0).abs() < 1e-13);
    /// assert!((Log::<f64>::gamma(0.5).to_value() - core::f64::consts::PI.sqrt()).abs() < 1e-15);
    /// // 999! = 4.0238726007709…·10²⁵⁶⁴, to the digits its stored log resolves
    /// assert_eq!(Log::<f64>::gamma(1000.0).to_string(), "4.02387260077e2564");
    /// ```
    #[must_use]
    #[track_caller]
    pub fn gamma(x: F) -> Self {
        assert!(x >= F::ZERO, "Log::gamma: argument must be >= 0, got {x:?}");
        if x == F::ZERO {
            return Self::INFINITY;
        }
        Self::from_ln(F::from_f64(ln_gamma(x.to_f64())))
    }

    /// n!, for any `u64`.
    ///
    /// Exact up to one rounding for n ≤ 20 (the product fits a `u64`), and Γ(n + 1)
    /// otherwise, with the error of [`Log::gamma`]. 1 000 000! is about 8.26·10⁵⁵⁶⁵⁷⁰⁸.
    ///
    /// ```
    /// use mirifici::Log;
    /// assert_eq!(Log::<f64>::factorial(5), Log::from_value(120.0));
    /// assert!(Log::<f64>::factorial(1_000_000).to_string().starts_with("8.26393168"));
    /// ```
    #[must_use]
    pub fn factorial(n: u64) -> Self {
        if n <= 20 {
            let p: u64 = (2..=n).product();
            return Self::from_ln(F::from_f64(ln(p as f64)));
        }
        Self::from_ln(F::from_f64(ln_gamma(n as f64 + 1.0)))
    }

    /// The binomial coefficient C(n, k) = n! / (k! (n − k)!). `ZERO` when k > n.
    ///
    /// Exact up to one rounding while C(n, k) fits in a `u128` (n up to about 130 for any
    /// k, much further for small k). Otherwise, for min(k, n − k) ≤ 4096, the compensated
    /// sum Σ ln((n − i)/(i + 1)), within about ε·|ln C| of the exact value. Beyond
    /// that, ln Γ(n + 1) − ln Γ(k + 1) − ln Γ(n − k + 1): the logs cancel, so the absolute
    /// error is about ε·n·ln n rather than ε·|ln C| (for n = 10⁹, about 5·10⁻⁶ relative).
    ///
    /// ```
    /// use mirifici::Log;
    /// assert!((Log::<f64>::binomial(10, 3).to_value() - 120.0).abs() < 1e-12);
    /// assert_eq!(Log::<f64>::binomial(3, 10), Log::ZERO);
    /// // C(10⁶, 5·10⁵) = 7.8995787…·10³⁰¹⁰²⁶, far beyond f64
    /// assert!(Log::<f64>::binomial(1_000_000, 500_000).to_string().starts_with("7.89957"));
    /// ```
    #[must_use]
    pub fn binomial(n: u64, k: u64) -> Self {
        if k > n {
            return Self::ZERO;
        }
        Self::from_ln(F::from_f64(ln_binomial(n, k)))
    }
}
