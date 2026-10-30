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
- **`log1mexp` is a public free function on raw floats and panics for d < 0 or NaN.** It is useful on its own (it is the kernel of subtraction), and returning NaN would break the crate's NaN policy.
- **The log-sum-exp accumulators leave one copy of the maximum out and finish with `m + ln_1p(s)`.** Adding the maximum's `exp(0) = 1` first would round away the low bits of small corrections.
- **`sum_two_pass` and `product_compensated` are associated functions** (`Log::sum_two_pass(&xs)`), so they do not need an extension trait.
- **`Sum`/`Product` are also implemented for `&Log`,** so `slice.iter().sum()` works without `.copied()`.
- **Literals sometimes need a suffix (`Log::from_value(5.0_f64)`).** With two `LogFloat` impls, a bare float literal is ambiguous when a method is called on the result before inference settles.
- **No `alloc` feature.** None of the slice utilities allocates (they work in place or reduce), so they are available everywhere; a feature that gates nothing would be noise.
- **Raw-log slice functions do not validate input.** NaN in gives NaN out (reductions) or `None` with the slice untouched (in-place functions); validating every element would cost a pass for data that is normally clean.
- **In-place normalisers return `Option` of the normaliser.** The normaliser is what a scaled forward algorithm accumulates, and `None` (with the slice untouched) covers the empty / all-zero / infinite cases without NaN.
- **Zero weights exclude their term in weighted means,** even if the value is infinite; that is what a weight of zero means, and it avoids 0·∞.
- **`SignedLog` stores zero with a positive sign,** so there is no −0 and `==`, ordering and `TryFrom<SignedLog> for Log` behave without special cases.
- **A non-zero `SignedLog` divided by zero is an infinity with the numerator's sign,** because zero carries no sign.
- **`SignedLog`'s `Sum` accumulates positive and negative terms separately and subtracts once,** so cancellation happens once rather than at every sign change.
- **`SignedLog` has no powers or roots.** Fractional powers of negative numbers leave the reals; take them on the magnitude.
