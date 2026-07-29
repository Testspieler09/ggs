#!/usr/bin/env bash
# Returns the list of workspace crate names whose source files are staged.
# Crates are ordered by dependency: ggs-core first, then dependents.
set -euo pipefail

CHANGED_FILES=$(git diff --cached --name-only 2>/dev/null || true)

# Ordered list of workspace members (dependency order).
CRATES="ggs-core ggs-strategy simulate train"

# Returns 1 if the given crate has staged files.
crate_changed() {
  local crate="$1"
  # Workspace root Cargo.toml change affects everything.
  echo "$CHANGED_FILES" | grep -q "^Cargo\.toml$" && return 0
  echo "$CHANGED_FILES" | grep -q "^${crate}/" && return 0
  return 1
}

# ggs-strategy / simulate / bench / train all depend on ggs-core.
depends_on_core() {
  case "$1" in
    ggs-strategy|simulate|train) return 0 ;;
    *) return 1 ;;
  esac
}

get_changed_crates() {
  local result=""
  local core_changed=0

  crate_changed "ggs-core" && core_changed=1 || true

  for crate in $CRATES; do
    if crate_changed "$crate"; then
      result="$result $crate"
    elif [[ $core_changed -eq 1 ]] && depends_on_core "$crate"; then
      result="$result $crate"
    fi
  done

  result="${result# }"  # trim leading space
  echo "$result"
}
