#!/usr/bin/env bash
#
# List every SvelteKit front-end project in the repository.
#
# Mirrors scripts/ci-crates.sh's shape for the Rust side (tasks.md WEB-2):
# a front-end has no root `package.json` linking it to any other, so
# nothing can discover the sixteen `*-front-end-with-svelte` projects
# except a directory scan. Unlike the Rust crates, front-ends do not
# depend on each other through any local path — each depends only on the
# Lily Design System's published npm packages (agents/share/
# svelte-front-end-stack.md) — so there is no path-dependency graph to
# walk here, just direct-change detection.
#
# Usage:
#   scripts/ci-front-ends.sh            # newline-separated paths
#   scripts/ci-front-ends.sh --json     # JSON array, for a CI matrix
#
# Change-aware mode (repo tasks.md WEB-8, same convention ci-crates.sh
# uses). When CI_CHANGED_SINCE names a git ref — a pull request's base —
# only the front-ends affected by the diff from `merge-base(ref, HEAD)`
# to HEAD are listed: a front-end whose own directory a changed path lies
# inside, or every front-end when the change touches the CI machinery
# itself (`ci/`, `scripts/`, the workflow files, `.woodpecker.yml`) or
# the shared Lily distribution doc (`agents/share/
# svelte-front-end-stack.md`) — a change to how every front-end resolves
# its Lily dependency is a change every front-end needs re-checked
# against.
#
# Unset or empty (a push to main, workflow_dispatch, a local run) lists
# every front-end, exactly as before. If the ref cannot be resolved the
# script fails OPEN — every front-end, with a note on stderr — same
# reasoning as ci-crates.sh: "ran too much" is recoverable, "ran
# nothing" is not.
#
# A directory qualifies when it matches `*/*-front-end-with-svelte` and
# holds a `package.json`. `main-x-service.github.io/` is deliberately
# excluded — it is a public documentation site, not one of the sixteen
# entity/consumer front-ends, and does not share their Lily chrome
# pattern (agents/share/svelte-front-end-stack.md).
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

front_ends="$(
  find . -maxdepth 2 -type d -name '*-front-end-with-svelte' \
    | sed 's|^\./||' \
    | while IFS= read -r d; do
        [[ -f "${d}/package.json" ]] && printf '%s\n' "${d}"
      done \
    | sort -u
)"

# ---------------------------------------------------------------- filters
json=0
for arg in "$@"; do
  case "${arg}" in
    --json) json=1 ;;
    *) echo "ci-front-ends.sh: unknown argument '${arg}'" >&2; exit 2 ;;
  esac
done

inside() { [[ "${1}" == "${2}" || "${1}" == "${2}/"* ]]; }

if [[ -n "${CI_CHANGED_SINCE:-}" ]]; then
  if base="$(git merge-base "${CI_CHANGED_SINCE}" HEAD 2>/dev/null)"; then
    changed="$(git diff --name-only "${base}" HEAD)"
    run_everything=0
    while IFS= read -r f; do
      [[ -z "${f}" ]] && continue
      case "${f}" in
        ci/*|scripts/*|.github/*|.woodpecker.yml|agents/share/svelte-front-end-stack.md)
          run_everything=1 ;;
      esac
    done <<< "${changed}"

    if [[ "${run_everything}" == 0 ]]; then
      declare -A affected=()
      while IFS= read -r fe; do
        [[ -z "${fe}" ]] && continue
        while IFS= read -r f; do
          [[ -z "${f}" ]] && continue
          if inside "${f}" "${fe}"; then affected["${fe}"]=1; break; fi
        done <<< "${changed}"
      done <<< "${front_ends}"
      front_ends="$(printf '%s\n' "${!affected[@]}" | sort -u)"
    fi
  else
    echo "ci-front-ends.sh: cannot resolve CI_CHANGED_SINCE='${CI_CHANGED_SINCE}'; listing every front-end" >&2
  fi
fi

if [[ "${json}" == 1 ]]; then
  printf '['
  first=1
  while IFS= read -r fe; do
    [[ -z "${fe}" ]] && continue
    [[ "${first}" == 1 ]] || printf ','
    printf '"%s"' "${fe}"
    first=0
  done <<< "${front_ends}"
  printf ']\n'
else
  [[ -n "${front_ends}" ]] && printf '%s\n' "${front_ends}"
  exit 0
fi
