//! Edge cases of construction, accessors, multiplicative arithmetic and ordering.
#![allow(clippy::op_ref)]

use core::cmp::Ordering;
use mirifici::{DomainError, Log};

type L = Log<f64>;
const INF: f64 = f64::INFINITY;

fn bits(x: L) -> u64 {
    x.ln().to_bits()
}

// ---- constants and representation ------------------------------------------------------

#[test]
fn constants_store_expected_logs() {
    assert_eq!(L::ZERO.ln(), -INF);
    assert_eq!(L::ONE.ln(), 0.0);
    assert_eq!(bits(L::ONE), 0, "ONE stores +0.0");
    assert_eq!(L::INFINITY.ln(), INF);
    assert_eq!(L::default(), L::ZERO);
}

#[test]
fn same_size_as_float() {
    assert_eq!(size_of::<Log<f64>>(), size_of::<f64>());
    assert_eq!(align_of::<Log<f64>>(), align_of::<f64>());
}

// ---- construction ----------------------------------------------------------------------

#[test]
fn from_value_special_values() {
    assert_eq!(L::from_value(0.0), L::ZERO);
    assert_eq!(L::from_value(-0.0), L::ZERO);
    assert_eq!(L::from_value(1.0), L::ONE);
    assert_eq!(L::from_value(INF), L::INFINITY);
    assert_eq!(
        L::from_value(f64::MIN_POSITIVE).ln(),
        f64::MIN_POSITIVE.ln()
    );
    assert_eq!(L::from_value(5e-324).ln(), 5e-324f64.ln());
    assert_eq!(L::from_value(f64::MAX).ln(), f64::MAX.ln());
}

#[test]
fn try_from_value_rejects_negative_and_nan() {
    assert_eq!(L::try_from_value(-1.0), Err(DomainError::Negative));
    assert_eq!(L::try_from_value(-5e-324), Err(DomainError::Negative));
    assert_eq!(L::try_from_value(-INF), Err(DomainError::Negative));
    assert_eq!(L::try_from_value(f64::NAN), Err(DomainError::NaN));
    assert_eq!(L::try_from_value(-f64::NAN), Err(DomainError::NaN));
}

#[test]
#[should_panic(expected = "negative")]
fn from_value_panics_on_negative() {
    let _ = L::from_value(-2.0);
}

#[test]
#[should_panic(expected = "NaN")]
fn from_value_panics_on_nan() {
    let _ = L::from_value(f64::NAN);
}

#[test]
fn from_ln_accepts_everything_but_nan() {
    assert_eq!(L::from_ln(-INF), L::ZERO);
    assert_eq!(L::from_ln(INF), L::INFINITY);
    assert_eq!(L::from_ln(-11512.9).ln(), -11512.9);
    assert_eq!(L::try_from_ln(f64::NAN), Err(DomainError::NaN));
    assert_eq!(bits(L::from_ln(-0.0)), 0, "-0 normalised to +0");
}

#[test]
#[should_panic(expected = "NaN")]
fn from_ln_panics_on_nan() {
    let _ = L::from_ln(f64::NAN);
}

#[test]
fn from_ln_unchecked_is_const_and_raw() {
    const HALF: L = Log::from_ln_unchecked(-core::f64::consts::LN_2);
    assert_eq!(HALF.to_value(), 0.5);
}

#[test]
fn from_log2_and_log10() {
    assert_eq!(L::from_log2(-1.0).to_value(), 0.5);
    assert_eq!(L::from_log2(-INF), L::ZERO);
    assert_eq!(L::from_log10(INF), L::INFINITY);
    assert!((L::from_log10(3.0).to_value() - 1000.0).abs() < 1e-12);
    let tiny = L::from_log10(-5000.0);
    assert!((tiny.ln() - -5000.0 * core::f64::consts::LN_10).abs() < 1e-12);
}

#[test]
#[should_panic(expected = "NaN")]
fn from_log10_panics_on_nan() {
    let _ = L::from_log10(f64::NAN);
}

// ---- accessors -------------------------------------------------------------------------

#[test]
fn log_accessors() {
    let x = L::from_value(1024.0);
    assert_eq!(x.ln(), 1024f64.ln());
    assert!((x.log2() - 10.0).abs() < 1e-14);
    assert!((L::from_value(1e5).log10() - 5.0).abs() < 1e-14);
    assert!((x.log(4.0) - 5.0).abs() < 1e-14);
    assert_eq!(L::ZERO.log2(), -INF);
    assert_eq!(L::INFINITY.log10(), INF);
    assert_eq!(L::ZERO.log(0.5), INF);
}

#[test]
#[should_panic(expected = "base")]
fn log_base_one_panics() {
    let _ = L::ONE.log(1.0);
}

#[test]
#[should_panic(expected = "base")]
fn log_base_zero_panics() {
    let _ = L::ONE.log(0.0);
}

