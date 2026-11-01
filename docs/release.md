# Release checklist

For 0.1.0 and later releases. Publishing is done by a maintainer, by hand; nothing in CI
publishes.

## Before tagging

- [ ] `main` is green in CI (fmt, clippy, stable + MSRV tests, no_std, feature powerset,
      docs, bench build).
- [ ] `./tools/ci.sh` passes locally, including `cargo +1.85 test --all-features` and,
      with a nightly toolchain installed, Miri.
- [ ] Each fuzz target has run for at least 10 minutes on the release commit with no
      findings: `cd fuzz && cargo +nightly fuzz run <target> -- -max_total_time=600`.
- [ ] `cargo llvm-cov --all-features --summary-only` reports ≥ 95 % of lines.
- [ ] If any numerical code changed: `python3 tools/gen_fixtures.py` reproduces the
      committed fixtures byte for byte (`git diff --exit-code tests/fixtures`), and
      `cargo test --release --test accuracy -- --nocapture` matches `docs/accuracy.md`.
- [ ] Every public item is documented (`#![deny(missing_docs)]` builds) and
      `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --no-deps` renders the
      feature badges as expected.
- [ ] `docs/api.md` matches the public API (compare with `cargo doc` output).
- [ ] `cargo semver-checks` against the previous release reports no unintended breakage.
- [ ] `tests/wire_format.rs` and `tests/reproducible.rs` are unchanged since the last
      release, or the changelog explains each change.
- [ ] `CHANGELOG.md`: move entries from *Unreleased* into the new version with today's
      date; add the compare link at the bottom.
- [ ] `Cargo.toml`: bump `version`; check `rust-version`, `description`, `keywords`
      (at most 5), `categories`, `repository`, `documentation`.
- [ ] README: the example still compiles (it is a doctest) and the accuracy table matches
      `docs/accuracy.md`.
- [ ] Examples run: `for e in hmm_forward likelihood briggs_table benford binomial_test; do cargo run
      --release --example $e; done`.

## Packaging

- [ ] `cargo package --list` contains only intended files (sources, tests, fixtures,
      examples, docs, licences) and no `bench/`, `tools/` or `.github/`.
- [ ] `cargo package` succeeds and the `.crate` is well under the 10 MB limit
      (0.1.0: ≈ 1.4 MB, mostly test fixtures).
- [ ] `cargo publish --dry-run` succeeds.
- [ ] Check the crate name is still free: <https://crates.io/crates/mirifici>.

## Publishing (maintainer)

- [ ] `git tag -s v0.1.0 -m "mirifici 0.1.0"` and push the tag.
- [ ] `cargo publish`.
- [ ] Create a GitHub release from the tag with the CHANGELOG section as notes.
- [ ] Confirm docs.rs built all features: <https://docs.rs/mirifici>.

## After

- [ ] Open a new *Unreleased* section in `CHANGELOG.md`.
