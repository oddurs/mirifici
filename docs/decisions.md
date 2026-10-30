# Decisions

Choices not fixed by the design brief. Each one keeps NaN out and the API small.

- **MSRV 1.85.** First release with edition 2024 and the MSRV-aware resolver; `core::error::Error` (1.81) is available.
- **`LogFloat` arrives in M1, implemented for `f64` only.** The crate is `#![no_std]` from day one, so math has to be routed through a trait anyway; M3 adds `f32` instead of rewriting M1.
- **`checked_mul` / `checked_div` return `Option`, not `Result`.** It matches `checked_sub` and the standard library's `checked_*` convention; there is only one failure reason for each.
- **∞/∞ is indeterminate, like 0/0.** `inf - inf` in the stored log is NaN, and either answer (0, 1, ∞) would be a guess.
- **`powf` follows IEEE `pow` for 0⁰ = ∞⁰ = 1ᵖ = 1 and panics on a NaN exponent.** These are the only cases where `p · ln` would be NaN.
- **`-0.0` is never stored by checked constructors or by arithmetic.** `from_ln(-0.0)` and `ONE.recip()` store `+0.0` so `Debug` and bit-level comparisons are unsurprising.
- **No `TryFrom<F> for Log<F>`.** `try_from_value` is the one fallible constructor; a trait impl would be a second spelling of it.
- **No `Ord` wrapper.** `total_cmp` covers sorting and `max_by`; a wrapper type would add surface for little gain.
- **`try_to_value` returns `None` only when the result rounds to 0 or ∞.** Subnormal results are returned (with reduced precision) because `F` can represent them.
- **`log(base)` panics on a base that is not finite, positive and ≠ 1.** Every such base would make the result NaN or meaningless.
- **`max` / `min` are provided.** Max-product (Viterbi) code needs them and `PartialOrd` alone does not give them.
- **`Log::ZERO` needs a type annotation (`Log::<f64>::ZERO`).** Rust does not use default type parameters for inference; this is a language limitation, not a choice.