#[test]
fn to_value_saturates() {
    assert_eq!(L::ZERO.to_value(), 0.0);
    assert_eq!(L::INFINITY.to_value(), INF);
    assert_eq!(L::ONE.to_value(), 1.0);
    assert_eq!(L::from_ln(-800.0).to_value(), 0.0);
    assert_eq!(L::from_ln(800.0).to_value(), INF);
}

#[test]
fn try_to_value_detects_underflow_and_overflow() {
    assert_eq!(L::ZERO.try_to_value(), Some(0.0));
    assert_eq!(L::INFINITY.try_to_value(), Some(INF));
    assert_eq!(L::from_value(3.5).try_to_value(), Some(3.5));
    assert_eq!(L::from_ln(-800.0).try_to_value(), None);
    assert_eq!(L::from_ln(800.0).try_to_value(), None);
    // subnormal results are returned
    let sub = L::from_ln(-720.0).try_to_value().unwrap();
    assert!(sub > 0.0 && sub < f64::MIN_POSITIVE);
}

// ---- multiplication --------------------------------------------------------------------

#[test]
fn mul_is_sum_of_logs() {
    let a = L::from_ln(-3000.0);
    let b = L::from_ln(-4000.5);
    assert_eq!((a * b).ln(), -7000.5);
    assert_eq!((a * L::ONE), a);
    assert_eq!(
        L::from_value(2.0) * L::from_value(4.0),
        L::from_ln(2f64.ln() + 4f64.ln())
    );
}

#[test]
fn mul_with_zero_and_infinity() {
    let x = L::from_value(7.0);
    assert_eq!(x * L::ZERO, L::ZERO);
    assert_eq!(L::ZERO * L::ZERO, L::ZERO);
    assert_eq!(x * L::INFINITY, L::INFINITY);
    assert_eq!(L::INFINITY * L::INFINITY, L::INFINITY);
    assert_eq!(L::ZERO.checked_mul(L::INFINITY), None);
    assert_eq!(L::INFINITY.checked_mul(L::ZERO), None);
}

#[test]
#[should_panic(expected = "indeterminate")]
fn mul_zero_by_infinity_panics() {
    let _ = L::ZERO * L::INFINITY;
}

#[test]
fn mul_overflow_and_underflow_of_the_log() {
    let big = L::from_ln(f64::MAX);
    assert_eq!(big * big, L::INFINITY);
    let small = L::from_ln(-f64::MAX);
    assert_eq!(small * small, L::ZERO);
}

#[test]
fn mul_reference_and_assign_forms() {
    let a = L::from_value(3.0);
    let b = L::from_value(5.0);
    let p = a * b;
    assert_eq!(&a * b, p);
    assert_eq!(a * &b, p);
    assert_eq!(&a * &b, p);
    let mut c = a;
    c *= b;
    assert_eq!(c, p);
    let mut d = a;
    d *= &b;
    assert_eq!(d, p);
}

// ---- division --------------------------------------------------------------------------

#[test]
fn div_is_difference_of_logs() {
    let a = L::from_ln(-3000.0);
    let b = L::from_ln(-4000.5);
    assert_eq!((a / b).ln(), 1000.5);
    assert_eq!(bits(a / a), 0, "x/x stores +0");
}

#[test]
fn div_by_zero_and_infinity() {
    let x = L::from_value(7.0);
    assert_eq!(x / L::ZERO, L::INFINITY);
    assert_eq!(L::INFINITY / L::ZERO, L::INFINITY);
    assert_eq!(x / L::INFINITY, L::ZERO);
    assert_eq!(L::ZERO / L::INFINITY, L::ZERO);
    assert_eq!(L::ZERO / x, L::ZERO);
    assert_eq!(L::ZERO.checked_div(L::ZERO), None);
    assert_eq!(L::INFINITY.checked_div(L::INFINITY), None);
}

#[test]
#[should_panic(expected = "indeterminate")]
fn div_zero_by_zero_panics() {
    let _ = L::ZERO / L::ZERO;
}

#[test]
#[should_panic(expected = "indeterminate")]
fn div_infinity_by_infinity_panics() {
    let _ = L::INFINITY / L::INFINITY;
}

#[test]
fn div_assign_forms() {
    let mut a = L::from_value(8.0);
    a /= L::from_value(2.0);
    a /= &L::from_value(2.0);
    assert!((a.to_value() - 2.0).abs() < 1e-15);
}

// ---- powers ----------------------------------------------------------------------------

#[test]
fn powf_basic() {
    let x = L::from_ln(-2.5);
    assert_eq!(x.powf(4.0).ln(), -10.0);
    assert_eq!(x.powf(-1.0).ln(), 2.5);
    assert_eq!(x.powf(1.0), x);
}

