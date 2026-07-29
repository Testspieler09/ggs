#!/usr/bin/env bash
set -euo pipefail

source scripts/changed-crates.sh

CRATES=$(get_changed_crates)
[ -z "$CRATES" ] && echo "No relevant changes, skipping check" && exit 0

for crate in $CRATES; do
  echo "Running cargo check -p $crate"
  cargo check -p "$crate" --all-targets
done
