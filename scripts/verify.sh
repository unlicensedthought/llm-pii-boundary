#!/usr/bin/env bash
# Runs every gate of this repo. The last stdout line is
#   VERIFY {"verdict":"pass","gates":N}   or   VERIFY {"verdict":"fail","failed":"<step>"}
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
gates=0
fail() { echo "VERIFY {\"verdict\":\"fail\",\"failed\":\"$1\"}"; exit 1; }
step() { # name, command...
  local name="$1"; shift
  echo "== $name"
  if "$@"; then gates=$((gates + 1)); else echo "step failed: $name" >&2; fail "$name"; fi
}

command -v cargo >/dev/null || fail "install: cargo not found"
step install cargo fetch --locked
step build   cargo build --locked --all-targets
step fmt     cargo fmt --check
step clippy  cargo clippy --locked --all-targets -- -D warnings
step test    cargo test --locked
step eval    cargo run --locked -q --bin pii-eval
echo "VERIFY {\"verdict\":\"pass\",\"gates\":$gates}"
