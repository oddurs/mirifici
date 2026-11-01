//! Operations on whole slices: log-sum-exp, softmax, normalisation and means.
//!
//! Functions taking `&[F]` work on **raw stored logarithms** (log-probabilities, logits,
//! log-weights), the form in which such data usually arrives. Functions taking `&[Log<F>]`
//! work on values. None of them allocates, so they are available without `std` or `alloc`.
//!
//! Raw slices are not validated. A NaN element makes a reduction return NaN, and makes
//! the in-place functions leave the slice unchanged and return `None`.
//!
//! All reductions use the two-pass log-sum-exp of [`Log::sum_two_pass`]: find the maximum
//! m, then compute `m + ln_1p(Σ_{i≠max} exp(lᵢ − m))`. Error: about n·ε in the stored log in
//! the worst case (typically √n·ε), see `docs/accuracy.md`.

use crate::aggregate::{Streaming, sum_two_pass_by};
use crate::float::LogFloat;
use crate::log::Log;

/// ln(Σ exp(lᵢ)): the logarithm of the sum of the values whose logs are `ln`.
///
/// The empty sum is −∞ (the log of 0). If any element is +∞ the result is +∞.
///
/// ```
/// use mirifici::slice::logsumexp;
/// let ln = [-1000.0, -1000.0];
/// assert!((logsumexp(&ln) - (-1000.0 + 2f64.ln())).abs() < 1e-12);
/// ```
#[inline]
#[must_use]
pub fn logsumexp<F: LogFloat>(ln: &[F]) -> F {
    sum_two_pass_by(ln, |&l| l)
}

/// Replaces each lᵢ by lᵢ − logsumexp(l), so that the represented values sum to one. Returns
/// the log-sum-exp that was subtracted (the log normaliser).
///
/// Returns `None` and leaves the slice unchanged when the normaliser is not finite: the
/// slice is empty, every element is −∞, or an element is +∞ or NaN.
///
/// ```
/// use mirifici::slice::{log_softmax_in_place, logsumexp};
/// let mut logits = [1.0, 2.0, 3.0_f64];
/// let lse = log_softmax_in_place(&mut logits).unwrap();
/// assert!((lse - 3.40760596444438).abs() < 1e-12);
/// assert!(logsumexp(&logits).abs() < 1e-15);
/// ```
pub fn log_softmax_in_place<F: LogFloat>(ln: &mut [F]) -> Option<F> {
    let lse = logsumexp(ln);
    if !lse.is_finite() {
        return None;
    }
    for l in ln.iter_mut() {
        *l -= lse;
    }
    Some(lse)
}

/// Replaces each lᵢ by exp(lᵢ − logsumexp(l)): converts logits to probabilities in place.
/// Returns the log normaliser.
///
/// The output holds plain probabilities (not logs) that sum to one up to rounding: each is
/// computed from `lᵢ − logsumexp`, which carries an absolute error of about ε·|lᵢ|, so
/// logits of magnitude 10⁶ give probabilities good to about 10⁻¹⁰. Returns
/// `None` and leaves the slice unchanged under the same conditions as
/// [`log_softmax_in_place`].
///
/// ```
/// use mirifici::slice::softmax_in_place;
/// let mut x = [0.0, 0.0_f64];
/// softmax_in_place(&mut x).unwrap();
/// assert_eq!(x, [0.5, 0.5]);
/// ```
pub fn softmax_in_place<F: LogFloat>(ln: &mut [F]) -> Option<F> {
    let lse = logsumexp(ln);
    if !lse.is_finite() {
        return None;
    }
    for l in ln.iter_mut() {
        *l = (*l - lse).exp();
    }
    Some(lse)
}

/// ln((1/n) Σ exp(lᵢ)): the log of the mean of the values whose logs are `ln`.
///
/// This is the estimator behind importance-sampling and marginal-likelihood estimates such
/// as the IWAE bound. Computed as `logsumexp(ln) − ln(n)`. Returns `None` for an empty
/// slice.
#[must_use]
pub fn log_mean_exp<F: LogFloat>(ln: &[F]) -> Option<F> {
    if ln.is_empty() {
        return None;
    }
    Some(logsumexp(ln) - len_ln::<F>(ln.len()))
}