#[test]
fn powf_zero_to_the_zero_is_one() {
    assert_eq!(L::ZERO.powf(0.0), L::ONE);
    assert_eq!(L::ZERO.powf(-0.0), L::ONE);
    assert_eq!(L::INFINITY.powf(0.0), L::ONE);
    assert_eq!(L::from_value(3.0).powf(0.0), L::ONE);
    assert_eq!(bits(L::ZERO.powf(0.0)), 0);
}

#[test]
fn powf_one_to_anything_is_one() {
    assert_eq!(L::ONE.powf(INF), L::ONE);
    assert_eq!(L::ONE.powf(-INF), L::ONE);
    assert_eq!(bits(L::ONE.powf(-3.0)), 0, "no -0 from 1^-3");
}

#[test]
fn powf_zero_and_infinity() {
    assert_eq!(L::ZERO.powf(2.0), L::ZERO);
    assert_eq!(L::ZERO.powf(-2.0), L::INFINITY);
    assert_eq!(L::INFINITY.powf(0.5), L::INFINITY);
    assert_eq!(L::INFINITY.powf(-0.5), L::ZERO);
    assert_eq!(L::from_value(0.5).powf(INF), L::ZERO);
    assert_eq!(L::from_value(2.0).powf(INF), L::INFINITY);
    assert_eq!(L::from_value(2.0).powf(-INF), L::ZERO);
}

#[test]
#[should_panic(expected = "NaN")]
fn powf_nan_exponent_panics() {
    let _ = L::from_value(2.0).powf(f64::NAN);
}

#[test]
fn powi_cases() {
    let x = L::from_ln(-2.5);
    assert_eq!(x.powi(3).ln(), -7.5);
    assert_eq!(x.powi(0), L::ONE);
    assert_eq!(L::ZERO.powi(0), L::ONE);
    assert_eq!(L::ZERO.powi(-1), L::INFINITY);
    assert_eq!(L::INFINITY.powi(-1), L::ZERO);
    assert_eq!(bits(L::ONE.powi(-7)), 0);
    assert_eq!(L::from_ln(1.0).powi(i32::MIN).ln(), i32::MIN as f64);
}

#[test]
fn sqrt_and_recip() {
    assert_eq!(L::from_ln(-7.0).sqrt().ln(), -3.5);
    assert_eq!(L::ZERO.sqrt(), L::ZERO);
    assert_eq!(L::INFINITY.sqrt(), L::INFINITY);
    assert_eq!(L::from_ln(-7.0).recip().ln(), 7.0);
    assert_eq!(L::ZERO.recip(), L::INFINITY);
    assert_eq!(L::INFINITY.recip(), L::ZERO);
    assert_eq!(bits(L::ONE.recip()), 0, "ONE.recip() stores +0");
}

// ---- ordering --------------------------------------------------------------------------

#[test]
fn ordering_follows_value() {
    let vals = [
        L::ZERO,
        L::from_ln(-1e300),
        L::from_value(1e-300),
        L::from_value(0.5),
        L::ONE,
        L::from_value(2.0),
        L::from_ln(1e300),
        L::INFINITY,
    ];
    for w in vals.windows(2) {
        assert!(w[0] < w[1], "{:?} < {:?}", w[0], w[1]);
        assert_eq!(w[0].total_cmp(&w[1]), Ordering::Less);
        assert_eq!(w[1].total_cmp(&w[0]), Ordering::Greater);
        assert_eq!(w[0].max(w[1]), w[1]);
        assert_eq!(w[0].min(w[1]), w[0]);
    }
    let mut shuffled = [
        vals[3], vals[7], vals[0], vals[5], vals[1], vals[6], vals[2], vals[4],
    ];
    shuffled.sort_by(L::total_cmp);
    assert_eq!(shuffled, vals);
}

#[test]
fn total_cmp_agrees_with_eq_on_signed_zero() {
    let pos = L::ONE;
    let neg = L::from_ln_unchecked(-0.0);
    assert_eq!(pos, neg);
    assert_eq!(pos.total_cmp(&neg), Ordering::Equal);
}

// ---- formatting ------------------------------------------------------------------------

#[test]
fn debug_shows_stored_log() {
    assert_eq!(format!("{:?}", L::from_ln(-11512.9)), "Log(ln = -11512.9)");
    assert_eq!(format!("{:?}", L::ZERO), "Log(ln = -inf)");
    assert_eq!(format!("{:?}", L::ONE), "Log(ln = 0.0)");
    assert_eq!(format!("{:.2?}", L::from_ln(-1.23456)), "Log(ln = -1.23)");
}

#[test]
fn domain_error_display() {
    assert!(DomainError::Negative.to_string().contains("negative"));
    assert!(DomainError::NaN.to_string().contains("NaN"));
    let _: &dyn core::error::Error = &DomainError::NaN;
}
