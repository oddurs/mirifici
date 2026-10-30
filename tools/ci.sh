#!/usr/bin/env bash
# Run the same checks as .github/workflows/ci.yml locally.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTFLAGS="-D warnings" RUSTDOCFLAGS="-D warnings"
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo +1.85 test --all-features
cargo build --target thumbv7em-none-eabihf --no-default-features
FEATS=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; f=json.load(sys.stdin)["packages"][0]["features"]; print(",".join(k for k in f if k not in ("default","std")))')
if [ -n "$FEATS" ]; then
  cargo build --target thumbv7em-none-eabihf --no-default-features --features "$FEATS"
fi
cargo hack check --feature-powerset --no-dev-deps
cargo hack test --feature-powerset --lib
cargo doc --no-deps --all-features
if [ -f bench/Cargo.toml ]; then
  cargo fmt --manifest-path bench/Cargo.toml --all --check
  cargo clippy --manifest-path bench/Cargo.toml --all-targets -- -D warnings
fi
echo "all checks passed"
