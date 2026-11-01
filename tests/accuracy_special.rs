//! Accuracy of Γ, binomial coefficients and the logistic function against mpmath.
//!
//! Same measure as `tests/accuracy.rs`: E = |computed − exact| / (ε · max(1, |exact|)) on
//! the stored log, except where noted. Run with `--nocapture` to print the table used in
//! `docs/accuracy.md`.
#![allow(clippy::cast_precision_loss)]

use mirifici::Log;

fn rows(csv: &str) -> impl Iterator<Item = Vec<&str>> {
    csv.lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(|l| l.split(',').collect())
}

fn err(got: f64, hi: f64, lo: f64, eps: f64, scale: f64) -> f64 {
    if hi.is_infinite() || got.is_infinite() {
        return if got == hi { 0.0 } else { f64::INFINITY };
    }
    (((got - hi) - lo) / (eps * hi.abs().max(scale))).abs()
}

struct Max(f64, String, usize);
impl Max {
    fn new() -> Self {
        Max(0.0, String::new(), 0)
    }
    fn push(&mut self, e: f64, row: &[&str]) {
        assert!(!e.is_nan(), "NaN error at {row:?}");
        self.2 += 1;
        if e > self.0 {
            self.0 = e;
            self.1 = row.join(",");
        }
    }
    fn check(&self, op: &str, ty: &str, bound: f64) {
        println!("| `{op}` | {ty} | {} | {:.3} | {bound} |", self.2, self.0);
        assert!(
            self.0 <= bound,
            "{op} ({ty}): max error {:.3} > {bound}; worst row: {}",
            self.0,
            self.1
        );
    }
}

const F32_EPS: f64 = f32::EPSILON as f64;

#[test]
fn gamma() {
    let mut m = Max::new();
    for r in rows(include_str!("fixtures/gamma_f64.csv")) {
        let x: f64 = r[0].parse().unwrap();
        let got = Log::<f64>::gamma(x).ln();
        m.push(
            err(
                got,
                r[1].parse().unwrap(),
                r[2].parse().unwrap(),
                f64::EPSILON,
                1.0,
            ),
            &r,
        );
    }
    m.check("gamma", "f64", 3.0);
    let mut m = Max::new();
    for r in rows(include_str!("fixtures/gamma_f32.csv")) {
        let x: f32 = r[0].parse().unwrap();
        let got = f64::from(Log::<f32>::gamma(x).ln());
        m.push(
            err(
                got,
                r[1].parse().unwrap(),
                r[2].parse().unwrap(),
                F32_EPS,
                1.0,
            ),
            &r,
        );
    }
    m.check("gamma", "f32", 1.0);
}

#[test]
fn binomial() {
    // Exact or ratio-sum paths (min(k, n−k) ≤ 4096) are measured against |ln C|; the
    // ln Γ path against n·ln n, the size of the logs that cancel.
    let (mut small, mut large) = (Max::new(), Max::new());
    for r in rows(include_str!("fixtures/binomial_f64.csv")) {
        let (n, k): (u64, u64) = (r[0].parse().unwrap(), r[1].parse().unwrap());
        let got = Log::<f64>::binomial(n, k).ln();
        let (hi, lo) = (r[2].parse().unwrap(), r[3].parse().unwrap());
        if k.min(n - k) <= 4096 {
            small.push(err(got, hi, lo, f64::EPSILON, 1.0), &r);
        } else {
            let nf = n as f64;
            large.push(err(got, hi, lo, f64::EPSILON, nf * nf.ln()), &r);
        }
    }
    small.check("binomial, min(k, n−k) ≤ 4096", "f64", 1.0);
    large.check("binomial, larger k (scale n·ln n)", "f64", 3.0);
}

#[test]
fn logistic() {
    let mut m = Max::new();
    for r in rows(include_str!("fixtures/logistic_f64.csv")) {
        let x: f64 = r[0].parse().unwrap();
        let got = Log::<f64>::from_logit(x).ln();
        m.push(
            err(
                got,
                r[1].parse().unwrap(),
                r[2].parse().unwrap(),
                f64::EPSILON,
                1.0,
            ),
            &r,
        );
    }
    m.check("from_logit", "f64", 1.0);
    let mut m = Max::new();
    for r in rows(include_str!("fixtures/logistic_f32.csv")) {
        let x: f32 = r[0].parse().unwrap();
        let got = f64::from(Log::<f32>::from_logit(x).ln());
        m.push(
            err(
                got,
                r[1].parse().unwrap(),
                r[2].parse().unwrap(),
                F32_EPS,
                1.0,
            ),
            &r,
        );
    }
    m.check("from_logit", "f32", 1.0);
}
