# mirifici

**[oddurs.github.io/mirifici](https://oddurs.github.io/mirifici/)** · [API reference](https://oddurs.github.io/mirifici/api/mirifici/)

A log-space number type for Rust. `Log<F>` stores a non-negative real number as its
natural logarithm, so multiplication becomes addition, and values like 1e-5000 or
1e+90000 stay representable, comparable and printable.

```rust
use mirifici::Log;

// The probability of 1000 independent events, each with probability 0.4.
let p = Log::from_value(0.4_f64);
let all: Log = std::iter::repeat_n(p, 1000).product();

assert_eq!(0.4_f64.powi(1000), 0.0);            // plain f64 underflows to zero
assert_eq!(format!("{all:.4}"), "1.1481e-398"); // Log keeps the value

// Operators act on values: + adds probabilities, * multiplies them.
let either = all + all;
assert!(either > all);
let parsed: Log = "2.5e-90210".parse().unwrap();
assert_eq!(parsed.to_string(), "2.5e-90210");
```

It is meant for probabilities and likelihoods (HMMs, Bayesian inference, long products
of small factors) and for any magnitude that does not fit in a float. Beyond arithmetic it
has what such code keeps rewriting: Γ, factorials and binomial coefficients; 1 − p and the
logistic function without loss in the tails; and log-sum-exp, softmax, log-dot,
sampling and effective sample size over slices of log-weights.

## Guarantees

- **Domain.** `Log<F>` represents x ≥ 0 as ln(x) in a single `F` (`f32` or `f64`), with
  `#[repr(transparent)]`. `Log::ZERO` stores −∞, `Log::ONE` stores 0, `Log::INFINITY`
  stores +∞. `SignedLog<F>` covers negative values.
- **No NaN.** Constructing from a negative value or NaN is an error (`try_from_value`) or
  a panic (`from_value`). Indeterminate forms (0·∞, 0/0, ∞/∞, ∞−∞) return `None` from the
  `checked_*` methods and panic in the operators. No operation on valid inputs produces
  NaN.
- **No subtraction operator.** x − y can leave the domain, so `Log` offers
  `checked_sub` and `abs_diff`, and `SignedLog` has a full `-`.
- **Accuracy.** Bounds are absolute errors in the stored log, i.e. relative errors in the
  value, in units of ε·max(1, |ln x|):

  | operation | bound (`f64`) |
  |---|---|
  | `*`, `/`, `powf`, `powi`, `sqrt`, `recip` | correctly rounded (≤ 0.5) |
  | `+` | ≤ 1 |
  | `checked_sub` | ≤ 2 (scaled by max(1, \|ln a\|, \|ln(a−b)\|)) |
  | `Sum`, `sum_two_pass`, `slice::logsumexp` (n ≤ 1000) | ≤ 2 |
  | `log2`, `log10`, `from_log2`, `from_log10` | ≤ 0.51 |
  | `FromStr` (any exponent) | ≤ 1 |

  Each bound is checked against 60-digit mpmath references in CI. See
  [`docs/accuracy.md`](docs/accuracy.md), including the `f32` table and the cost of range:
  the stored log of 1e-300 resolves the value to about 13 significant digits, not 16.
- **Beyond-range text.** `Display` matches the float's own output inside its normal range
  and prints scientific notation from the log outside it (`1e-5000`), choosing the
  shortest mantissa that parses back to the same value. `FromStr` accepts exponents of
  any size.
- **`no_std`.** The crate is `#![no_std]` and never allocates. Use
  `default-features = false, features = ["libm"]`.
- **Reproducible.** With the `libm` feature, every result (including formatted text) is
  bit-for-bit identical on every platform; CI checks one hash of a large battery of
  results on Linux, macOS and Windows.
- **Stable.** The public API, accuracy bounds, serialized form and text form are covered
  by the [stability policy](docs/stability.md), each with a CI check; data serialized by
  0.1 stays readable.

## Features

| feature | default | provides |
|---|---|---|
| `std` | yes | the platform math library (glibc, Apple libm, Windows UCRT) |
| `libm` | no | the pure-Rust `libm` for all math: bit-for-bit identical results on every platform; required for `no_std` |
| `serde` | no | `Serialize`/`Deserialize` (stored log; `"-inf"`/`"inf"` strings in JSON), and `serde_as_value` |
| `num-traits` | no | `Zero`, `One`, `Pow`, `Bounded` |
| `approx` | no | `AbsDiffEq`, `RelativeEq`, `UlpsEq` on the stored log |
| `bytemuck` | no | `Pod`, `Zeroable` for `Log<f32>` and `Log<f64>` (zeroed bits are `ONE`) |

## Examples

- [`hmm_forward`](examples/hmm_forward.rs): the forward algorithm over 10,000 dice rolls,
  where `f64` underflows and `Log` gives 4.444e-7569.
- [`likelihood`](examples/likelihood.rs): the likelihood of 100,000 samples, and a
  likelihood ratio between two models.
- [`briggs_table`](examples/briggs_table.rs): a page of a fourteen-place base-10 table in
  the manner of Briggs, checked against references.
- [`benford`](examples/benford.rs): Benford's law on 2¹…2¹⁰⁰⁰⁰ and 1!…3000!.
- [`binomial_test`](examples/binomial_test.rs): exact binomial p-values down to 10⁻¹⁶⁰¹,
  where plain `f64` returns NaN, checked against big-integer arithmetic.

Run one with `cargo run --example hmm_forward`.

## The name

John Napier's *Mirifici Logarithmorum Canonis Descriptio* (Edinburgh, 1614), "A
Description of the Wonderful Canon of Logarithms", introduced logarithms so that the long
multiplications of astronomy and navigation could be done as additions. This crate does
the same for floating-point numbers: it carries the logarithm and lets you write
the multiplication. The API documentation includes short, sourced notes on Napier,
Bürgi, Briggs, Saint-Vincent and de Sarasa, and Mercator, next to the parts of the API
their ideas underlie.

## How it is tested

- Accuracy against 60-digit mpmath references for every operation, `f32` and `f64`,
  with both the platform math library and `libm`.
- Property tests over every float bit pattern and arbitrary strings: no NaN, no
  unexpected panic, exact `Display`/`FromStr` round trips.
- Fuzzing (`fuzz/`: parser, formatter, arithmetic), Miri, ≥ 95 % line coverage, and
  tests on Linux, macOS and Windows, stable, beta and the MSRV, in CI.

## More

- [`docs/api.md`](docs/api.md): the API at a glance
- [`docs/accuracy.md`](docs/accuracy.md): error measurements
- [`docs/performance.md`](docs/performance.md): benchmarks
- [`docs/stability.md`](docs/stability.md): what counts as a breaking change
- [`docs/decisions.md`](docs/decisions.md): design decisions and their reasons
- [`docs/prior-art.md`](docs/prior-art.md): related crates

Minimum supported Rust version: **1.85**, checked in CI.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional terms or
conditions.
