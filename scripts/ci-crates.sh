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
#   scripts/ci-crates.sh --db       # only crates enrolled in ci/db-suites.txt
#                                   # (combinable with --json)
#
# Change-aware mode (repo tasks.md WEB-8). When CI_CHANGED_SINCE names a
# git ref — a pull request's base — only the crates *affected* by the diff
# from `merge-base(ref, HEAD)` to HEAD are listed. A crate is affected when
#
#   - a changed path lies inside it (its `migration/` and `fuzz/` sub-crates
#     are inside it too, so they come along), or
#   - it depends, through a `path = "…"` dependency, on an affected crate —
#     transitively, so editing `entity-ref` re-checks every service that
#     embeds it, and editing a matcher re-checks its service and both fuzz
#     sub-crates, or
#   - the change touches the CI machinery itself (`ci/`, `scripts/`, the
#     workflow files, `rust-toolchain.toml`, `deny.toml`, `.cargo/`), in
#     which case every crate is listed — a change to how checks run is a
#     change to every check.
#
# Unset or empty (a push to main, workflow_dispatch, a local run) lists
# every crate, exactly as before. If the ref cannot be resolved the script
# fails OPEN — every crate, with a note on stderr — because "ran too much"
# is recoverable and "ran nothing" is not. Both pipelines get this from the
# same script (GitHub through the `discover` matrix, Woodpecker through
# `ci-check.sh`'s all-crates loop), so a crate skipped on one is skipped on
# the other. An empty result is printed as `[]` / nothing; the GitHub
# workflow gates its matrix jobs on that.
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

# ---------------------------------------------------------------- filters
json=0; db_only=0
for arg in "$@"; do
  case "${arg}" in
    --json) json=1 ;;
    --db)   db_only=1 ;;
    *) echo "ci-crates.sh: unknown argument '${arg}'" >&2; exit 2 ;;
  esac
done

# Is ${1} inside crate directory ${2}?  ("a/b/c" is inside "a/b"; "a/bc" is not.)
inside() { [[ "${1}" == "${2}" || "${1}" == "${2}/"* ]]; }

if [[ -n "${CI_CHANGED_SINCE:-}" ]]; then
  if base="$(git merge-base "${CI_CHANGED_SINCE}" HEAD 2>/dev/null)"; then
    changed="$(git diff --name-only "${base}" HEAD)"
    run_everything=0
    while IFS= read -r f; do
      [[ -z "${f}" ]] && continue
      case "${f}" in
        ci/*|scripts/*|.github/*|.woodpecker.yml|rust-toolchain.toml|deny.toml|.cargo/*)
          run_everything=1 ;;
      esac
    done <<< "${changed}"

    if [[ "${run_everything}" == 0 ]]; then
      # Directly affected: a changed path inside the crate.
      declare -A affected=()
      while IFS= read -r c; do
        [[ -z "${c}" ]] && continue
        while IFS= read -r f; do
          [[ -z "${f}" ]] && continue
          if inside "${f}" "${c}"; then affected["${c}"]=1; break; fi
        done <<< "${changed}"
      done <<< "${crates}"

      # Path-dependency edges: crate -> the crates it depends on. Only
      # `path = "…"` values that resolve to another discovered crate count;
      # `[lib] path = "src/lib.rs"`-style entries resolve to no Cargo.toml
      # and fall out. Resolved with `cd … && pwd -P` rather than `realpath
      # -m`, which macOS lacks.
      declare -A deps=()
      while IFS= read -r c; do
        [[ -z "${c}" ]] && continue
        while IFS= read -r rel; do
          [[ -z "${rel}" ]] && continue
          d="$(cd "${c}/${rel}" 2>/dev/null && pwd -P)" || continue
          d="${d#"${ROOT}/"}"
          [[ -f "${d}/Cargo.toml" ]] || continue
          deps["${c}"]="${deps["${c}"]:-} ${d}"
        done < <(sed -nE 's/.*path[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "${c}/Cargo.toml" | sort -u)
      done <<< "${crates}"

      # Transitive closure over reverse edges: a dependant of an affected
      # crate is affected. Iterate to a fixpoint (the graph is tiny).
      grew=1
      while [[ "${grew}" == 1 ]]; do
        grew=0
        for c in "${!deps[@]}"; do
          [[ -n "${affected["${c}"]:-}" ]] && continue
          for d in ${deps["${c}"]}; do
            if [[ -n "${affected["${d}"]:-}" ]]; then affected["${c}"]=1; grew=1; break; fi
          done
        done
      done

      crates="$(printf '%s\n' "${!affected[@]}" | sort -u)"
    fi
  else
    echo "ci-crates.sh: cannot resolve CI_CHANGED_SINCE='${CI_CHANGED_SINCE}'; listing every crate" >&2
  fi
fi

if [[ "${db_only}" == 1 ]]; then
  enrolled="$(grep -v '^[[:space:]]*#' ci/db-suites.txt | grep -v '^[[:space:]]*$' | sort -u)"
  crates="$(comm -12 <(printf '%s\n' "${crates}" | sort -u) <(printf '%s\n' "${enrolled}"))"
fi

if [[ "${json}" == 1 ]]; then
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
  # Nothing affected prints nothing at all — not one blank line, which
  # ci-check.sh's `while read` loop would run once as an empty crate path.
  [[ -n "${crates}" ]] && printf '%s\n' "${crates}"
  exit 0
fi
