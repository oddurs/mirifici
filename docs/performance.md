# Performance

Measured with Criterion (`cd bench && cargo bench`) on 2026-10-30.

**Machine:** cloud VM, Intel Xeon @ 2.10 GHz (4 vCPUs, AVX2 and FMA available but not
enabled at compile time), Linux 6.18, glibc 2.39, rustc 1.97.0, default `release`
profile, no `target-cpu=native`. Each benchmark used 30 samples over 1 s; the VM is
shared, so differences under ~10 % are noise.

Comparisons: plain `f64` arithmetic (only meaningful while values stay in range),
hand-written log-space code on raw `f64` logs, and the two crates from the
[prior-art survey](prior-art.md) that cover these operations: `logaddexp` 0.1.4 (raw-float
`ln_add_exp` / `ln_sum_exp`) and `logprob` 0.4.0 (`LogProb::add_log_prob_float`,
`log_sum_exp_float`).

## Single operations (1024 independent operations, median time)

| operation | `f64` | hand-written | `Log<f64>` | `logaddexp` | `logprob` |
|---|---|---|---|---|---|
| multiply | 1.50 µs | — | 1.62 µs | — | — |
| add | 1.54 µs | 16.1 µs | 15.6 µs | 19.1 µs | 17.3 µs |
| `checked_sub` | — | — | 16.9 µs | — | — |

`Log` multiplication costs the same as float multiplication (one add and a NaN check).
Addition costs one `exp` and one `ln_1p`, about 15 ns, the same as every other log-space
implementation.

## Log-sum-exp over a slice (median time)

| n | `f64` sum (+ ln) | hand-written two-pass | `slice::logsumexp` | `Log::sum_two_pass` | `Log` `Sum` (streaming) | `logaddexp` | `logprob` |
|---|---|---|---|---|---|---|---|
| 10 | 7.5 ns | 86 ns | 67 ns | 66 ns | 92 ns | 84 ns | 70 ns |
| 1 000 | 0.69 µs | 7.6 µs | 6.9 µs | 6.2 µs | 6.7 µs | 6.4 µs | 7.0 µs |
| 1 000 000 | 0.73 ms | 6.4 ms | 6.8 ms | 6.7 ms | 6.6 ms | 6.3 ms | 6.8 ms |

All log-space versions spend their time in one `exp` per element (~6.5 ns with glibc)
and are within noise of each other. The streaming `Sum` is slower only for tiny slices,
where its per-element branches are not amortised.

## Products of 10⁶ terms

| `Product` | `product_compensated` |
|---|---|
| 0.75 ms | 1.62 ms |

Compensation roughly doubles the cost (still under 2 ns per term) and removes the
error growth shown in [accuracy.md](accuracy.md).

## HMM forward pass (16 states, 8 symbols, 100 observations)

| plain `f64` | `f64` with per-step rescaling | hand-written log-space | `Log<f64>` with `Sum` | `Log<f64>` with `sum_two_pass` |
|---|---|---|---|---|
| 25.0 µs | 22.4 µs | 257 µs | 245 µs | 260 µs |

The sequence is short enough (likelihood ≈ 10⁻⁹⁰) for plain `f64` to work. Log-space
is ~10× slower because every one of the 16·16·100 inner terms needs an `exp`; this is the
cost of log space itself, not of the `Log` type, which matches hand-written code. For
long sequences, plain `f64` underflows (see `examples/hmm_forward.rs`); per-step
rescaling is the fast alternative when the model allows it.

## Optimisation

No benchmark shows `Log<f64>` behind hand-written log-space code or the other crates by
more than the noise, so no operation was specialised. The remaining gap to plain `f64`
is the transcendental functions; vectorised `exp` (e.g. with `target-cpu=native` and a
SIMD math library) would be the next step, and is outside this crate's scope.
