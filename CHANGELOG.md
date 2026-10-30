# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
