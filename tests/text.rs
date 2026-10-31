//! Formatting and parsing, inside and beyond the float's range.

use mirifici::{Log, SignedLog};
use proptest::prelude::*;

type L = Log<f64>;

// ---- Display inside the float range matches the float ----------------------------------

#[test]
fn display_matches_float_in_range() {
    for v in [
        0.0,
        1.0,
        0.25,
        3.0,
        1e-300,
        2.5e300,
        123456.789,
        f64::MIN_POSITIVE,
        f64::MAX,
    ] {
        let x = L::from_value(v);
        let w = x.to_value();
        assert_eq!(format!("{x}"), format!("{w}"));
        assert_eq!(format!("{x:.3}"), format!("{w:.3}"));
        assert_eq!(format!("{x:e}"), format!("{w:e}"));
        assert_eq!(format!("{x:.2E}"), format!("{w:.2E}"));
        assert_eq!(format!("{x:>30}"), format!("{w:>30}"));
    }
    assert_eq!(L::ZERO.to_string(), "0");
    assert_eq!(L::INFINITY.to_string(), "inf");
    assert_eq!(format!("{:e}", L::ZERO), "0e0");
}

// ---- Display beyond the float range ------------------------------------------------------

#[test]
fn display_beyond_range() {
    assert_eq!(L::from_log10(-5000.0).to_string(), "1e-5000");
    assert_eq!(L::from_log10(90000.0).to_string(), "1e90000");
    assert_eq!(format!("{:.3}", L::from_log10(-5000.0)), "1.000e-5000");
    assert_eq!(format!("{:E}", L::from_log10(-5000.0)), "1E-5000");
    assert_eq!(
        format!("{:.1e}", L::from_log10(-5000.0) * L::from_value(2.5)),
        "2.5e-5000"
    );
    // Subnormal f64 values are printed from the log, not the lossy float, with as many
    // digits as the stored log resolves (ulp(744.4) ≈ 1.1e-13).
    let sub = L::from_value(5e-324);
    assert_eq!(sub.to_string(), "4.9406564584127e-324");
    assert_eq!(sub.to_string().parse::<L>().unwrap(), sub);
}

#[test]
fn display_beyond_range_honours_flags() {
    let x = L::from_log10(-5000.0);
    assert_eq!(format!("{x:>10}"), "   1e-5000");
    assert_eq!(format!("{x:<10}|"), "1e-5000   |");
    assert_eq!(format!("{x:*^11}"), "**1e-5000**");
    assert_eq!(format!("{x:+}"), "+1e-5000");
    assert_eq!(format!("{x:010}"), "0001e-5000");
}

#[test]
fn display_rounding_carries_into_exponent() {
    // 9.9999999990e-5001 rounded to 1 decimal is 1.0e-5000
    let x = L::from_ln(-5000.0 * core::f64::consts::LN_10 - 1e-9);
    assert_eq!(format!("{x:.1}"), "1.0e-5000");
    assert_eq!(format!("{x:.0}"), "1e-5000");
}

#[test]
fn display_extreme_logs() {
    // Beyond |log10 x| = 2^53 not every exponent is representable: print a power of
    // ten, which parses back to within an ulp of the log.
    let x = L::from_ln(1e300);
    let s = x.to_string();
    assert!(s.starts_with("1e434294481903251"), "{s}");
    let back = s.parse::<L>().unwrap();
    assert!((back.ln() - x.ln()).abs() <= 2.0 * f64::EPSILON * 1e300);
    let y = L::from_ln(-f64::MAX);
    assert_eq!(y.to_string().parse::<L>().unwrap(), y);
}

#[test]
fn signed_display() {
    let x = SignedLog::new(true, L::from_log10(-5000.0));
    assert_eq!(x.to_string(), "-1e-5000");
    assert_eq!(format!("{x:>10}"), "  -1e-5000");
    assert_eq!(SignedLog::from_value(-2.5_f64).to_string(), "-2.5");
    assert_eq!(format!("{:e}", SignedLog::from_value(-2.5_f64)), "-2.5e0");
    assert_eq!(
        "-2.5e-90210".parse::<SignedLog>().unwrap().to_string(),
        "-2.5e-90210"
    );
    assert_eq!(
        "+3".parse::<SignedLog>().unwrap(),
        SignedLog::from_value(3.0)
    );
}

