//! An exact binomial test whose p-value lies far below the range of `f64`.
//!
//! Toss a fair coin n times. How surprising is seeing at least k heads? The one-sided
//! p-value is
//!
//! ```text
//! P(X ≥ k) = Σ_{j=k}^{n} C(n, j) · 2⁻ⁿ
//! ```
//!
//! For n = 100 000 every piece of that formula breaks a float: C(100 000, 60 000) is about
//! 10²⁹²²⁶, 2⁻¹⁰⁰⁰⁰⁰ is about 10⁻³⁰¹⁰³, and the answer is about 10⁻⁸⁷⁷. Plain `f64` returns
//! NaN (∞ · 0) even for p-values that are themselves ordinary numbers. With `Log`, each term
//! is `Log::binomial(n, j) * Log::from_log2(-n)` and the sum is `Log::sum_two_pass`, all
//! exact to about ten digits. The results are checked against exact big-integer
//! arithmetic (`tests/fixtures/binomial_tail.csv`, from `tools/gen_fixtures.py`).
//!
//! Run with `cargo run --release --example binomial_test`.

use mirifici::Log;

const FIXTURE: &str = include_str!("../tests/fixtures/binomial_tail.csv");

/// P(X ≥ k) for X ~ Binomial(n, ½).
fn upper_tail(n: u64, k: u64) -> Log {
    let half_n = Log::from_log2(-(n as f64));
    let terms: Vec<Log> = (k..=n).map(|j| Log::binomial(n, j) * half_n).collect();
    Log::sum_two_pass(&terms)
}

/// The same formula in plain `f64`, for comparison.
fn upper_tail_f64(n: u64, k: u64) -> f64 {
    let half_n = 0.5f64.powi(n as i32);
    let mut c: f64 = (0..k).map(|i| (n - i) as f64 / (i + 1) as f64).product();
    let mut total = 0.0;
    for j in k..=n {
        total += c * half_n;
        c *= (n - j) as f64 / (j + 1) as f64;
    }
    total
}

fn main() {
    println!(
        "{:>8} {:>8}   {:<22} {:<22} {:<12}",
        "n", "heads", "p-value (Log)", "exact", "plain f64"
    );
    for line in FIXTURE.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        let (n, k): (u64, u64) = (cols[0].parse().unwrap(), cols[1].parse().unwrap());
        let exact_log10: f64 = cols[2].parse().unwrap();

        let p = upper_tail(n, k);
        let exact = Log::from_log10(exact_log10);
        let rel = (p.ln() - exact.ln()).abs();

        println!(
            "{n:>8} {k:>8}   {:<22} {:<22} {:<12e}",
            format!("{p:.10e}"),
            format!("{exact:.10e}"),
            upper_tail_f64(n, k)
        );
        assert!(rel < 1e-8, "relative error {rel:e} for n = {n}, k = {k}");
    }
    println!();
    println!("Every p-value agrees with exact integer arithmetic to better than 1e-8 (relative).");
}
