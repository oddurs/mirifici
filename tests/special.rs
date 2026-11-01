//! Edge cases of the special functions, probability helpers, new slice tools and
//! `SignedLog` powers.

use mirifici::slice::{effective_sample_size, log_cumsum_exp_in_place, log_dot, sample_index};
use mirifici::{Log, SignedLog};
use proptest::prelude::*;

type L = Log<f64>;
const INF: f64 = f64::INFINITY;

fn close(a: f64, b: f64, rel: f64) -> bool {
    a == b || (a - b).abs() <= rel * b.abs().max(1.0)
}

// ---- gamma, factorial, binomial --------------------------------------------------------

#[test]
fn gamma_special_points() {
    assert_eq!(L::gamma(0.0), L::INFINITY);
    assert_eq!(L::gamma(INF), L::INFINITY);
    assert!(L::gamma(1.0).ln().abs() < 1e-16);
    assert!(L::gamma(2.0).ln().abs() < 1e-16);
    assert!(close(
        L::gamma(0.5).to_value(),
        core::f64::consts::PI.sqrt(),
        1e-15
    ));
    // Γ(x) ≈ 1/x for tiny x: the stored log is about −ln x
    assert!(close(L::gamma(1e-300).ln(), 1e-300f64.ln().abs(), 1e-15));
    // the Stirling branch begins at 10
    assert!(close(L::gamma(10.0).to_value(), 362_880.0, 1e-14));
    // continuity across the branch: ln Γ(10 − h) ≈ ln Γ(10) − h·ψ(10), ψ(10) = 2.2517525890…
    let h = 1e-9;
    let expect = 362_880f64.ln() - h * 2.251_752_589_066_721;
    assert!(close(L::gamma(10.0 - h).ln(), expect, 1e-15));
    // past e^(1.8e308) the log itself overflows
    assert_eq!(L::gamma(f64::MAX), L::INFINITY);
}

#[test]
#[should_panic(expected = ">= 0")]
fn gamma_negative_panics() {
    let _ = L::gamma(-0.5);
}

#[test]
#[should_panic(expected = ">= 0")]
fn gamma_nan_panics() {
    let _ = L::gamma(f64::NAN);
}

#[test]
fn factorials() {
    assert_eq!(L::factorial(0), L::ONE);
    assert_eq!(L::factorial(1), L::ONE);
    assert_eq!(L::factorial(20), L::from_value(2_432_902_008_176_640_000.0));
    assert!(close(
        L::factorial(21).to_value(),
        51_090_942_171_709_440_000.0,
        1e-14
    ));
    assert!(L::factorial(u64::MAX) < L::INFINITY);
    assert_eq!(Log::<f32>::factorial(5), Log::from_value(120.0f32));
}

#[test]
fn binomials() {
    assert_eq!(L::binomial(5, 0), L::ONE);
    assert_eq!(L::binomial(5, 5), L::ONE);
    assert_eq!(L::binomial(0, 0), L::ONE);
    assert_eq!(L::binomial(5, 6), L::ZERO);
    assert_eq!(L::binomial(52, 5), L::from_value(2_598_960.0));
    assert_eq!(L::binomial(1000, 3), L::binomial(1000, 997));
    // C(n, 1) = n even where n is not an f64
    assert!(close(
        L::binomial(u64::MAX, 1).ln(),
        (u64::MAX as f64).ln(),
        1e-15
    ));
    // Pascal's rule in log space
    let (n, k) = (100_000, 31_337);
    let lhs = L::binomial(n, k);
    let rhs = L::binomial(n - 1, k - 1) + L::binomial(n - 1, k);
    assert!(close(lhs.ln(), rhs.ln(), 1e-12));
}

// ---- probability helpers ---------------------------------------------------------------

#[test]
fn complements() {
    assert_eq!(L::ZERO.complement(), Some(L::ONE));
    assert_eq!(L::ONE.complement(), Some(L::ZERO));
    assert_eq!(L::from_value(1.5).complement(), None);
    assert_eq!(L::INFINITY.complement(), None);
    let p = L::from_value(0.25);
    assert!(close(p.complement().unwrap().to_value(), 0.75, 1e-15));
}

#[test]
fn logistic_edges() {
    assert_eq!(L::from_logit(INF), L::ONE);
    assert_eq!(L::from_logit(-INF), L::ZERO);
    assert_eq!(L::from_logit(0.0).to_value(), 0.5);
    assert_eq!(
        L::from_logit(0.0).ln().to_bits(),
        (-core::f64::consts::LN_2).to_bits()
    );
    // the tiny complement of σ(40) survives
    let c = L::from_logit(40.0).complement().unwrap();
    assert!(close(
        c.to_value(),
        (-40f64).exp() / (1.0 + (-40f64).exp()),
        1e-12
    ));
    assert_eq!(L::ZERO.logit(), Some(-INF));
    assert_eq!(L::ONE.logit(), Some(INF));
    assert_eq!(L::from_value(0.5).logit(), Some(0.0));
    assert_eq!(L::from_value(1.0 + 1e-15).logit(), None);
}