#[test]
fn log_display_adapter() {
    let x = L::from_log2(-1024.5);
    assert_eq!(x.display_base(2.0).to_string(), "2^-1024.5");
    assert_eq!(format!("{:.1}", x.display_base(10.0)), "10^-308.4");
    assert_eq!(L::ZERO.display_base(2.0).to_string(), "0");
    assert_eq!(L::INFINITY.display_base(2.0).to_string(), "inf");
    assert_eq!(L::ONE.display_base(10.0).to_string(), "10^0");
}

#[test]
#[should_panic(expected = "base")]
fn log_display_rejects_bad_base() {
    let _ = L::ONE.display_base(1.0);
}

// ---- parsing ---------------------------------------------------------------------------

#[test]
fn parse_in_range_matches_from_value() {
    for s in [
        "0.1", "0.5", "3", "1e-300", "2.5e300", ".5", "5.", "+7", "1E3", "1e+3", "0.000123",
    ] {
        let v: f64 = s.parse().unwrap();
        assert_eq!(s.parse::<L>().unwrap(), L::from_value(v), "{s}");
    }
}

#[test]
fn parse_special_values() {
    assert_eq!("0".parse::<L>().unwrap(), L::ZERO);
    assert_eq!("-0".parse::<L>().unwrap(), L::ZERO);
    assert_eq!("0.000e99999999".parse::<L>().unwrap(), L::ZERO);
    assert_eq!("inf".parse::<L>().unwrap(), L::INFINITY);
    assert_eq!("+Infinity".parse::<L>().unwrap(), L::INFINITY);
    assert_eq!("INF".parse::<L>().unwrap(), L::INFINITY);
}

#[test]
fn parse_errors() {
    for s in [
        "", "-1", "-inf", "nan", "NaN", "abc", ".", "e5", "1e", "1e+", "1.2.3", " 1", "1 ", "--1",
        "1e5.5", "0x10",
    ] {
        assert!(s.parse::<L>().is_err(), "{s:?} should fail");
    }
    let e = "-1".parse::<L>().unwrap_err();
    assert!(e.to_string().contains("negative"));
    assert!("nan".parse::<L>().unwrap_err().to_string().contains("NaN"));
    assert!("-nan".parse::<SignedLog>().is_err());
}

#[test]
fn parse_beyond_range() {
    let x: L = "2.5e-90210".parse().unwrap();
    let expected = -90210.0 * core::f64::consts::LN_10 + 2.5f64.ln();
    assert!((x.ln() - expected).abs() <= 2.0 * f64::EPSILON * expected.abs());
    assert_eq!(x.to_string(), "2.5e-90210");
    // the mantissa may be written any way
    assert_eq!("0.0025e-90207".parse::<L>().unwrap(), x);
    assert_eq!("25000e-90214".parse::<L>().unwrap(), x);
}

#[test]
fn parse_enormous_exponents() {
    let x: L = "1e1000000000000000000000".parse().unwrap();
    assert!((x.ln() / (1e21 * core::f64::consts::LN_10) - 1.0).abs() < 1e-15);
    let huge = format!("1e{}", "9".repeat(400));
    assert_eq!(huge.parse::<L>().unwrap(), L::INFINITY);
    let tiny = format!("1e-{}", "9".repeat(400));
    assert_eq!(tiny.parse::<L>().unwrap(), L::ZERO);
    // long mantissas are fine too
    let long = format!("1.{}e-5000", "0".repeat(500));
    assert_eq!(long.parse::<L>().unwrap(), L::from_log10(-5000.0));
}

