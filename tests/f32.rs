//! `Log<f32>`: the same semantics as `Log<f64>`, at single precision.

use mirifici::{DomainError, Log, log1mexp};

type L = Log<f32>;
const INF: f32 = f32::INFINITY;

#[test]
fn representation() {
    assert_eq!(size_of::<L>(), 4);
    assert_eq!(L::ZERO.ln(), -INF);
    assert_eq!(L::ONE.ln(), 0.0);
    assert_eq!(L::INFINITY.ln(), INF);
}

#[test]
fn construction() {
    assert_eq!(L::from_value(0.0), L::ZERO);
    assert_eq!(L::try_from_value(-1.0), Err(DomainError::Negative));
    assert_eq!(L::try_from_value(f32::NAN), Err(DomainError::NaN));
    assert_eq!(L::from_value(0.5).ln(), 0.5f32.ln());
    // f32 values reach only ~1e-45, but Log<f32> holds 1e-1000.
    let tiny = L::from_log10(-1000.0);
    assert!(tiny > L::ZERO);
    assert_eq!(tiny.try_to_value(), None);
}

#[test]
fn arithmetic_edges() {
    let x = L::from_value(3.0);
    assert_eq!(L::ZERO.checked_mul(L::INFINITY), None);
    assert_eq!(x * L::ZERO, L::ZERO);
    assert_eq!(x / L::ZERO, L::INFINITY);
    assert_eq!(L::ZERO.checked_div(L::ZERO), None);
    assert_eq!(L::ZERO.powf(0.0), L::ONE);
    assert_eq!(x + L::ZERO, x);
    assert_eq!(L::INFINITY + L::INFINITY, L::INFINITY);
    assert_eq!(x.checked_sub(x), Some(L::ZERO));
    assert_eq!(L::INFINITY.checked_sub(L::INFINITY), None);
    assert_eq!(log1mexp(0.0f32), -INF);
}

#[test]
fn sums_and_products() {
    let xs: Vec<L> = (1..=100).map(|i| L::from_value(i as f32)).collect();
    let s: L = xs.iter().sum();
    assert!((s.to_value() - 5050.0).abs() < 5050.0 * 4.0 * f32::EPSILON * 10.0);
    assert!((L::sum_two_pass(&xs).to_value() - 5050.0).abs() < 1e-2);
    let p: L = xs.iter().product();
    // 100! ≈ 9.33e157, far beyond f32
    assert!((p.log10() - 157.97).abs() < 1e-2);
    assert!((L::product_compensated(xs.iter().copied()).log10() - 157.97).abs() < 1e-2);
}

#[test]
fn widening_is_lossless() {
    for x in [
        L::ZERO,
        L::ONE,
        L::INFINITY,
        L::from_log10(-1000.0),
        L::from_value(0.3),
    ] {
        let w: Log<f64> = x.into();
        assert_eq!(w.ln(), f64::from(x.ln()));
    }
    let s: mirifici::SignedLog<f64> = mirifici::SignedLog::<f32>::from_value(-2.5).into();
    assert_eq!(
        s,
        mirifici::SignedLog::new(true, Log::<f32>::from_value(2.5).into())
    );
}
