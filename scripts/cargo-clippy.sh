#!/usr/bin/env bash
set -euo pipefail

source scripts/changed-crates.sh

CRATES=$(get_changed_crates)
[ -z "$CRATES" ] && echo "No relevant changes, skipping clippy" && exit 0

for crate in $CRATES; do
  echo "Running cargo clippy -p $crate"
  cargo clippy -p "$crate" --all-targets -- -D warnings
done
