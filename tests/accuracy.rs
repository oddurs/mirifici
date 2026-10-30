//! Accuracy against 60-digit mpmath references (`tools/gen_fixtures.py`).
//!
//! The error measure is
//!
//! ```text
//! E = |computed − exact| / (ε · max(1, |exact|))
//! ```
//!
//! on the stored logarithm, where ε is the machine epsilon of the float type. Absolute error
//! in the stored log is relative error in the value; dividing by `max(1, |exact|)` allows for
//! the half-ulp the representation itself costs (ulp(ln x) ≈ ε·|ln x|). A correctly rounded
//! result has E ≤ 0.5. Two operations use a different scale:
//!
//! - `checked_sub` divides by ε·max(1, |la|, |exact|), because its result is
//!   `la + log1mexp(la − lb)` and the rounding of `log1mexp` (whose magnitude can approach
//!   |la|) survives the final addition;
//! - `to_value` returns a plain float, so E is its ordinary relative error
//!   `|computed − exact| / (ε · |exact|)`.
//!
//! Run `cargo test --release --test accuracy -- --nocapture report` to print the table used
//! in `docs/accuracy.md`.

use mirifici::{Log, LogFloat, log1mexp};

/// One fixture row: input columns and `(hi, lo)` reference pairs.
struct Row {
    cols: Vec<String>,
}

impl Row {
    fn f64(&self, i: usize) -> f64 {
        self.cols[i].parse().unwrap()
    }
    fn reference(&self, i: usize) -> (f64, f64) {
        (self.f64(i), self.f64(i + 1))
    }
}

fn load(csv: &str) -> Vec<Row> {
    csv.lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(|l| Row {
            cols: l.split(',').map(str::to_owned).collect(),
        })
        .collect()
}

/// Error of `got` against `hi + lo`, in units of ε·max(1, |ref|).
fn err_log(got: f64, reference: (f64, f64), eps: f64) -> f64 {
    err_log_scaled(got, reference, eps, 1.0)
}

/// Error of `got` against `hi + lo`, in units of ε·max(scale, |ref|).
fn err_log_scaled(got: f64, (hi, lo): (f64, f64), eps: f64, scale: f64) -> f64 {
    if hi.is_infinite() || got.is_infinite() {
        return if got == hi { 0.0 } else { f64::INFINITY };
    }
    (((got - hi) - lo) / (eps * hi.abs().max(scale))).abs()
}

/// Relative error of `got` against `hi + lo`, in units of ε.
fn err_rel(got: f64, (hi, lo): (f64, f64), eps: f64) -> f64 {
    if hi == 0.0 || hi.is_infinite() {
        return if got == hi { 0.0 } else { f64::INFINITY };
    }
    (((got - hi) - lo) / (eps * hi.abs())).abs()
}

/// The maximum error over all rows, with the row that produced it.
#[derive(Default)]
struct Max {
    err: f64,
    at: String,
    n: usize,
}

impl Max {
    fn push(&mut self, err: f64, row: &Row) {
        assert!(!err.is_nan(), "NaN error at {:?}", row.cols);
        self.n += 1;
        if err > self.err {
            self.err = err;
            self.at = row.cols.join(",");
            self.at.truncate(120);
        }
    }
    fn check(&self, op: &str, ty: &str, bound: f64) {
        println!("| `{op}` | {ty} | {} | {:.3} | {bound} |", self.n, self.err);
        assert!(
            self.err <= bound,
            "{op} ({ty}): max error {:.3} exceeds documented bound {bound}; worst row: {}",
            self.err,
            self.at
        );
    }
}

/// Parses a decimal produced by Python's `repr` into `F`. Inputs are exactly representable.
trait Parse: LogFloat + core::str::FromStr<Err: core::fmt::Debug> + Into<f64> {
    const EPS: f64;
    const TY: &'static str;
}
impl Parse for f64 {
    const EPS: f64 = f64::EPSILON;
    const TY: &'static str = "f64";
}

impl Parse for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const TY: &'static str = "f32";
}

fn p<F: Parse>(s: &str) -> F {
    s.parse().unwrap()
}

/// Documented bounds, in the units of the module docs. Keep in sync with docs/accuracy.md.
struct Bounds {
    add: f64,
    sub: f64,
    log1mexp: f64,
    sum_stream: f64,
    sum_two_pass: f64,
    muldiv: f64,
    powf: f64,
    from_value: f64,
    to_value: f64,
    log2_log10: f64,
}

