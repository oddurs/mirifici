//! Criterion benchmarks: `Log<f64>` against plain `f64`, hand-written log-space code, and
//! the `logaddexp` and `logprob` crates.

#![allow(clippy::needless_range_loop)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use logaddexp::{LogAddExp, LogSumExp};
use logprob::LogProb;
use mirifici::Log;
use mirifici_bench::{Hmm, forward, log_probs};

fn single_ops(c: &mut Criterion) {
    // 1024 independent operations per iteration so loop overhead is amortised.
    let a = log_probs(1024, 1);
    let b = log_probs(1024, 2);
    let la: Vec<Log> = a.iter().map(|&x| Log::from_ln(x)).collect();
    let lb: Vec<Log> = b.iter().map(|&x| Log::from_ln(x)).collect();
    let va: Vec<f64> = a.iter().map(|x| x.exp()).collect();
    let vb: Vec<f64> = b.iter().map(|x| x.exp()).collect();
    let pa: Vec<LogProb<f64>> = a.iter().map(|&x| LogProb::new(x).unwrap()).collect();
    let pb: Vec<LogProb<f64>> = b.iter().map(|&x| LogProb::new(x).unwrap()).collect();
    let mut out = vec![0.0f64; 1024];
    let mut lout = vec![Log::ZERO; 1024];

    let mut g = c.benchmark_group("mul");
    g.throughput(Throughput::Elements(1024));
    g.bench_function("f64", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                out[i] = black_box(va[i]) * vb[i];
            }
        })
    });
    g.bench_function("Log<f64>", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                lout[i] = black_box(la[i]) * lb[i];
            }
        })
    });
    g.finish();

    let mut g = c.benchmark_group("add");
    g.throughput(Throughput::Elements(1024));
    g.bench_function("f64", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                out[i] = black_box(va[i]) + vb[i];
            }
        })
    });
    g.bench_function("hand-written", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                let (x, y) = (black_box(a[i]), b[i]);
                let (hi, lo) = if x > y { (x, y) } else { (y, x) };
                out[i] = hi + (lo - hi).exp().ln_1p();
            }
        })
    });
    g.bench_function("Log<f64>", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                lout[i] = black_box(la[i]) + lb[i];
            }
        })
    });
    g.bench_function("logaddexp", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                out[i] = black_box(a[i]).ln_add_exp(b[i]);
            }
        })
    });
    g.bench_function("logprob", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                out[i] = black_box(pa[i]).add_log_prob_float(pb[i]);
            }
        })
    });
    g.finish();

    let mut g = c.benchmark_group("checked_sub");
    g.throughput(Throughput::Elements(1024));
    let (hi, lo): (Vec<Log>, Vec<Log>) = la
        .iter()
        .zip(&lb)
        .map(|(&x, &y)| if x > y { (x, y) } else { (y, x) })
        .unzip();
    g.bench_function("Log<f64>", |bn| {
        bn.iter(|| {
            for i in 0..1024 {
                lout[i] = black_box(hi[i]).checked_sub(lo[i]).unwrap_or(Log::ZERO);
            }
        })
    });
    g.finish();
}

fn sums(c: &mut Criterion) {
    let mut g = c.benchmark_group("logsumexp");
    for n in [10usize, 1_000, 1_000_000] {
        let raw = log_probs(n, 3);
        let logs: Vec<Log> = raw.iter().map(|&x| Log::from_ln(x)).collect();
        let vals: Vec<f64> = raw.iter().map(|x| x.exp()).collect();
        let probs: Vec<LogProb<f64>> = raw.iter().map(|&x| LogProb::new(x).unwrap()).collect();
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("f64 sum", n), &vals, |bn, v| {
            bn.iter(|| black_box(v).iter().sum::<f64>().ln())
        });
        g.bench_with_input(
            BenchmarkId::new("hand-written two-pass", n),
            &raw,
            |bn, v| {
                bn.iter(|| {
                    let v = black_box(v);
                    let m = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    m + v.iter().map(|x| (x - m).exp()).sum::<f64>().ln()
                })
            },
        );
        g.bench_with_input(BenchmarkId::new("slice::logsumexp", n), &raw, |bn, v| {
            bn.iter(|| mirifici::slice::logsumexp(black_box(v)))
        });
        g.bench_with_input(BenchmarkId::new("Log::sum_two_pass", n), &logs, |bn, v| {
            bn.iter(|| Log::sum_two_pass(black_box(v)))
        });
        g.bench_with_input(
            BenchmarkId::new("Log Sum (streaming)", n),
            &logs,
            |bn, v| bn.iter(|| black_box(v).iter().sum::<Log>()),
        );
        g.bench_with_input(
            BenchmarkId::new("logaddexp ln_sum_exp", n),
            &raw,
            |bn, v| bn.iter(|| black_box(v).iter().copied().ln_sum_exp()),
        );
        g.bench_with_input(
            BenchmarkId::new("logprob log_sum_exp", n),
            &probs,
            |bn, v| bn.iter(|| logprob::log_sum_exp_float(black_box(v))),
        );
    }
    g.finish();
}

fn products(c: &mut Criterion) {
    let n = 1_000_000;
    let logs: Vec<Log> = log_probs(n, 4).into_iter().map(Log::from_ln).collect();
    let mut g = c.benchmark_group("product");
    g.throughput(Throughput::Elements(n as u64));
    g.bench_function("Product", |bn| {
        bn.iter(|| black_box(&logs).iter().product::<Log>())
    });
    g.bench_function("product_compensated", |bn| {
        bn.iter(|| Log::product_compensated(black_box(&logs).iter().copied()))
    });
    g.finish();
}

fn hmm(c: &mut Criterion) {
    // 16 states, 8 symbols, 100 steps: short enough that plain f64 does not underflow
    // (the likelihood is about 1e-90).
    let h = Hmm::random(16, 8, 100, 5);
    let p = forward::LogParams::new(&h);
    let to_log = |v: &[f64]| v.iter().map(|&x| Log::from_ln(x)).collect::<Vec<Log>>();
    let (li, lt, le) = (to_log(&p.init), to_log(&p.trans), to_log(&p.emit));
    let reference = forward::scaled(&h);
    assert!((forward::naive(&h) - reference).abs() < 1e-9);
    assert!((forward::hand_log(&h, &p) - reference).abs() < 1e-9);
    assert!((forward::log_type(&h, &li, &lt, &le).ln() - reference).abs() < 1e-9);

    let mut g = c.benchmark_group("hmm_forward_16x100");
    g.bench_function("f64 naive", |bn| bn.iter(|| forward::naive(black_box(&h))));
    g.bench_function("f64 scaled", |bn| {
        bn.iter(|| forward::scaled(black_box(&h)))
    });
    g.bench_function("hand-written log", |bn| {
        bn.iter(|| forward::hand_log(black_box(&h), &p))
    });
    g.bench_function("Log<f64> Sum", |bn| {
        bn.iter(|| forward::log_type(black_box(&h), &li, &lt, &le))
    });
    g.bench_function("Log<f64> sum_two_pass", |bn| {
        bn.iter(|| forward::log_type_two_pass(black_box(&h), &li, &lt, &le))
    });
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(300))
        .measurement_time(std::time::Duration::from_secs(1))
        .sample_size(30);
    targets = single_ops, sums, products, hmm
}
criterion_main!(benches);
