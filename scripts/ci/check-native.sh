#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all --check
native_tmp=$(mktemp -d)
trap 'rm -rf -- "$native_tmp"' EXIT
mkdir "$native_tmp/real"
ln -s real "$native_tmp/alias"
# Exercise canonical-path contracts even when the platform's temp directory has no alias.
TMPDIR="$native_tmp/alias" cargo test --locked --workspace --all-features --no-fail-fast
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
