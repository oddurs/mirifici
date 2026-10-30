# Prior art

Surveyed on crates.io on 2026-10-30. Download counts are all-time.

| Crate | Version (updated) | What it is |
|---|---|---|
| [`logprob`](https://crates.io/crates/logprob) | 0.4.0 (2026-04) | `LogProb<T>` wrapper restricted to log *probabilities* (≤ 0), `num_traits::Float` bound, no_std, softmax. ~4.6k downloads. |
| [`logaddexp`](https://crates.io/crates/logaddexp) | 0.1.4 (2026-05) | `ln_add_exp` / `ln_sum_exp` extension traits on raw floats. ~47k downloads. |
| [`logsumexp`](https://crates.io/crates/logsumexp) | 0.1.0 (2023-07) | One-pass `ln_sum_exp` over iterators of raw floats, built on `lnexp`. |
| [`lnexp`](https://crates.io/crates/lnexp) | 0.2.1 | `ln_1p_exp`, `ln_1m_exp` (Mächler 2012) and friends on raw floats. |
| [`log-num`](https://crates.io/crates/log-num) | 0.1.0 (2023-08) | A 32-bit fixed-point logarithmic number system (`L32`) with sign. |

Larger numerics crates (`statrs`, `scirs2-special`, ML frameworks) contain private or
raw-float log-sum-exp helpers, but none exposes a log-space number type.

## Where each falls short

- **`logprob`** is the closest relative and is well made, but its domain is [0, 1]: values
  above one are errors, so it cannot hold likelihoods, partition functions, densities or
  counts. Its operators are named in log space (`+` multiplies probabilities, `-` divides),
  which reads backwards next to the mathematics it implements. Addition is a method that
  returns `Result`. Subtraction of probabilities is not available. It bounds on
  `num_traits::Float`, so no_std relies on `num-traits/libm`. Formatting prints the
  stored log, so a probability of 1e-5000 cannot be printed as such.
- **`logaddexp`** and **`logsumexp`** are correct, small, raw-float helpers. They have no
  type, so nothing stops a caller from adding a raw probability to a log probability.
  `logaddexp` needs `std` and is MIT-only; `logsumexp` has not been updated since 2023.
  Neither documents error bounds or offers checked subtraction.
- **`lnexp`** has the right primitives (including `log1mexp`), but again only on raw floats.
- **`log-num`** is a hardware-style LNS with fixed-point exponent. It is a different goal
  (bit-level LNS emulation, a non-goal here), is incomplete (no addition, placeholder
  `Debug`) and has been idle since 2023.

None of them parses or prints values beyond the float range, tests against high-precision
references, or states an error bound.

## What `mirifici` does differently

1. **One type, value semantics.** `Log<F>` represents any x ≥ 0, and operators mean what
   they mean on x: `a * b` multiplies, `a + b` adds. There is no `Sub` because x − y can
   leave the domain; `checked_sub` and `abs_diff` are explicit, and `SignedLog` exists for
   signed work.
2. **No NaN from valid inputs.** Indeterminate forms (0·∞, 0/0, ∞/∞, ∞ − ∞) are reported
   by `checked_*` methods and panic in operators, with the choice documented.
3. **Documented accuracy.** Error bounds are stated as absolute error in the stored log
   (= relative error in the value) and checked against 50-digit `mpmath` fixtures.
4. **Beyond-range I/O.** `Display`/`FromStr` handle `1e-5000` and `2.5e-90210` exactly
   in log space.
5. **Small sealed float trait** (`f32`, `f64`), `#![no_std]` with `libm`, no
   `num-traits` requirement, dual MIT/Apache-2.0.
