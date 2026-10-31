# Contributing

Thanks for helping. The crate's bar is *small surface, exact semantics, documented error
bounds*, so changes are judged mostly on whether they keep those three.

## Before opening a pull request

```sh
./tools/ci.sh          # fmt, clippy, tests (stable and 1.85), no_std builds, feature powerset, docs
```

CI additionally runs Miri, the fuzz targets, coverage (≥ 95 % of lines), and tests on
macOS and Windows.

## Changing numerical code

1. State the formula and its edge cases in the rustdoc, as the existing items do.
2. If accuracy can change, regenerate and re-check the references:
   ```sh
   pip install -r tools/requirements.txt   # pinned, so fixtures are reproducible
   python3 tools/gen_fixtures.py               # deterministic; must not change existing rows
   cargo test --release --test accuracy -- --nocapture
   ```
   and update `docs/accuracy.md` with the new maxima. Bounds in `tests/accuracy.rs` are
   the documented guarantees: do not raise one without saying why in the PR.
3. Never let NaN out. Every new operation needs a test for ±0, ±∞, and the largest and
   smallest finite stored logs, and should be added to `tests/robustness.rs` and
   `fuzz/fuzz_targets/arith.rs`.
4. If results under the `libm` feature change, `tests/reproducible.rs` fails. Update its
   expected hash only together with a changelog entry saying which results changed and
   why (see `docs/stability.md`).
5. Never change the serialized form: `tests/wire_format.rs` must keep passing unchanged.
6. Record any semantic choice the brief did not fix in `docs/decisions.md`, with one
   sentence of reasoning.

## Fuzzing locally

```sh
cargo install cargo-fuzz
cd fuzz && cargo +nightly fuzz run parse -- -max_total_time=300
```

Targets: `parse` (arbitrary strings), `format` (arbitrary stored logs and flags), `arith`
(arbitrary operands).

## Benchmarks

`cd bench && cargo bench`. Report numbers with the machine description, as in
`docs/performance.md`.

## Licence

Contributions are dual-licensed under MIT and Apache-2.0, as described in the README.
