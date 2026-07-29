#!/usr/bin/env bash
set -euo pipefail

source scripts/changed-crates.sh

CRATES=$(get_changed_crates)
[ -z "$CRATES" ] && echo "No relevant changes, skipping tests" && exit 0

for crate in $CRATES; do
  echo "Running cargo test -p $crate"
  cargo test -p "$crate"
done
