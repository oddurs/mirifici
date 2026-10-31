# Accuracy

## How error is measured

A `Log` stores ln(x). If the stored log is off by δ, the value is off by a factor
e^δ = 1 + δ + O(δ²). **Absolute error in the stored log is relative error in the value**,
so every bound below is a bound on the relative error of x.

The representation has a floor of its own: the nearest float to ln(x) can be half an ulp
away, and ulp(ln x) ≈ ε·|ln x|. A value near 1 is stored with full precision, while
1e-300 (|ln x| ≈ 690) is stored with a relative error of up to ~690·ε/2 ≈ 7.7·10⁻¹⁴
in `f64`. Bounds are therefore stated in units of

```text
E = |computed − exact| / (ε · max(1, |exact|))
```

on the stored log, where ε is the machine epsilon of the float (2⁻⁵² for `f64`). A
correctly rounded result has E ≤ 0.5. Exceptions to this scale:

- `checked_sub` divides by ε·max(1, |la|, |exact|). Its result is
  `la + log1mexp(la − lb)`, and when a and b are close, `log1mexp` is large and negative
  and its rounding survives the final addition.
- `to_value` returns a plain float, so E is its ordinary relative error in units of ε.

Inputs in the fixtures are exact floats; the reference is the exact result for those
inputs. Subtraction is ill-conditioned for any representation: if the inputs carry
relative error δ, `a − b` carries about δ·a/(a − b) on top of the bounds below.

## Method

`tools/gen_fixtures.py` draws inputs across many scales (|ln x| from 10⁻²⁰ to 10³⁰⁰,
pairs that are equal, close, and far apart) and computes references with
[mpmath](https://mpmath.org) at 60 significant digits. Each reference is stored as two
doubles `hi + lo`, so the tests resolve errors far below one ulp. The fixtures are
committed under `tests/fixtures/`, and `tests/accuracy.rs` checks every row against the
bounds below on every CI run, with both the `std` math library and `libm`.

## Results: `f64`

Measured with `cargo test --release --test accuracy -- --nocapture` (glibc via
`std`; the `libm` column is `--no-default-features`).

| operation | cases | max E (`std`) | max E (`libm`) | documented bound |
|---|---|---|---|---|
| `a * b` | 2000 | 0.491 | 0.491 | 0.5 (correctly rounded) |
| `a / b` | 2000 | 0.497 | 0.497 | 0.5 (correctly rounded) |
| `powf` | 2000 | 0.477 | 0.477 | 0.5 (correctly rounded) |
| `a + b` | 4000 | 0.751 | 0.663 | 1 |
| `checked_sub` | 1510 | 0.854 | 0.854 | 2 (scale includes \|la\|) |
| `log1mexp` | 2000 | 0.524 | 0.624 | 1.5 |
| `Sum` (streaming), n ≤ 1000 | 170 | 1.067 | 1.067 | 2 |
| `sum_two_pass`, n ≤ 1000 | 170 | 1.067 | 1.067 | 2 |
| `from_value` | 2000 | 0.492 | 0.492 | 1 |
| `to_value` (relative error) | 2000 | 0.483 | 0.520 | 1 |
| `log2` | 2000 | 0.497 | 0.497 | 0.51 |
| `log10` | 2000 | 0.481 | 0.481 | 0.51 |

`sqrt` and `recip` are exact. `powi(n)` is `powf(n as F)`. `log2`, `log10`,
`from_log2` and `from_log10` multiply by their constant held to twice the float's
precision (a double-double), so they are correctly rounded up to a tiny fraction of an ulp.

## Results: `f32`

Same fixtures generator, with inputs exactly representable in `f32` and ε = 2⁻²³. The
`libm` column uses the single-precision `libm` functions (`expf`, `log1pf`, …), which are
slightly less accurate than glibc's.

| operation | cases | max E (`std`) | max E (`libm`) | documented bound |
|---|---|---|---|---|
| `a * b` | 2000 | 0.487 | 0.487 | 0.5 (correctly rounded) |
| `a / b` | 2000 | 0.493 | 0.493 | 0.5 (correctly rounded) |
| `powf` | 2000 | 0.476 | 0.476 | 0.5 (correctly rounded) |
| `a + b` | 4000 | 0.544 | 0.514 | 1 |
| `checked_sub` | 1506 | 1.111 | 1.111 | 2 (scale includes \|la\|) |
| `log1mexp` | 2000 | 0.655 | 1.035 | 1.5 |
| `Sum` (streaming), n ≤ 1000 | 170 | 0.962 | 0.962 | 2 |
| `sum_two_pass`, n ≤ 1000 | 170 | 0.786 | 0.786 | 2 |
| `from_value` | 2000 | 0.492 | 0.587 | 1 |
| `to_value` (relative error) | 2000 | 0.488 | 0.537 | 1 |
| `log2` | 2000 | 0.487 | 0.487 | 0.51 |
| `log10` | 2000 | 0.488 | 0.488 | 0.51 |

With ε ≈ 1.2·10⁻⁷, a `Log<f32>` holding 1e-30 (|ln x| ≈ 69) carries a representation
error of up to ~4·10⁻⁶ relative. Use `f64` unless memory or bandwidth dominates.

## Text: `FromStr` and `Display` beyond the float's range

`tests/text.rs` checks against `tests/fixtures/{parse,display}_{f64,f32}.csv`:

- **Parsing** 1000 decimal strings with 1–25 significant digits and exponents up to
  ±10¹⁵: max E = 0.482 (`f64`) and 0.470 (`f32`); documented bound 1. Outside the normal
  range the stored log is `ln(m) + k·ln 10` with `ln 10` in double-double; inside it the
  float's own correctly rounded parser is used, so `s.parse::<Log>()` equals
  `Log::from_value(s.parse::<f64>()?)` exactly.
- **Displaying** 1000 stored logs outside the float's range with a fixed precision
  (`{:.12e}` for `f64`, `{:.5e}` for `f32`): every mantissa matches mpmath's correctly
  rounded digits of the exact value e^ln. The decimal exponent and the mantissa come from
  `ln · log10(e)` in double-double.
- **Round trips** (property tests): `parse(display(x)) == x` exactly for every `x`
  outside the float's range with |log10 x| < 2⁵³ (|ln x| < 2·10¹⁶), because the default precision is the
  shortest mantissa that parses back to the same `Log`. Inside the range `Display`
  prints the float `to_value()`, which costs one rounding of the value.

Beyond |log10 x| ≥ 2⁵³ (|ln x| ≳ 2·10¹⁶) not every integer decimal exponent is an `f64`,
so the value is printed as the nearest power of ten and round-trips to within an ulp or
two of the stored log (whose ulp there is at least 4).

### Sums and products of many terms

The sum bounds above are for n ≤ 1000 terms. In general the accumulation in `Sum` and
`sum_two_pass` is ordinary floating-point summation of terms in (0, 1], with the classical
worst case of about n·ε relative error in the accumulator (typically √n·ε), which is the
same absolute error in the stored log.

`Product` adds the stored logs with plain floating-point addition, whose worst-case
error is about n·ε·Σ|lᵢ|. `product_compensated` uses Neumaier summation and stays within
one ulp of the result regardless of n. In `tests/additive.rs`, a product of 10⁶
likelihood terms has an error of **153 ulp** with `Product` and **0 ulp** with
`product_compensated`.