/// ln(Σ wᵢ exp(lᵢ) / Σ wᵢ), with the weights also given as logs (`ln_weights[i] = ln wᵢ`).
///
/// Terms with zero weight (`ln wᵢ = −∞`) are skipped, even if their value is infinite.
/// Returns `None` if the total weight is zero or infinite (including an empty slice).
///
/// # Panics
///
/// If the slices have different lengths.
#[must_use]
pub fn log_mean_exp_weighted<F: LogFloat>(ln: &[F], ln_weights: &[F]) -> Option<F> {
    assert_eq!(
        ln.len(),
        ln_weights.len(),
        "log_mean_exp_weighted: values and weights differ in length"
    );
    let total = logsumexp(ln_weights);
    if !total.is_finite() {
        return None;
    }
    let mut acc = Streaming::NEW;
    for (&l, &w) in ln.iter().zip(ln_weights) {
        if w != F::NEG_INFINITY {
            acc.push(l + w);
        }
    }
    Some(acc.finish() - total)
}

/// Divides every value by the total so that the slice sums to [`Log::ONE`]. Returns the
/// total.
///
/// This is the rescaling step of a scaled forward algorithm: the returned totals multiply
/// to the likelihood. Returns `None` and leaves the slice unchanged when the total is
/// `ZERO` (including an empty slice) or `INFINITY`.
///
/// ```
/// use mirifici::{Log, slice::normalize_in_place};
/// let mut xs = [Log::from_log10(-5000.0_f64), Log::from_log10(-5000.0)];
/// let total = normalize_in_place(&mut xs).unwrap();
/// assert!((xs[0].to_value() - 0.5).abs() < 1e-12);
/// assert!((total.log10() - (-5000.0 + 2f64.log10())).abs() < 1e-9);
/// ```
pub fn normalize_in_place<F: LogFloat>(xs: &mut [Log<F>]) -> Option<Log<F>> {
    let total = Log::sum_two_pass(xs);
    if total == Log::ZERO || total == Log::INFINITY {
        return None;
    }
    for x in xs.iter_mut() {
        x.0 -= total.0;
    }
    Some(total)
}

/// The arithmetic mean (1/n) Σ xᵢ. Returns `None` for an empty slice.
#[must_use]
pub fn mean<F: LogFloat>(xs: &[Log<F>]) -> Option<Log<F>> {
    if xs.is_empty() {
        return None;
    }
    Some(Log(Log::sum_two_pass(xs).0 - len_ln::<F>(xs.len())))
}

/// The weighted mean Σ wᵢ xᵢ / Σ wᵢ.
///
/// Terms with zero weight are skipped, even if their value is infinite. Returns `None` if
/// the total weight is zero or infinite (including empty slices).
///
/// # Panics
///
/// If the slices have different lengths.
#[must_use]
pub fn mean_weighted<F: LogFloat>(xs: &[Log<F>], weights: &[Log<F>]) -> Option<Log<F>> {
    assert_eq!(
        xs.len(),
        weights.len(),
        "mean_weighted: values and weights differ in length"
    );
    let total = Log::sum_two_pass(weights);
    if total == Log::ZERO || total == Log::INFINITY {
        return None;
    }
    let mut acc = Streaming::NEW;
    for (x, w) in xs.iter().zip(weights) {
        if *w != Log::ZERO {
            acc.push(x.0 + w.0);
        }
    }
    Some(Log(acc.finish() - total.0))
}

/// ln(n), for dividing a sum by its number of terms.
#[inline]
fn len_ln<F: LogFloat>(n: usize) -> F {
    // `as f64` is exact below 2^53; F::from_f64 then rounds once.
    F::from_f64(n as f64).ln()
}

/// ln Σ exp(aᵢ + bᵢ): the log of the dot product of two vectors given as logs.
///
/// This is the inner step of the HMM forward algorithm (α'ⱼ = Σᵢ αᵢ·Aᵢⱼ) and of any
/// log-space matrix–vector product, computed in two passes like [`logsumexp`] without
/// forming aᵢ + bᵢ in memory. `−∞` for empty slices.
///
/// # Panics
///
/// If the slices have different lengths.
///
/// ```
/// use mirifici::slice::log_dot;
/// let a = [0.5f64.ln(), 0.5f64.ln()];
/// let b = [0.2f64.ln(), 0.6f64.ln()];
/// assert!((log_dot(&a, &b).exp() - 0.4).abs() < 1e-15);
/// ```
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)] // deliberate: also true for NaN
pub fn log_dot<F: LogFloat>(a: &[F], b: &[F]) -> F {
    assert_eq!(a.len(), b.len(), "log_dot: slices differ in length");
    let mut m = F::NEG_INFINITY;
    let mut at = 0;
    for (i, (&x, &y)) in a.iter().zip(b).enumerate() {
        let t = x + y;
        // `!(t <= m)`: greater, or NaN (from NaN input, or −∞ + ∞), which propagates.
        if !(t <= m) {
            if t.is_nan() {
                return t;
            }
            m = t;
            at = i;
        }
    }
    if m == F::NEG_INFINITY || m == F::INFINITY {
        return m;
    }
    let mut s = F::ZERO;
    for (i, (&x, &y)) in a.iter().zip(b).enumerate() {
        if i != at {
            s += (x + y - m).exp();
        }
    }
    m + s.ln_1p()
}