fn run<F: Parse>(fixtures: [&str; 9], b: Bounds) {
    let [
        add,
        sub,
        l1me,
        sum,
        muldiv,
        powf,
        from_value,
        to_value,
        log2_log10,
    ] = fixtures;
    let eps = F::EPS;
    let ty = F::TY;

    let mut m = Max::default();
    for r in load(add) {
        let (a, bb) = (
            Log::from_ln(p::<F>(&r.cols[0])),
            Log::from_ln(p::<F>(&r.cols[1])),
        );
        m.push(err_log((a + bb).ln().into(), r.reference(2), eps), &r);
        m.push(err_log((bb + a).ln().into(), r.reference(2), eps), &r);
    }
    m.check("a + b", ty, b.add);

    let mut m = Max::default();
    for r in load(sub) {
        let (a, bb) = (
            Log::from_ln(p::<F>(&r.cols[0])),
            Log::from_ln(p::<F>(&r.cols[1])),
        );
        // Scaled by max(1, |la|, |result|): the result is la + log1mexp(la − lb), and the
        // rounding of log1mexp (|log1mexp| ≤ |la| + |result|) carries through unchanged.
        let la: f64 = a.ln().into();
        let got = a.checked_sub(bb).unwrap().ln().into();
        m.push(
            err_log_scaled(got, r.reference(2), eps, la.abs().max(1.0)),
            &r,
        );
    }
    m.check("checked_sub", ty, b.sub);

    let mut m = Max::default();
    for r in load(l1me) {
        m.push(
            err_log(log1mexp(p::<F>(&r.cols[0])).into(), r.reference(1), eps),
            &r,
        );
    }
    m.check("log1mexp", ty, b.log1mexp);

    let (mut ms, mut m2) = (Max::default(), Max::default());
    for r in load(sum) {
        let xs: Vec<Log<F>> = r.cols[0]
            .split(';')
            .map(|s| Log::from_ln(p::<F>(s)))
            .collect();
        let streamed: Log<F> = xs.iter().sum();
        ms.push(err_log(streamed.ln().into(), r.reference(1), eps), &r);
        m2.push(
            err_log(Log::sum_two_pass(&xs).ln().into(), r.reference(1), eps),
            &r,
        );
    }
    ms.check("Sum (streaming)", ty, b.sum_stream);
    m2.check("sum_two_pass", ty, b.sum_two_pass);

    let (mut mm, mut md) = (Max::default(), Max::default());
    for r in load(muldiv) {
        let (a, bb) = (
            Log::from_ln(p::<F>(&r.cols[0])),
            Log::from_ln(p::<F>(&r.cols[1])),
        );
        mm.push(err_log((a * bb).ln().into(), r.reference(2), eps), &r);
        md.push(err_log((a / bb).ln().into(), r.reference(4), eps), &r);
    }
    mm.check("a * b", ty, b.muldiv);
    md.check("a / b", ty, b.muldiv);

    let mut m = Max::default();
    for r in load(powf) {
        let a = Log::from_ln(p::<F>(&r.cols[0]));
        m.push(
            err_log(a.powf(p::<F>(&r.cols[1])).ln().into(), r.reference(2), eps),
            &r,
        );
    }
    m.check("powf", ty, b.powf);

    let mut m = Max::default();
    for r in load(from_value) {
        m.push(
            err_log(
                Log::from_value(p::<F>(&r.cols[0])).ln().into(),
                r.reference(1),
                eps,
            ),
            &r,
        );
    }
    m.check("from_value", ty, b.from_value);

    let mut m = Max::default();
    for r in load(to_value) {
        let x = Log::from_ln(p::<F>(&r.cols[0]));
        m.push(err_rel(x.to_value().into(), r.reference(1), eps), &r);
    }
    m.check("to_value (relative)", ty, b.to_value);

    let (mut m2, mut m10) = (Max::default(), Max::default());
    for r in load(log2_log10) {
        let x = Log::from_ln(p::<F>(&r.cols[0]));
        m2.push(err_log(x.log2().into(), r.reference(1), eps), &r);
        m10.push(err_log(x.log10().into(), r.reference(3), eps), &r);
    }
    m2.check("log2", ty, b.log2_log10);
    m10.check("log10", ty, b.log2_log10);
}

macro_rules! fixtures {
    ($ty:literal) => {
        [
            include_str!(concat!("fixtures/add_", $ty, ".csv")),
            include_str!(concat!("fixtures/sub_", $ty, ".csv")),
            include_str!(concat!("fixtures/log1mexp_", $ty, ".csv")),
            include_str!(concat!("fixtures/sum_", $ty, ".csv")),
            include_str!(concat!("fixtures/muldiv_", $ty, ".csv")),
            include_str!(concat!("fixtures/powf_", $ty, ".csv")),
            include_str!(concat!("fixtures/from_value_", $ty, ".csv")),
            include_str!(concat!("fixtures/to_value_", $ty, ".csv")),
            include_str!(concat!("fixtures/log2_log10_", $ty, ".csv")),
        ]
    };
}

#[test]
fn report_f64() {
    println!("| operation | type | cases | max E | bound |\n|---|---|---|---|---|");
    run::<f64>(
        fixtures!("f64"),
        Bounds {
            add: 1.0,
            sub: 2.0,
            log1mexp: 1.0,
            sum_stream: 2.0,
            sum_two_pass: 2.0,
            muldiv: 0.501,
            powf: 0.501,
            from_value: 1.0,
            to_value: 1.0,
            log2_log10: 0.51,
        },
    );
}

#[test]
fn report_f32() {
    println!("| operation | type | cases | max E | bound |\n|---|---|---|---|---|");
    run::<f32>(
        fixtures!("f32"),
        Bounds {
            add: 1.0,
            sub: 2.0,
            log1mexp: 1.5,
            sum_stream: 2.0,
            sum_two_pass: 2.0,
            muldiv: 0.501,
            powf: 0.501,
            from_value: 1.0,
            to_value: 1.0,
            log2_log10: 0.51,
        },
    );
}
