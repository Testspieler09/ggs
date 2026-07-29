#!/usr/bin/env bash
set -euo pipefail

source scripts/changed-crates.sh

CRATES=$(get_changed_crates)
[ -z "$CRATES" ] && echo "No relevant changes, skipping fmt check" && exit 0

echo "Running cargo fmt check for: $CRATES"
cargo +nightly fmt --check
