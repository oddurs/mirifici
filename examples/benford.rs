//! Benford's law on numbers far beyond the range of `f64`.
//!
//! In many data sets that span several orders of magnitude, the leading digit d appears with
//! frequency log10(1 + 1/d): about 30 % ones and under 5 % nines. Logarithms explain why:
//! x has leading digit d exactly when the fractional part of log10(x) lies in
//! [log10 d, log10(d + 1)), an interval of width log10(1 + 1/d). Data that is spread evenly
//! across many orders of magnitude has its log10 spread evenly modulo 1, so each digit's
//! share is the width of its interval.
//!
//! The data sets here are the powers 2¹ … 2¹⁰⁰⁰⁰ and the factorials 1! … 3000!, almost all
//! of which overflow `f64` (2¹⁰²⁴ and 171! already do). `Log` holds them, and its `log10`
//! gives the leading digit directly.
//!
//! Run with `cargo run --example benford`.

use mirifici::Log;

/// The leading decimal digit of x > 0, from the fractional part of log10(x).
fn leading_digit(x: Log) -> usize {
    let l = x.log10();
    let frac = l - l.floor();
    (10f64.powf(frac) as usize).clamp(1, 9)
}

fn tally(values: impl Iterator<Item = Log>) -> ([usize; 10], usize) {
    let mut counts = [0; 10];
    let mut n = 0;
    for x in values {
        counts[leading_digit(x)] += 1;
        n += 1;
    }
    (counts, n)
}

fn main() {
    let two = Log::from_value(2.0_f64);
    let powers = (1..=10_000).map(|k| two.powi(k));
    let factorials = (1..=3000u32).scan(Log::ONE, |acc, k| {
        *acc *= Log::from_value(f64::from(k));
        Some(*acc)
    });

    println!("2^10000 = {}", two.powi(10_000));
    // Display prints as many digits as the stored log resolves (about 12 here).
    let factorial_3000 =
        Log::product_compensated((1..=3000u32).map(|k| Log::from_value(f64::from(k))));
    println!("3000!   = {factorial_3000}");
    println!();

    let (p, np) = tally(powers);
    let (f, nf) = tally(factorials);
    println!("digit   Benford   2^1..2^10000   1!..3000!");
    let mut worst = 0.0f64;
    for d in 1..=9 {
        let benford = (1.0 + 1.0 / d as f64).log10();
        let fp = p[d] as f64 / np as f64;
        let ff = f[d] as f64 / nf as f64;
        worst = worst.max((fp - benford).abs()).max((ff - benford).abs());
        println!(
            "  {d}     {:6.2}%      {:6.2}%       {:6.2}%",
            100.0 * benford,
            100.0 * fp,
            100.0 * ff
        );
    }
    println!();
    println!(
        "largest deviation from Benford: {:.2} percentage points",
        100.0 * worst
    );
    assert!(worst < 0.02);
}
