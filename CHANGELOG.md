# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### M7: Benchmarks
- Criterion benchmarks in `bench/` (a separate workspace): single operations,
  log-sum-exp on 10 / 1k / 1M elements, products, and an HMM forward pass, against plain
  `f64`, hand-written log-space code, `logaddexp` and `logprob`.
- `docs/performance.md`. No optimisation was needed: `Log<f64>` is within noise of
  hand-written code.

### M6: Integrations
- `serde`: `Serialize`/`Deserialize` for `Log` (stored log; infinities as strings in
  human-readable formats) and `SignedLog`; `serde_as_value` for value-based output.
- `num-traits`: `Zero`, `One`, `Pow<F>`, `Pow<i32>`, `Bounded` for `Log`; `Zero`, `One` for
  `SignedLog`.
- `approx`: `AbsDiffEq`, `RelativeEq`, `UlpsEq` on the stored log.
- `bytemuck`: `Pod`, `Zeroable` for `Log<f32>` and `Log<f64>` (zeroed bits are `ONE`).
- docs.rs builds with all features.

### M5: Formatting and parsing beyond float range
- `Display`, `LowerExp`, `UpperExp` for `Log` and `SignedLog`: identical to the float's
  output in range, scientific notation from the stored log beyond it (`1e-5000`), honouring
  precision, width, fill, alignment, `+` and `0`.
- `FromStr` for `Log` and `SignedLog` with arbitrarily large exponents (`2.5e-90210`);
  `ParseLogError`.
- `Log::display_base` and `LogDisplay` (`2^-1024.5`, `10^-308.4`).
- `log2`, `log10`, `from_log2`, `from_log10` now use double-double constants (≤ 0.5 ulp).
- Parse/display fixtures and round-trip property tests.

### M4: Slices and `SignedLog`
- `mirifici::slice`: `logsumexp`, `log_softmax_in_place`, `softmax_in_place`,
  `log_mean_exp`, `log_mean_exp_weighted` on raw logs; `normalize_in_place`, `mean`,
  `mean_weighted` on `Log` slices. No allocation, no feature gate.
- `SignedLog<F>` with `Add`, `Sub`, `Mul`, `Div`, `Neg` (plus assign/reference forms),
  `checked_*`, `Sum`, `Product`, ordering, `From<Log>` and `TryFrom<SignedLog> for Log`.

### M3: Generic floats and no_std
- `LogFloat` implemented for `f32`; `Log<F>` is generic with `f64` as the default.
- Math routes through `libm` when the `std` feature is off; the crate builds for
  `thumbv7em-none-eabihf`.
- `f32` accuracy fixtures and bounds in `docs/accuracy.md`.

### M2: Addition, subtraction, aggregation
- `Add`, `AddAssign` (log-sum-exp), `checked_sub`, `abs_diff`, and the free function
  `log1mexp` (Mächler 2012).
- Streaming `Sum`, `Log::sum_two_pass`, `Product`, `Log::product_compensated` (Neumaier).
- mpmath fixtures (`tools/gen_fixtures.py`, `tests/fixtures/`), accuracy tests, property
  tests, and `docs/accuracy.md`.

### M1: Core type (f64)
- `Log<f64>` with `ZERO`, `ONE`, `INFINITY`; `from_value`, `try_from_value`, `from_ln`,
  `try_from_ln`, `from_ln_unchecked`, `from_log2`, `from_log10`.
- Accessors `ln`, `log2`, `log10`, `log`, saturating `to_value`, `try_to_value`.
- `Mul`, `Div` and their assign/reference forms; `checked_mul`, `checked_div`, `powf`,
  `powi`, `sqrt`, `recip`.
- `PartialEq`, `PartialOrd`, `total_cmp`, `max`, `min`, `Debug`, `Default`.
- `DomainError`.

### M0: Scaffold
- Cargo project, dual MIT/Apache-2.0 licences, `rustfmt.toml`, `clippy.toml`.
- CI: fmt, clippy, tests on stable and MSRV (1.85), no_std build for
  `thumbv7em-none-eabihf`, feature powerset with `cargo hack`.
- `docs/prior-art.md` (survey of existing crates) and `docs/api.md` (API sketch).
