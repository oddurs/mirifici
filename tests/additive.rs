//! Edge cases of addition, subtraction and aggregation.
#![allow(clippy::op_ref)]

use mirifici::{Log, log1mexp};

type L = Log<f64>;
const INF: f64 = f64::INFINITY;

fn close(a: L, b: L, tol: f64) -> bool {
    a == b || (a.ln() - b.ln()).abs() <= tol
}

// ---- addition --------------------------------------------------------------------------

#[test]
fn add_identity_and_infinities() {
    let x = L::from_value(3.25);
    assert_eq!(x + L::ZERO, x);
    assert_eq!(L::ZERO + x, x);
    assert_eq!(L::ZERO + L::ZERO, L::ZERO);
    assert_eq!(x + L::INFINITY, L::INFINITY);
    assert_eq!(L::INFINITY + x, L::INFINITY);
    assert_eq!(L::INFINITY + L::INFINITY, L::INFINITY);
    assert_eq!(L::INFINITY + L::ZERO, L::INFINITY);
}

#[test]
fn add_equal_terms_doubles() {
    let x = L::from_ln(-12345.5);
    assert_eq!((x + x).ln(), -12345.5 + core::f64::consts::LN_2);
}

#[test]
fn add_matches_float_in_range() {
    let a = L::from_value(0.3);
    let b = L::from_value(0.6);
    assert!(((a + b).to_value() - 0.9).abs() < 1e-15);
}

#[test]
fn add_far_apart_returns_larger() {
    let big = L::from_ln(0.0);
    let small = L::from_ln(-1000.0);
    assert_eq!(big + small, big);
}

#[test]
fn add_beyond_float_range() {
    // 1e-5000 + 1e-5000 = 2e-5000
    let x = L::from_log10(-5000.0);
    let two_x = x + x;
    assert!(close(two_x, x * L::from_value(2.0), 1e-12));
}

#[test]
fn add_assign_and_reference_forms() {
    let a = L::from_value(1.0);
    let b = L::from_value(2.0);
    let mut c = a;
    c += b;
    c += &b;
    assert!((c.to_value() - 5.0).abs() < 1e-15);
    assert_eq!(&a + &b, a + b);
}

// ---- log1mexp --------------------------------------------------------------------------

#[test]
fn log1mexp_edges() {
    assert_eq!(log1mexp(0.0f64), -INF);
    assert_eq!(log1mexp(INF), 0.0);
    // both branches, either side of ln 2
    let ln2 = core::f64::consts::LN_2;
    assert!((log1mexp(ln2) - 0.5f64.ln()).abs() < 1e-16);
    assert!((log1mexp(1.0f64) - (1.0 - (-1.0f64).exp()).ln()).abs() < 1e-15);
    // tiny d: ln(1 - e^-d) ≈ ln d
    assert!((log1mexp(1e-300f64) - 1e-300f64.ln()).abs() < 1e-13);
    // huge d: ln(1 - e^-d) ≈ -e^-d
    assert_eq!(log1mexp(50.0f64), -(-50.0f64).exp());
}

#[test]
#[should_panic(expected = ">= 0")]
fn log1mexp_negative_panics() {
    let _ = log1mexp(-1.0f64);
}

#[test]
#[should_panic(expected = ">= 0")]
fn log1mexp_nan_panics() {
    let _ = log1mexp(f64::NAN);
}

// ---- checked_sub and abs_diff ----------------------------------------------------------

#[test]
fn checked_sub_basic() {
    let a = L::from_value(5.0);
    let b = L::from_value(3.0);
    assert!(((a.checked_sub(b).unwrap()).to_value() - 2.0).abs() < 1e-15);
    assert_eq!(b.checked_sub(a), None);
}

#[test]
fn checked_sub_equal_is_exact_zero() {
    let a = L::from_ln(-777.0);
    assert_eq!(a.checked_sub(a), Some(L::ZERO));
    assert_eq!(L::ZERO.checked_sub(L::ZERO), Some(L::ZERO));
}

#[test]
fn checked_sub_zero_is_identity() {
    let a = L::from_ln(-777.0);
    assert_eq!(a.checked_sub(L::ZERO), Some(a));
    assert_eq!(L::ZERO.checked_sub(a), None);
}

#[test]
fn checked_sub_infinities() {
    let a = L::from_value(2.0);
    assert_eq!(L::INFINITY.checked_sub(a), Some(L::INFINITY));
    assert_eq!(L::INFINITY.checked_sub(L::ZERO), Some(L::INFINITY));
    assert_eq!(L::INFINITY.checked_sub(L::INFINITY), None);
    assert_eq!(a.checked_sub(L::INFINITY), None);
}

#[test]
fn checked_sub_log_difference_overflows() {
    // la - lb overflows to +inf; the answer is a itself.
    let a = L::from_ln(1e308);
    let b = L::from_ln(-1e308);
    assert_eq!(a.checked_sub(b), Some(a));
}

#[test]
fn checked_sub_near_cancellation() {
    // (1 + 1e-10) - 1 = 1e-10 to within the input rounding
    let a = L::from_ln(1e-10f64.ln_1p());
    let d = a.checked_sub(L::ONE).unwrap();
    assert!((d.to_value() / 1e-10 - 1.0).abs() < 1e-6);
}