/// Replaces each lᵢ by ln Σⱼ≤ᵢ exp(lⱼ): running sums of the values whose logs are `ln`.
///
/// The last element becomes the total ([`logsumexp`]). Cumulative sums are what
/// inverse-CDF sampling and the prefix sums of forward–backward need. Each step is one
/// log-sum-exp addition, so element i carries the error of i additions.
///
/// ```
/// use mirifici::slice::log_cumsum_exp_in_place;
/// let mut x = [1f64.ln(), 2f64.ln(), 3f64.ln()];
/// log_cumsum_exp_in_place(&mut x);
/// let v: Vec<f64> = x.iter().map(|l| l.exp()).collect();
/// assert!((v[0] - 1.0).abs() < 1e-15 && (v[1] - 3.0).abs() < 1e-15 && (v[2] - 6.0).abs() < 1e-14);
/// ```
pub fn log_cumsum_exp_in_place<F: LogFloat>(ln: &mut [F]) {
    let mut acc = F::NEG_INFINITY;
    for l in ln.iter_mut() {
        acc = crate::add::ln_add_exp(acc, *l);
        *l = acc;
    }
}

/// Draws an index with probability proportional to exp(lᵢ), given a uniform `u` in [0, 1).
///
/// Inverse-CDF sampling in log space: the first i whose running log-sum-exp exceeds
/// ln u + logsumexp(l). Bring your own random number generator, so the crate needs none;
/// with `rand`, pass `rng.random::<f64>()`. Entries of −∞ (zero weight) are never chosen.
/// This is the resampling step of a particle filter, or sampling from a softmax.
///
/// Returns `None` if `u` is not in [0, 1) or if the total weight is zero, infinite or NaN.
/// One pass, O(n); for many draws from the same weights, take
/// [`log_cumsum_exp_in_place`] once and binary-search it.
///
/// ```
/// use mirifici::slice::sample_index;
/// let w = [f64::NEG_INFINITY, 1f64.ln(), 3f64.ln()]; // probabilities 0, 1/4, 3/4
/// assert_eq!(sample_index(&w, 0.1), Some(1));
/// assert_eq!(sample_index(&w, 0.3), Some(2));
/// assert_eq!(sample_index(&w, 1.0), None);
/// ```
#[must_use]
pub fn sample_index<F: LogFloat>(ln_weights: &[F], u: F) -> Option<usize> {
    if !(u >= F::ZERO && u < F::ONE) {
        return None;
    }
    let total = logsumexp(ln_weights);
    if !total.is_finite() {
        return None;
    }
    let target = u.ln() + total;
    let mut acc = F::NEG_INFINITY;
    let mut last = None;
    for (i, &l) in ln_weights.iter().enumerate() {
        if l == F::NEG_INFINITY {
            continue;
        }
        acc = crate::add::ln_add_exp(acc, l);
        last = Some(i);
        if acc > target {
            return last;
        }
    }
    // Rounding can leave the running sum a hair below the total for u close to 1.
    last
}

/// Kish's effective sample size (Σwᵢ)² / Σwᵢ² of importance weights given as logs.
///
/// Computed relative to the largest weight, so weights far outside the range of a float (as
/// importance weights in high dimensions are) work. The result is a plain number
/// between 1 (one weight dominates) and n (all weights equal). Returns `None` if every
/// weight is zero, or a weight is infinite or NaN.
///
/// ```
/// use mirifici::slice::effective_sample_size;
/// let equal = [-5000.0_f64; 8];
/// assert!((effective_sample_size(&equal).unwrap() - 8.0).abs() < 1e-9);
/// let lopsided = [0.0_f64, -1000.0, -1000.0];
/// assert!((effective_sample_size(&lopsided).unwrap() - 1.0).abs() < 1e-12);
/// ```
#[must_use]
pub fn effective_sample_size<F: LogFloat>(ln_weights: &[F]) -> Option<F> {
    // Relative to the largest weight m: ESS = (Σ e^(lᵢ−m))² / Σ e^(2(lᵢ−m)). Doubling the
    // logs themselves would overflow for weights near e^(f64::MAX).
    let mut m = F::NEG_INFINITY;
    for &l in ln_weights {
        if l.is_nan() {
            return None;
        }
        if l > m {
            m = l;
        }
    }
    if !m.is_finite() {
        return None;
    }
    let (mut s1, mut s2) = (F::ZERO, F::ZERO);
    for &l in ln_weights {
        let w = (l - m).exp();
        s1 += w;
        s2 += w * w;
    }
    Some(s1 * s1 / s2)
}