#[test]
fn f32_text() {
    let x: Log<f32> = "1e-50".parse().unwrap();
    assert_eq!(x.to_string(), "1e-50");
    assert_eq!(x.try_to_value(), None);
    assert_eq!(Log::<f32>::from_value(0.25).to_string(), "0.25");
    assert_eq!(
        "1e-30".parse::<Log<f32>>().unwrap(),
        Log::from_value(1e-30f32)
    );
}

// ---- fixtures ----------------------------------------------------------------------------

fn rows(csv: &str) -> impl Iterator<Item = Vec<&str>> {
    csv.lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(|l| l.split(',').collect())
}

fn check_parse<F>(csv: &str, eps: f64, bound: f64)
where
    F: mirifici::LogFloat + Into<f64>,
    Log<F>: core::str::FromStr<Err: core::fmt::Debug>,
{
    let mut worst = 0.0f64;
    for r in rows(csv) {
        let got: f64 = r[0].parse::<Log<F>>().unwrap().ln().into();
        let (hi, lo): (f64, f64) = (r[1].parse().unwrap(), r[2].parse().unwrap());
        let e = if hi.is_infinite() {
            if got == hi { 0.0 } else { f64::INFINITY }
        } else {
            (((got - hi) - lo) / (eps * hi.abs().max(1.0))).abs()
        };
        assert!(e <= bound, "parse {}: error {e} > {bound}", r[0]);
        worst = worst.max(e);
    }
    println!("parse: max E = {worst:.3}");
}

#[test]
fn parse_fixtures() {
    check_parse::<f64>(include_str!("fixtures/parse_f64.csv"), f64::EPSILON, 1.0);
    check_parse::<f32>(
        include_str!("fixtures/parse_f32.csv"),
        f64::from(f32::EPSILON),
        1.0,
    );
}

fn check_display<F>(csv: &str)
where
    F: mirifici::LogFloat + core::str::FromStr<Err: core::fmt::Debug>,
{
    let (mut n, mut mismatches) = (0, 0);
    for r in rows(csv) {
        let x = Log::<F>::from_ln(r[0].parse().unwrap());
        let p: usize = r[1].parse().unwrap();
        let got = format!("{x:.p$e}");
        n += 1;
        if got != r[2] {
            mismatches += 1;
            println!("display ln={}: got {got}, expected {}", r[0], r[2]);
        }
    }
    println!("display: {mismatches} of {n} differ");
    assert_eq!(mismatches, 0);
}

#[test]
fn display_fixtures() {
    check_display::<f64>(include_str!("fixtures/display_f64.csv"));
    check_display::<f32>(include_str!("fixtures/display_f32.csv"));
}

// ---- round trips -------------------------------------------------------------------------

proptest! {
    #[test]
    fn round_trip_beyond_range_is_exact(ln in prop_oneof![-2e16..-745.2f64, 709.8..2e16f64, -2e16..-1e15f64, 1e15..2e16f64]) {
        let x = L::from_ln(ln);
        let s = x.to_string();
        prop_assert_eq!(s.parse::<L>().unwrap(), x, "{}", s);
    }

    #[test]
    fn round_trip_in_range_is_close(v in 1e-300..1e300f64) {
        let x = L::from_value(v);
        let back: L = x.to_string().parse().unwrap();
        // in range, Display goes through the float: one rounding of the value
        prop_assert!((back.ln() - x.ln()).abs() <= 2.0 * f64::EPSILON * x.ln().abs().max(1.0));
    }

    #[test]
    fn round_trip_f32_beyond_range_is_exact(ln in prop_oneof![-1e6..-103.3f32, 88.8..1e6f32]) {
        let x = Log::<f32>::from_ln(ln);
        let s = x.to_string();
        prop_assert_eq!(s.parse::<Log<f32>>().unwrap(), x, "{}", s);
    }

    #[test]
    fn round_trip_with_precision(ln in -1e6..-745.2f64, p in 0usize..14) {
        let x = L::from_ln(ln);
        let back: L = format!("{x:.p$}").parse().unwrap();
        // p decimals of mantissa: relative error ≤ 5·10^-(p+1), plus the ulp of ln
        let tol = 5.0 * 10f64.powi(-(p as i32) - 1) + 2.0 * f64::EPSILON * ln.abs();
        prop_assert!((back.ln() - x.ln()).abs() <= tol);
    }
}

