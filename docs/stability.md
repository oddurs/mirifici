# Stability policy

What you can rely on across releases, and what CI checks for each promise.

## Versioning

`mirifici` follows [Semantic Versioning](https://semver.org/). Before 1.0, a minor bump
(0.1 → 0.2) may break things and a patch bump (0.1.0 → 0.1.1) may not. From 1.0 on, only
a major bump may break.

"Breaking" covers more than signatures. Each of the following counts as a breaking change:

| promise | where it is stated | checked by |
|---|---|---|
| Public API (types, traits, methods, trait impls) | rustdoc | `cargo-semver-checks` on every pull request |
| Semantics: domain, NaN policy, edge cases, panics | rustdoc, `docs/decisions.md` | unit, property and fuzz tests |
| Accuracy bounds | `docs/accuracy.md`, `tests/accuracy.rs` | 60-digit mpmath fixtures, every platform |
| Serialized form (serde) | `src/integrations/serde.rs` | frozen documents in `tests/wire_format.rs` |
| Text form: `Display` / `FromStr` round trip | rustdoc on `Display` | fixtures and round-trip property tests |
| Bit-exact results with the `libm` feature | `Cargo.toml` feature docs | one hash over a battery of results, `tests/reproducible.rs`, on Linux, macOS and Windows |
| `#![no_std]`, no allocation | crate docs | builds for `thumbv6m`, `thumbv7em`, `wasm32` |
| MSRV 1.85 | README, `Cargo.toml` | tests on 1.85 |

These are **not** breaking changes:

- **Smaller errors.** A more accurate result is a fix, even if it changes the last bit.
  With the `libm` feature it is announced in the changelog, because the
  reproducibility hash changes.
- **Different last-ulp results with the default `std` feature.** With `std`, results
  depend on the platform's math library, which the crate does not control. Only the
  documented bounds are promised there.
- **Printing a shorter mantissa** that still parses back to the same value.
- **New items, new trait impls, new features**, as long as existing code keeps compiling
  under Rust's [API evolution rules](https://rust-lang.github.io/rfcs/1105-api-evolution.html).

## Serialized data

Data serialized by 0.1.0 or any later release deserializes to the same value with every
later release, including across major versions: if the format ever has to change, the old
form will still be read. `tests/wire_format.rs` holds documents written by 0.1.0, and a
change that fails it does not ship.

## Minimum supported Rust version

The MSRV is 1.85. Raising it is a minor-version change (not a patch), it is listed in the
changelog, and the new MSRV is at least six months old when the release ships.

## Dependencies

The only required dependency is `libm` (and only with the `libm` feature). The optional
integrations (`serde`, `num-traits`, `approx`, `bytemuck`) are pinned at their oldest
compatible versions in `Cargo.toml`. CI builds against exactly those lower bounds
(`-Z direct-minimal-versions`), so a downstream crate with an older lock file still
builds. `cargo-deny` checks licences, advisories and sources weekly.

## Reproducible results

By default the crate uses the platform math library through `std`: glibc on Linux, Apple's
libm on macOS, the UCRT on Windows. All of them meet the documented bounds, but they differ
in the last bit. Enable the `libm` feature to route every operation through the pure-Rust
`libm` instead:

```toml
mirifici = { version = "0.1", features = ["libm"] }
```

Results, including formatted text, are then bit-for-bit identical on every platform and
compiler version. CI checks this on x86-64 Linux, arm64 macOS and x86-64 Windows, in debug
and release, by hashing a fixed battery of results against one expected value.
