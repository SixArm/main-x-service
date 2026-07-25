#!/usr/bin/env bash
#
# List every Cargo workspace root in the repository.
#
# This repository has **no root `Cargo.toml`** — each crate is its own
# workspace — so `cargo fmt --all` / `cargo test --workspace` cannot cover
# the tree from one place. Every CI job therefore iterates this list.
#
# Usage:
#   scripts/ci-crates.sh            # newline-separated paths
#   scripts/ci-crates.sh --json     # JSON array, for a CI matrix
#
# A directory qualifies when it holds a `Cargo.toml` and is not inside
# `target/` or `node_modules/`. Nested workspaces (a crate's `migration/`
# and `fuzz/` sub-crates) are listed separately because they really are
# separate workspaces and are checked separately.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

# `-prune` rather than `-not -path`: the latter still *descends* into
# `target/`, which holds hundreds of thousands of build artefacts and turns
# a sub-second scan into minutes.
crates="$(
  find . \
       \( -name target -o -name node_modules -o -name .svelte-kit -o -name .git \) -prune \
       -o -name Cargo.toml -print \
    | xargs -n1 dirname \
    | sed 's|^\./||' \
    | sort -u
)"

if [[ "${1:-}" == "--json" ]]; then
  # Hand-rolled rather than shelling out to jq, which CI images may lack.
  printf '['
  first=1
  while IFS= read -r c; do
    [[ -z "${c}" ]] && continue
    [[ "${first}" == 1 ]] || printf ','
    printf '"%s"' "${c}"
    first=0
  done <<< "${crates}"
  printf ']\n'
else
  printf '%s\n' "${crates}"
fi
