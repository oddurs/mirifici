//! Algebraic properties, checked with proptest.

use mirifici::Log;
use proptest::prelude::*;

type L = Log<f64>;
const EPS: f64 = f64::EPSILON;

/// Finite stored logs across many scales, plus the two infinities now and then.
fn any_log() -> impl Strategy<Value = L> {
    prop_oneof![
        8 => -1000.0..1000.0f64,
        4 => -1e6..1e6f64,
        2 => -1e300..1e300f64,
        1 => Just(f64::NEG_INFINITY),
        1 => Just(f64::INFINITY),
    ]
    .prop_map(L::from_ln)
}

fn finite_log() -> impl Strategy<Value = L> {
    prop_oneof![-1000.0..1000.0f64, -1e6..1e6f64, -1e300..1e300f64].prop_map(L::from_ln)
}

/// |la − lb| ≤ k·ε·max(1, |la|, |lb|), or exactly equal (covers infinities).
fn close(a: L, b: L, k: f64) -> bool {
    a == b || (a.ln() - b.ln()).abs() <= k * EPS * a.ln().abs().max(b.ln().abs()).max(1.0)
}

proptest! {
    #[test]
    fn add_commutes(a in any_log(), b in any_log()) {
        prop_assert_eq!(a + b, b + a);
    }

    #[test]
    fn mul_commutes(a in any_log(), b in any_log()) {
        prop_assert_eq!(a.checked_mul(b), b.checked_mul(a));
    }

    #[test]
    fn mul_then_div_round_trips(a in finite_log(), b in finite_log()) {
        // la + lb may overflow only beyond 1e308; skip those.
        prop_assume!((a.ln() + b.ln()).is_finite());
        let back = (a * b) / b;
        // Two roundings of magnitude up to max(|la|, |lb|, |la + lb|).
        let scale = a.ln().abs().max(b.ln().abs()).max((a * b).ln().abs()).max(1.0);
        prop_assert!((back.ln() - a.ln()).abs() <= 1.0 * EPS * scale, "{a:?} {b:?} {back:?}");
    }

    #[test]
    fn sum_is_at_least_the_max(a in any_log(), b in any_log()) {
        let s = a + b;
        prop_assert!(s >= a.max(b));
    }

    #[test]
    fn sub_inverts_add(a in finite_log(), b in finite_log()) {
        let s = a + b;
        let back = s.checked_sub(b).expect("a + b >= b");
        // In value space the error is about ε·max(1, |ln s|)·s, so relative to a it is
        // amplified by s/a = exp(ls − la). When that factor makes the tolerance ≥ 1, `a` was
        // absorbed by rounding in a + b and cannot be recovered; skip those cases.
        let amp = (s.ln() - a.ln()).exp();
        let tol = 4.0 * EPS * s.ln().abs().max(1.0) * amp;
        prop_assume!(tol < 1e-3);
        prop_assert!((back.ln() - a.ln()).abs() <= tol, "{a:?} {b:?} {back:?} tol {tol:e}");
    }

    #[test]
    fn sum_is_order_insensitive(
        xs in proptest::collection::vec(-50.0..50.0f64, 1..200),
        seed in any::<u64>(),
        offset in -1e4..1e4f64,
    ) {
        let xs: Vec<L> = xs.into_iter().map(|l| L::from_ln(l + offset)).collect();
        let mut ys = xs.clone();
        // Fisher-Yates with a tiny LCG so the permutation is reproducible.
        let mut state = seed | 1;
        for i in (1..ys.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ys.swap(i, (state >> 33) as usize % (i + 1));
        }
        let n = xs.len() as f64;
        let s1: L = xs.iter().sum();
        let s2: L = ys.iter().sum();
        let t1 = L::sum_two_pass(&xs);
        let t2 = L::sum_two_pass(&ys);
        let k = 2.0 * (n + 2.0);
        prop_assert!(close(s1, s2, k), "{s1:?} {s2:?}");
        prop_assert!(close(t1, t2, k), "{t1:?} {t2:?}");
        prop_assert!(close(s1, t1, k), "{s1:?} {t1:?}");
    }

    #[test]
    fn checked_sub_never_nan(a in any_log(), b in any_log()) {
        if let Some(d) = a.checked_sub(b) {
            prop_assert!(!d.ln().is_nan());
            prop_assert!(d <= a);
        } else {
            prop_assert!(b > a || (a == L::INFINITY && b == L::INFINITY));
        }
    }

    #[test]
    fn add_never_nan(a in any_log(), b in any_log()) {
        prop_assert!(!(a + b).ln().is_nan());
    }
}