// ---- regressions from review ----------------------------------------------------------------

#[test]
fn round_trip_between_2_pow_52_and_2_pow_53() {
    let x = L::from_ln(-1.5645060302681268e16);
    assert_eq!(x.to_string().parse::<L>().unwrap(), x);
}

#[test]
fn parse_matches_from_value_at_range_edges() {
    for s in [
        "1.1457388879924349e308",
        "1.7976931348623157e308",
        "2.2250738585072014e-308",
    ] {
        let v: f64 = s.parse().unwrap();
        assert_eq!(s.parse::<L>().unwrap(), L::from_value(v), "{s}");
    }
    let v: f32 = "6.643192e-38".parse().unwrap();
    assert_eq!(
        "6.643192e-38".parse::<Log<f32>>().unwrap(),
        Log::from_value(v)
    );
    // Just past the edge the log-space path keeps what the float would lose.
    let sub: L = "1e-310".parse().unwrap();
    assert!((sub.log10() + 310.0).abs() < 1e-12);
}

#[test]
fn negative_values_are_rejected_even_when_they_underflow() {
    let tiny = format!("-1e-{}", "9".repeat(320));
    assert!(tiny.parse::<L>().is_err());
    assert!("-0e-99999".parse::<L>().is_ok());
    assert!("-0.000".parse::<L>().is_ok());
    let e = "+".parse::<L>().unwrap_err().to_string();
    assert!(e.contains("invalid"), "{e}");
    assert!("-".parse::<SignedLog>().is_err());
}

#[test]
fn log_display_uses_accurate_conversions() {
    let x = L::from_log10(90000.0);
    assert_eq!(x.display_base(10.0).to_string(), "10^90000");
    assert_eq!(
        L::from_log2(-1074.0).display_base(2.0).to_string(),
        "2^-1074"
    );
}

#[test]
fn no_negative_zero_from_arithmetic() {
    assert_eq!(L::from_ln(1e-300).powf(-1e-300).ln().to_bits(), 0);
    assert_eq!(L::from_ln(-5e-324).sqrt().ln().to_bits(), 0);
}

#[test]
fn very_long_precision_beyond_range() {
    // Mantissas too long for the internal buffer are written unpadded but in full.
    let x = L::from_log10(-5000.0);
    let s = format!("{x:.150}");
    // The stored log of 1e-5000 is 4.4e-13 (relative) below it, which 150 digits show.
    assert!(
        s.starts_with("9.99999999999") && s.ends_with("e-5001"),
        "{s}"
    );
    assert_eq!(s.len(), "9.".len() + 150 + "e-5001".len());
    let neg = SignedLog::new(true, x);
    assert!(format!("{neg:.150}").starts_with("-9.999"));
}

#[test]
fn all_formatting_traits_on_signed() {
    let x = SignedLog::new(true, L::from_log10(-5000.0) * L::from_value(2.5));
    assert_eq!(format!("{x:E}"), "-2.5E-5000");
    assert_eq!(format!("{x:e}"), "-2.5e-5000");
    assert_eq!(format!("{:E}", SignedLog::from_value(-0.5_f64)), "-5E-1");
}

#[test]
fn log_display_in_base_e_and_others() {
    let x = L::from_ln(-710.0);
    assert_eq!(x.display_base(core::f64::consts::E).to_string(), "e^-710");
    assert_eq!(format!("{:.2}", x.display_base(3.0)), "3^-646.27");
}

#[test]
fn parse_error_messages() {
    for (s, needle) in [
        ("", "empty"),
        ("abc", "invalid"),
        ("-2", "negative"),
        ("nan", "NaN"),
    ] {
        let e = s.parse::<L>().unwrap_err();
        assert!(e.to_string().contains(needle), "{s:?}: {e}");
        let _: &dyn core::error::Error = &e;
    }
}