#[test]
fn abs_diff_is_symmetric() {
    let a = L::from_value(5.0);
    let b = L::from_value(3.0);
    assert_eq!(a.abs_diff(b), b.abs_diff(a));
    assert_eq!(a.abs_diff(a), L::ZERO);
    assert_eq!(a.abs_diff(L::ZERO), a);
    assert_eq!(L::INFINITY.abs_diff(a), L::INFINITY);
}

#[test]
#[should_panic(expected = "indeterminate")]
fn abs_diff_inf_inf_panics() {
    let _ = L::INFINITY.abs_diff(L::INFINITY);
}

// ---- Sum -------------------------------------------------------------------------------

#[test]
fn sum_empty_and_singletons() {
    let empty: [L; 0] = [];
    assert_eq!(empty.iter().sum::<L>(), L::ZERO);
    assert_eq!(L::sum_two_pass(&empty), L::ZERO);
    let x = L::from_ln(-3.0);
    assert_eq!([x].iter().sum::<L>(), x);
    assert_eq!(L::sum_two_pass(&[x]), x);
}

#[test]
fn sum_with_zeros_and_infinities() {
    let x = L::from_value(2.0);
    let zs = [L::ZERO, L::ZERO];
    assert_eq!(zs.iter().sum::<L>(), L::ZERO);
    assert_eq!(L::sum_two_pass(&zs), L::ZERO);
    let with_zero = [L::ZERO, x, L::ZERO];
    assert_eq!(with_zero.iter().sum::<L>(), x);
    assert_eq!(L::sum_two_pass(&with_zero), x);
    let with_inf = [x, L::INFINITY, x, L::INFINITY];
    assert_eq!(with_inf.into_iter().sum::<L>(), L::INFINITY);
    assert_eq!(L::sum_two_pass(&with_inf), L::INFINITY);
}

#[test]
fn sum_streaming_handles_rising_maximum() {
    let xs: Vec<L> = (0..100)
        .map(|i| L::from_value(f64::from(i) + 1.0))
        .collect();
    let s: L = xs.iter().sum();
    assert!((s.to_value() - 5050.0).abs() < 1e-11);
    assert!((L::sum_two_pass(&xs).to_value() - 5050.0).abs() < 1e-11);
}

#[test]
fn sum_beyond_float_range() {
    // 1000 copies of 1e-5000 = 1e-4997
    let x = L::from_log10(-5000.0);
    let xs = vec![x; 1000];
    let s: L = xs.iter().sum();
    assert!((s.log10() - -4997.0).abs() < 1e-11);
    assert!((L::sum_two_pass(&xs).log10() - -4997.0).abs() < 1e-11);
}

// ---- Product ---------------------------------------------------------------------------

#[test]
fn product_empty_is_one() {
    let empty: [L; 0] = [];
    assert_eq!(empty.iter().product::<L>(), L::ONE);
    assert_eq!(L::product_compensated(empty), L::ONE);
}

#[test]
fn product_with_zero_and_infinity() {
    let x = L::from_value(2.0);
    assert_eq!([x, L::ZERO].iter().product::<L>(), L::ZERO);
    assert_eq!(L::product_compensated([x, L::ZERO]), L::ZERO);
    assert_eq!([x, L::INFINITY].into_iter().product::<L>(), L::INFINITY);
    assert_eq!(L::product_compensated([x, L::INFINITY]), L::INFINITY);
}

#[test]
#[should_panic(expected = "indeterminate")]
fn product_zero_times_infinity_panics() {
    let _: L = [L::ZERO, L::from_value(2.0), L::INFINITY].iter().product();
}

#[test]
#[should_panic(expected = "indeterminate")]
fn product_compensated_zero_times_infinity_panics() {
    let _ = L::product_compensated([L::INFINITY, L::ZERO]);
}

#[test]
fn product_compensated_overflowing_log() {
    let big = L::from_ln(1e308);
    assert_eq!(L::product_compensated([big, big]), L::INFINITY);
    let small = L::from_ln(-1e308);
    assert_eq!(L::product_compensated([small, small]), L::ZERO);
}

/// Deterministic xorshift generator so the test needs no dependencies.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn product_compensated_is_more_accurate_over_a_million_terms() {
    // Log-likelihood terms are multiples of 2^-52 in (-1, 0], so their exact sum is an
    // integer multiple of 2^-52 that an i128 holds exactly. The reference is that exact sum,
    // rounded once to f64.
    const N: usize = 1_000_000;
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut exact: i128 = 0;
    let mut terms = Vec::with_capacity(N);
    for _ in 0..N {
        let k = i128::from(rng.next_u64() >> 12); // 52 random bits
        exact -= k;
        terms.push(L::from_ln(-(k as f64) * 2f64.powi(-52)));
    }
    let reference = exact as f64 * 2f64.powi(-52);
    let ulp = reference.abs() * f64::EPSILON;

    let plain: L = terms.iter().product();
    let compensated = L::product_compensated(terms.iter().copied());
    let err_plain = (plain.ln() - reference).abs();
    let err_comp = (compensated.ln() - reference).abs();
    println!(
        "plain error {:.2} ulp, compensated {:.2} ulp",
        err_plain / ulp,
        err_comp / ulp
    );
    assert!(err_comp <= ulp, "compensated error {err_comp:e} > 1 ulp");
    assert!(
        err_plain > 10.0 * err_comp.max(ulp / 2.0),
        "plain error {err_plain:e}"
    );
}
