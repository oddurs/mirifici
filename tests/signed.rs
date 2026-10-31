//! `SignedLog`.
#![allow(clippy::op_ref)]

use core::cmp::Ordering;
use mirifici::{DomainError, Log, SignedLog};

type S = SignedLog<f64>;
type L = Log<f64>;

fn s(x: f64) -> S {
    S::from_value(x)
}

fn approx(a: S, b: f64) -> bool {
    (a.to_value() - b).abs() <= 1e-14 * b.abs().max(1.0)
}

#[test]
fn construction_and_zero_sign() {
    assert_eq!(s(0.0), S::ZERO);
    assert_eq!(s(-0.0), S::ZERO);
    assert!(!s(-0.0).is_negative());
    assert!(s(-2.0).is_negative());
    assert_eq!(s(-2.0).magnitude(), L::from_value(2.0));
    assert_eq!(S::new(true, L::ZERO), S::ZERO);
    assert_eq!(S::try_from_value(f64::NAN), Err(DomainError::NaN));
    assert_eq!(s(f64::NEG_INFINITY), S::NEG_INFINITY);
    assert_eq!(S::default(), S::ZERO);
}

#[test]
#[should_panic(expected = "NaN")]
fn from_value_nan_panics() {
    let _ = s(f64::NAN);
}

#[test]
fn conversions() {
    let x = L::from_log10(-5000.0);
    let sx = S::from(x);
    assert_eq!(Log::try_from(sx), Ok(x));
    assert_eq!(Log::try_from(-sx), Err(DomainError::Negative));
    assert_eq!(Log::try_from(-S::ZERO), Ok(L::ZERO));
}

#[test]
fn arithmetic_signs() {
    let cases = [
        (2.0, 5.0),
        (-2.0, 5.0),
        (2.0, -5.0),
        (-2.0, -5.0),
        (5.0, -5.0),
        (0.0, -3.0),
    ];
    for (a, b) in cases {
        assert!(approx(s(a) + s(b), a + b), "{a} + {b}");
        assert!(approx(s(a) - s(b), a - b), "{a} - {b}");
        assert!(approx(s(a) * s(b), a * b), "{a} * {b}");
        if b != 0.0 {
            assert!(approx(s(a) / s(b), a / b), "{a} / {b}");
        }
        assert!(approx(-s(a), -a));
    }
    assert_eq!(s(5.0) + s(-5.0), S::ZERO);
    assert!(!(s(5.0) + s(-5.0)).is_negative());
    assert_eq!(s(-3.0) * S::ZERO, S::ZERO);
}

#[test]
fn infinities_and_indeterminate_forms() {
    assert_eq!(S::INFINITY + s(-1e300), S::INFINITY);
    assert_eq!(S::NEG_INFINITY - s(1.0), S::NEG_INFINITY);
    assert_eq!(S::INFINITY.checked_add(S::NEG_INFINITY), None);
    assert_eq!(S::INFINITY.checked_sub(S::INFINITY), None);
    assert_eq!(S::ZERO.checked_mul(S::NEG_INFINITY), None);
    assert_eq!(S::ZERO.checked_div(S::ZERO), None);
    assert_eq!(S::INFINITY.checked_div(S::NEG_INFINITY), None);
    assert_eq!(s(-2.0) / S::ZERO, S::NEG_INFINITY);
    assert_eq!(s(2.0) / S::ZERO, S::INFINITY);
    assert_eq!(s(-2.0) * S::NEG_INFINITY, S::INFINITY);
}

#[test]
#[should_panic(expected = "indeterminate")]
fn add_inf_minus_inf_panics() {
    let _ = S::INFINITY + S::NEG_INFINITY;
}

#[test]
#[should_panic(expected = "indeterminate")]
fn mul_zero_inf_panics() {
    let _ = S::ZERO * S::INFINITY;
}

#[test]
fn assign_and_reference_forms() {
    let mut x = s(1.0);
    x += s(2.0);
    x -= &s(4.0);
    x *= s(-3.0);
    x /= &s(2.0);
    assert!(approx(x, 1.5));
    assert!(approx(&s(1.0) + &s(2.0), 3.0));
    assert!(approx(-&s(1.0), -1.0));
}

#[test]
fn ordering() {
    let vals = [
        S::NEG_INFINITY,
        s(-1e300),
        s(-1.0),
        s(-1e-300),
        S::ZERO,
        s(1e-300),
        S::ONE,
        S::INFINITY,
    ];
    for w in vals.windows(2) {
        assert!(w[0] < w[1], "{:?} < {:?}", w[0], w[1]);
        assert_eq!(w[0].total_cmp(&w[1]), Ordering::Less);
    }
    assert_eq!(S::NEG_ONE, s(-1.0));
}

#[test]
fn sum_and_product() {
    let xs = [s(1.0), s(-2.0), s(3.0), s(-4.0)];
    assert!(approx(xs.iter().sum(), -2.0));
    assert!(approx(xs.iter().product(), 24.0));
    assert!(approx(xs[..3].iter().product(), -6.0));
    assert!(approx(xs.into_iter().product(), 24.0));
    assert!(s(1.0) > s(-1.0) && s(-1.0) < s(1.0));
    let empty: [S; 0] = [];
    assert_eq!(empty.iter().sum::<S>(), S::ZERO);
    assert_eq!(empty.iter().product::<S>(), S::ONE);
    // alternating series beyond f64 range: Σ (-1)^k 1e-5000·2^-k = (2/3)·1e-5000
    let tiny = L::from_log10(-5000.0);
    let series: S = (0..60)
        .map(|k| S::new(k % 2 == 1, tiny * L::from_value(0.5f64.powi(k))))
        .sum();
    let expected = tiny * L::from_value(2.0 / 3.0);
    assert!((series.magnitude().ln() - expected.ln()).abs() < 1e-11);
    assert!(!series.is_negative());
}

#[test]
#[should_panic(expected = "indeterminate")]
fn sum_of_opposite_infinities_panics() {
    let _: S = [S::INFINITY, S::NEG_INFINITY].into_iter().sum();
}

#[test]
fn debug_format() {
    assert_eq!(format!("{:?}", s(-1.0)), "SignedLog(-, ln = 0.0)");
    assert_eq!(format!("{:?}", S::ZERO), "SignedLog(+, ln = -inf)");
}

#[test]
fn try_to_value() {
    assert_eq!(s(-2.5).try_to_value(), Some(-2.5));
    assert_eq!(S::NEG_INFINITY.try_to_value(), Some(f64::NEG_INFINITY));
    assert_eq!(S::new(true, L::from_log10(-400.0)).try_to_value(), None);
}