#[test]
#[should_panic(expected = "NaN")]
fn from_logit_nan_panics() {
    let _ = L::from_logit(f64::NAN);
}

proptest! {
    #[test]
    fn logit_inverts_from_logit(x in -700.0..700.0f64) {
        let back = L::from_logit(x).logit().unwrap();
        prop_assert!((back - x).abs() <= 8.0 * f64::EPSILON * x.abs().max(1.0), "{x} -> {back}");
    }

    #[test]
    fn sigma_and_its_complement_sum_to_one(x in -700.0..700.0f64) {
        let p = L::from_logit(x);
        let q = L::from_logit(-x);
        prop_assert!((p + q).ln().abs() <= 4.0 * f64::EPSILON);
        prop_assert_eq!(p.complement().map(|c| c.ln() - q.ln()).map(|d| d.abs() <= 1e-12 * q.ln().abs().max(1.0)), Some(true));
    }
}

// ---- slices ----------------------------------------------------------------------------

#[test]
fn log_dot_edges() {
    assert_eq!(log_dot::<f64>(&[], &[]), -INF);
    assert_eq!(log_dot(&[-INF, 0.0], &[0.0, -INF]), -INF);
    assert_eq!(log_dot(&[INF, 0.0], &[0.0, 0.0]), INF);
    assert!(
        log_dot(&[INF], &[-INF]).is_nan(),
        "∞·0 in a raw slice propagates as NaN"
    );
    let a = [-9000.0, -9001.0];
    let b = [-1.0, 0.0];
    let expect = (-9001f64) + 2f64.ln();
    assert!(close(log_dot(&a, &b), expect, 1e-15));
}

#[test]
#[should_panic(expected = "length")]
fn log_dot_length_mismatch() {
    let _ = log_dot(&[0.0, 1.0], &[0.0]);
}

#[test]
fn cumsum() {
    let mut empty: [f64; 0] = [];
    log_cumsum_exp_in_place(&mut empty);
    let mut x = [-INF, 0.0, -INF, 0.0];
    log_cumsum_exp_in_place(&mut x);
    assert_eq!(x[0], -INF);
    assert_eq!(x[1], 0.0);
    assert_eq!(x[2], 0.0);
    assert!(close(x[3], 2f64.ln(), 1e-16));
}

#[test]
fn sampling() {
    let w = [-INF, 1f64.ln(), -INF, 3f64.ln(), -INF];
    assert_eq!(sample_index(&w, 0.0), Some(1));
    assert_eq!(sample_index(&w, 0.2499), Some(1));
    assert_eq!(sample_index(&w, 0.2501), Some(3));
    assert_eq!(sample_index(&w, 0.999_999_999), Some(3));
    assert_eq!(sample_index(&w, 1.0), None);
    assert_eq!(sample_index(&w, -0.1), None);
    assert_eq!(sample_index(&w, f64::NAN), None);
    assert_eq!(sample_index::<f64>(&[], 0.5), None);
    assert_eq!(sample_index(&[-INF, -INF], 0.5), None);
    // frequencies follow the weights, also far beyond the float range
    let w = [-50_000.0, -50_000.0 + 3f64.ln()];
    let hits = (0..10_000)
        .filter(|i| sample_index(&w, (f64::from(*i) + 0.5) / 10_000.0) == Some(1))
        .count();
    assert_eq!(hits, 7_500);
}

#[test]
fn ess() {
    assert_eq!(effective_sample_size::<f64>(&[]), None);
    assert_eq!(effective_sample_size(&[-INF, -INF]), None);
    assert_eq!(effective_sample_size(&[0.0, INF]), None);
    assert!(close(effective_sample_size(&[0.0]).unwrap(), 1.0, 1e-15));
    // weights 1, 1, 2: (4)² / (1 + 1 + 4) = 8/3
    let w = [0.0, 0.0, 2f64.ln()];
    assert!(close(effective_sample_size(&w).unwrap(), 8.0 / 3.0, 1e-15));
}

// ---- SignedLog powers ------------------------------------------------------------------

#[test]
fn signed_powers() {
    let x = SignedLog::from_value(-2.0_f64);
    assert!(x.powi(3).is_negative());
    assert!(!x.powi(2).is_negative());
    assert!(!x.powi(-2).is_negative());
    assert!(x.powi(-3).is_negative());
    assert_eq!(x.powi(0), SignedLog::ONE);
    assert_eq!(SignedLog::<f64>::ZERO.powi(0), SignedLog::ONE);
    assert_eq!(SignedLog::<f64>::ZERO.powi(-1), SignedLog::INFINITY);
    assert!(close(x.recip().to_value(), -0.5, 1e-15));
    assert_eq!(SignedLog::<f64>::ZERO.recip(), SignedLog::INFINITY);
    assert_eq!(SignedLog::<f64>::NEG_INFINITY.recip(), SignedLog::ZERO);
}
