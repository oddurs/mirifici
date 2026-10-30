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
/// The output holds plain probabilities (not logs) that sum to one up to rounding. Returns
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
