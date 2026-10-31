//! Robustness: arbitrary inputs never panic (outside documented panics) and never produce
//! NaN. Inputs are drawn from every bit pattern, not just "nice" ranges.

use mirifici::{Log, SignedLog, slice};
use proptest::prelude::*;

/// Any non-NaN f64, including ±0, ±∞, subnormals and extreme magnitudes.
fn any_ln() -> impl Strategy<Value = f64> {
    prop_oneof![
        any::<u64>().prop_map(f64::from_bits),
        Just(0.0),
        Just(-0.0),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        Just(f64::MAX),
        Just(-f64::MAX),
        Just(f64::MIN_POSITIVE),
        Just(5e-324),
        -800.0..800.0f64,
    ]
    .prop_filter("not NaN", |x| !x.is_nan())
}

fn any_log() -> impl Strategy<Value = Log> {
    any_ln().prop_map(Log::from_ln)
}

fn any_log32() -> impl Strategy<Value = Log<f32>> {
    prop_oneof![
        any::<u32>().prop_map(f32::from_bits),
        Just(f32::INFINITY),
        Just(f32::NEG_INFINITY),
        -120.0..120.0f32,
    ]
    .prop_filter("not NaN", |x| !x.is_nan())
    .prop_map(Log::from_ln)
}

/// Strings that look like numbers, half-numbers, or junk.
fn numberish() -> impl Strategy<Value = String> {
    prop_oneof![
        "[+-]?[0-9]{0,30}(\\.[0-9]{0,30})?([eE][+-]?[0-9]{0,25})?",
        "[+-]?(inf|INF|infinity|Infinity|nan|NaN)",
        "[0-9eE.+-]{0,12}",
        any::<String>(),
    ]
}

fn no_nan(x: Log) -> bool {
    !x.ln().is_nan()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn parse_never_panics(s in numberish()) {
        if let Ok(x) = s.parse::<Log>() {
            prop_assert!(no_nan(x));
        }
        if let Ok(x) = s.parse::<Log<f32>>() {
            prop_assert!(!x.ln().is_nan());
        }
        if let Ok(x) = s.parse::<SignedLog>() {
            prop_assert!(!x.magnitude().ln().is_nan());
        }
    }

    #[test]
    fn parse_agrees_with_f64_in_range(s in "[0-9]{1,20}(\\.[0-9]{0,20})?([eE][+-]?[0-9]{1,2})?") {
        // Strings f64 can parse to a normal value must give exactly from_value of it.
        let v: f64 = s.parse().unwrap();
        if v == 0.0 || (v >= f64::MIN_POSITIVE && v.is_finite()) {
            prop_assert_eq!(s.parse::<Log>().unwrap(), Log::from_value(v));
        }
    }

    #[test]
    fn formatting_never_panics_and_round_trips(x in any_log(), p in 0usize..30, w in 0usize..40) {
        let shortest = x.to_string();
        let back: Log = shortest.parse().unwrap();
        if x.try_to_value().is_some_and(|v| v == 0.0 || v >= f64::MIN_POSITIVE) {
            // printed through the float: one rounding of the value
            let tol = 2.0 * f64::EPSILON * x.ln().abs().max(1.0);
            prop_assert!(back == x || (back.ln() - x.ln()).abs() <= tol, "{shortest}");
        } else if x.ln().abs() < 2e16 {
            prop_assert_eq!(back, x, "{}", shortest);
        }
        for s in [
            format!("{x:.p$}"), format!("{x:e}"), format!("{x:.p$E}"),
            format!("{x:>w$.p$}"), format!("{x:<+w$}"), format!("{x:^w$e}"), format!("{x:0w$}"),
            format!("{x:?}"), format!("{}", x.display_base(2.0)), format!("{:.3}", x.display_base(10.0)),
        ] {
            prop_assert!(!s.is_empty());
        }
        // Padded output has at least the requested width.
        let padded = format!("{x:>w$}");
        prop_assert!(padded.chars().count() >= w);
    }

    #[test]
    fn f32_formatting_round_trips(x in any_log32()) {
        let s = x.to_string();
        let back: Log<f32> = s.parse().unwrap();
        if x.try_to_value().is_some_and(|v| v == 0.0 || v >= f32::MIN_POSITIVE) {
            let tol = 2.0 * f32::EPSILON * x.ln().abs().max(1.0);
            prop_assert!(back == x || (back.ln() - x.ln()).abs() <= tol, "{s}");
        } else {
            prop_assert_eq!(back, x, "{}", s);
        }
    }

    #[test]
    fn arithmetic_never_nan(a in any_log(), b in any_log(), p in any_ln(), n in any::<i32>()) {
        prop_assert!(no_nan(a + b));
        for r in [a.checked_mul(b), a.checked_div(b), a.checked_sub(b)].into_iter().flatten() {
            prop_assert!(no_nan(r));
        }
        prop_assert!(no_nan(a.powf(p)));
        prop_assert!(no_nan(a.powi(n)));
        prop_assert!(no_nan(a.sqrt()));
        prop_assert!(no_nan(a.recip()));
        if !(a == Log::INFINITY && b == Log::INFINITY) {
            prop_assert!(no_nan(a.abs_diff(b)));
        }
        prop_assert!(!a.log2().is_nan() && !a.log10().is_nan() && !a.to_value().is_nan());
    }

    #[test]
    fn signed_arithmetic_never_nan(
        a in any_log(), b in any_log(), sa in any::<bool>(), sb in any::<bool>()
    ) {
        let (x, y) = (SignedLog::new(sa, a), SignedLog::new(sb, b));
        for r in [x.checked_add(y), x.checked_sub(y), x.checked_mul(y), x.checked_div(y)]
            .into_iter()
            .flatten()
        {
            prop_assert!(!r.magnitude().ln().is_nan());
            prop_assert!(!(r.is_negative() && r == SignedLog::ZERO));
        }
        let shown = format!("{x:.4}");
        prop_assert!(!shown.is_empty());
    }

    #[test]
    fn aggregation_never_nan(xs in proptest::collection::vec(any_log(), 0..50)) {
        let has_zero = xs.contains(&Log::ZERO);
        let has_inf = xs.contains(&Log::INFINITY);
        prop_assert!(no_nan(xs.iter().sum()));
        prop_assert!(no_nan(Log::sum_two_pass(&xs)));
        if !(has_zero && has_inf) {
            prop_assert!(no_nan(xs.iter().product()));
            prop_assert!(no_nan(Log::product_compensated(xs.iter().copied())));
        }
        let raw: Vec<f64> = xs.iter().map(|x| x.ln()).collect();
        prop_assert!(!slice::logsumexp(&raw).is_nan());
        let mut ys = xs.clone();
        if let Some(total) = slice::normalize_in_place(&mut ys) {
            prop_assert!(no_nan(total));
            prop_assert!(ys.iter().all(|&y| no_nan(y) && y <= Log::ONE));
        }
        let mut zs = raw.clone();
        if slice::softmax_in_place(&mut zs).is_some() {
            prop_assert!(zs.iter().all(|z| (0.0..=1.0).contains(z)));
            // Each output carries an error of about ulp(lᵢ), which grows with |lᵢ|.
            let scale = raw.iter().filter(|l| l.is_finite()).fold(1.0f64, |m, l| m.max(l.abs()));
            let tol = 1e-12 + 4.0 * f64::EPSILON * scale * zs.len() as f64;
            prop_assert!((zs.iter().sum::<f64>() - 1.0).abs() <= tol);
        }
        if let Some(m) = slice::mean(&xs) {
            prop_assert!(no_nan(m));
        }
    }
}
