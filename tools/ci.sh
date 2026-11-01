#!/usr/bin/env bash
# Run the same checks as .github/workflows/ci.yml locally.
# Optional tools are used when installed: nightly (Miri), cargo-deny, mpmath.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTFLAGS="-D warnings" RUSTDOCFLAGS="-D warnings"
INTEGRATIONS=serde,num-traits,approx,bytemuck

cargo fmt --all --check
cargo fmt --manifest-path fuzz/Cargo.toml --all --check
cargo fmt --manifest-path bench/Cargo.toml --all --check

cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --features "$INTEGRATIONS" -- -D warnings
cargo clippy --all-targets --no-default-features --features libm -- -D warnings
cargo clippy --manifest-path fuzz/Cargo.toml -- -D warnings
cargo clippy --manifest-path bench/Cargo.toml --all-targets -- -D warnings

# Platform math (the default), then libm (including the reproducibility hash).
cargo test --features "$INTEGRATIONS"
cargo test --all-features
cargo test --no-default-features --features libm
cargo test -q --release --features "$INTEGRATIONS" --test accuracy --test robustness
cargo test -q --release --features libm --test accuracy --test reproducible
cargo +1.85 test --features "$INTEGRATIONS"
cargo +1.85 test --all-features
for e in hmm_forward likelihood briggs_table benford; do cargo run -q --release --example "$e" > /dev/null; done

cargo build --target thumbv7em-none-eabihf --no-default-features --features libm
cargo build --target thumbv7em-none-eabihf --no-default-features --features "libm,$INTEGRATIONS"
cargo hack check --feature-powerset --at-least-one-of std,libm --no-dev-deps
cargo hack test --feature-powerset --at-least-one-of std,libm --lib
cargo doc --no-deps --all-features

node --check site/main.js
cargo fmt --manifest-path site/calc/Cargo.toml --check
cargo clippy --manifest-path site/calc/Cargo.toml --all-targets -- -D warnings
cargo test -q --manifest-path site/calc/Cargo.toml
cargo build -q --release --target wasm32-unknown-unknown --manifest-path site/calc/Cargo.toml

if rustup toolchain list | grep -q nightly; then
  MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-deterministic-floats" \
    cargo +nightly miri test -q --features "$INTEGRATIONS" --test integrations --test core --test signed
fi
if command -v cargo-deny > /dev/null; then
  cargo deny check
fi
if python3 -c "import mpmath" 2> /dev/null; then
  python3 tools/gen_fixtures.py > /dev/null
  git diff --exit-code --stat tests/fixtures
fi
echo "all checks passed"
