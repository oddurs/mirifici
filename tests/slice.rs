//! Slice utilities.

use mirifici::Log;
use mirifici::slice::*;

type L = Log<f64>;
const INF: f64 = f64::INFINITY;

#[test]
fn logsumexp_edges() {
    assert_eq!(logsumexp::<f64>(&[]), -INF);
    assert_eq!(logsumexp(&[-INF, -INF]), -INF);
    assert_eq!(logsumexp(&[0.0, INF, 3.0]), INF);
    assert!(logsumexp(&[0.0, f64::NAN]).is_nan());
    assert!(logsumexp(&[-INF, f64::NAN]).is_nan());
    assert!(logsumexp(&[f64::NAN, INF]).is_nan());
    assert_eq!(logsumexp(&[-3.5]), -3.5);
}

#[test]
fn logsumexp_matches_naive_in_range() {
    let ln = [-1.0, 0.5, 2.0, -3.0f64];
    let naive: f64 = ln.iter().map(|l| l.exp()).sum::<f64>().ln();
    assert!((logsumexp(&ln) - naive).abs() < 1e-15);
}

#[test]
fn logsumexp_out_of_range() {
    let ln = [-20000.0, -20000.0, -20000.0, -20000.0f64];
    assert!((logsumexp(&ln) - (-20000.0 + 4f64.ln())).abs() < 1e-11);
}

#[test]
fn log_softmax_and_softmax() {
    let mut a = [1.0, 2.0, 3.0f64];
    let mut b = a;
    let lse_a = log_softmax_in_place(&mut a).unwrap();
    let lse_b = softmax_in_place(&mut b).unwrap();
    assert_eq!(lse_a, lse_b);
    let z: f64 = [1.0f64, 2.0, 3.0].iter().map(|x| x.exp()).sum();
    for i in 0..3 {
        let p = ((i + 1) as f64).exp() / z;
        assert!((b[i] - p).abs() < 1e-15);
        assert!((a[i].exp() - p).abs() < 1e-15);
    }
    assert!((b.iter().sum::<f64>() - 1.0).abs() < 1e-15);
}

#[test]
fn softmax_keeps_zeros_and_handles_extreme_logits() {
    let mut x = [-INF, -1e5, -1e5 + 2f64.ln()];
    softmax_in_place(&mut x).unwrap();
    assert_eq!(x[0], 0.0);
    assert!((x[1] - 1.0 / 3.0).abs() < 1e-12);
    assert!((x[2] - 2.0 / 3.0).abs() < 1e-12);
}

#[test]
fn softmax_degenerate_inputs_leave_slice_unchanged() {
    for mut x in [
        vec![],
        vec![-INF, -INF],
        vec![0.0, INF],
        vec![0.0, f64::NAN],
    ] {
        let before = x.clone();
        assert_eq!(softmax_in_place(&mut x), None);
        assert_eq!(log_softmax_in_place(&mut x), None);
        assert_eq!(format!("{x:?}"), format!("{before:?}"));
    }
}

#[test]
fn log_mean_exp_cases() {
    assert_eq!(log_mean_exp::<f64>(&[]), None);
    let ln = [0.0, 2f64.ln()];
    assert!((log_mean_exp(&ln).unwrap() - 1.5f64.ln()).abs() < 1e-15);
    assert_eq!(log_mean_exp(&[-INF, -INF]), Some(-INF));
    assert_eq!(log_mean_exp(&[0.0, INF]), Some(INF));
}

#[test]
fn log_mean_exp_weighted_cases() {
    // weights 1 and 3 on values 1 and 2: (1 + 6) / 4
    let ln = [0.0, 2f64.ln()];
    let w = [0.0, 3f64.ln()];
    assert!((log_mean_exp_weighted(&ln, &w).unwrap() - 1.75f64.ln()).abs() < 1e-15);
    // a zero weight excludes even an infinite value
    let ln = [0.0, INF];
    let w = [0.0, -INF];
    assert_eq!(log_mean_exp_weighted(&ln, &w), Some(0.0));
    assert_eq!(log_mean_exp_weighted(&[0.0], &[-INF]), None);
    assert_eq!(log_mean_exp_weighted(&[0.0], &[INF]), None);
    assert_eq!(log_mean_exp_weighted::<f64>(&[], &[]), None);
}

#[test]
#[should_panic(expected = "length")]
fn log_mean_exp_weighted_length_mismatch() {
    let _ = log_mean_exp_weighted(&[0.0, 1.0], &[0.0]);
}

#[test]
fn normalize_sums_to_one() {
    let mut xs: Vec<L> = [1.0, 2.0, 5.0]
        .map(|v| L::from_ln(-9000.0 + f64::ln(v)))
        .to_vec();
    let total = normalize_in_place(&mut xs).unwrap();
    assert!((total.ln() - (-9000.0 + 8f64.ln())).abs() < 1e-11);
    let s: L = xs.iter().sum();
    // The inputs carry ulp(9000) ≈ 1.8e-12 of representation error.
    assert!(s.ln().abs() < 1e-11);
    assert!((xs[2].to_value() - 5.0 / 8.0).abs() < 1e-11);
}

#[test]
fn normalize_degenerate() {
    let mut empty: [L; 0] = [];
    assert_eq!(normalize_in_place(&mut empty), None);
    let mut zeros = [L::ZERO, L::ZERO];
    assert_eq!(normalize_in_place(&mut zeros), None);
    let mut inf = [L::ONE, L::INFINITY];
    assert_eq!(normalize_in_place(&mut inf), None);
    assert_eq!(inf, [L::ONE, L::INFINITY]);
}

#[test]
fn mean_and_weighted_mean() {
    assert_eq!(mean::<f64>(&[]), None);
    let xs = [L::from_value(1.0), L::from_value(2.0), L::from_value(6.0)];
    assert!((mean(&xs).unwrap().to_value() - 3.0).abs() < 1e-15);
    let w = [L::from_value(1.0), L::ZERO, L::from_value(1.0)];
    assert!((mean_weighted(&xs, &w).unwrap().to_value() - 3.5).abs() < 1e-15);
    assert_eq!(mean_weighted(&xs, &[L::ZERO; 3]), None);
    let with_inf = [L::ONE, L::INFINITY];
    assert_eq!(mean_weighted(&with_inf, &[L::ONE, L::ZERO]), Some(L::ONE));
}
