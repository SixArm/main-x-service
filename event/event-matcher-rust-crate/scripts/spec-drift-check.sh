#!/usr/bin/env bash
# Spec-drift check (CI guardrail; see spec/09-public-api-contract-semver.md).
#
# Fail with a non-zero exit code if any "watched" source file changed in
# the diff but no spec file under `spec/` did — unless the changed paths
# match an allowlist pattern in `.spec-allow`.
#
# Note: the spec is a directory (`spec/index.md` + `spec/NN-*.md`), not a
# single top-level `spec.md` file. The check is satisfied when any path
# under `spec/` is updated in the same diff.
#
# Usage: spec-drift-check.sh [base-ref] [head-ref]
#
#   base-ref  — the PR base (typically `main`).  Default: $GITHUB_BASE_REF
#               if set, else `main`.
#   head-ref  — the PR head commit.  Default: HEAD.
#
# The script uses only POSIX-compatible bash + git so it can also be run
# locally from a contributor's machine before pushing.

set -euo pipefail

base_ref="${1:-${GITHUB_BASE_REF:-main}}"
head_ref="${2:-HEAD}"

# Files whose changes REQUIRE a matching update to the spec.
# Scope covers the behaviour-bearing source modules the spec describes:
# matcher, scorer, normalizer, and models.
watched_pattern='^src/(matcher|scorer|normalizer|models)\.rs$'

allow_file=".spec-allow"
# The spec lives under spec/ (index.md + NN-*.md), not a top-level file.
spec_pattern='^spec/'

# Resolve a base ref that exists locally.  In GitHub Actions PRs the
# default checkout fetches `pull/<n>/merge`; on a contributor machine
# `main` or `origin/main` usually works.
if git rev-parse --verify --quiet "origin/${base_ref}" >/dev/null; then
  base_commit="origin/${base_ref}"
elif git rev-parse --verify --quiet "${base_ref}" >/dev/null; then
  base_commit="${base_ref}"
else
  echo "spec-drift-check: cannot resolve base ref '${base_ref}' (tried origin/${base_ref} and ${base_ref}); skipping." >&2
  exit 0
fi

merge_base=$(git merge-base "${base_commit}" "${head_ref}" 2>/dev/null || echo "${base_commit}")
changed_files=$(git diff --name-only "${merge_base}" "${head_ref}")

watched_changes=$(printf '%s\n' "${changed_files}" | grep -E "${watched_pattern}" || true)

if [ -z "${watched_changes}" ]; then
  echo "spec-drift-check: no watched files changed — spec sync not required."
  exit 0
fi

spec_changed=$(printf '%s\n' "${changed_files}" | grep -E "${spec_pattern}" || true)

if [ -n "${spec_changed}" ]; then
  echo "spec-drift-check: watched files AND spec/*.md both changed — OK."
  exit 0
fi

# Allowlist: if EVERY watched-change line matches at least one allow
# pattern, the PR is permitted.  A pattern is an extended regex matched
# against the path; blank lines and `#`-prefixed lines are ignored.
if [ -f "${allow_file}" ]; then
  # Build the union of allow patterns.
  union_pattern=$(grep -Ev '^\s*(#|$)' "${allow_file}" | paste -sd '|' -)
  if [ -n "${union_pattern}" ]; then
    unallowed=$(printf '%s\n' "${watched_changes}" | grep -Ev "${union_pattern}" || true)
    if [ -z "${unallowed}" ]; then
      echo "spec-drift-check: all watched changes match ${allow_file} patterns — OK."
      exit 0
    fi
  fi
fi

echo "spec-drift-check: FAIL — a spec file under spec/ MUST be updated in the same PR as watched-file changes." >&2
echo "  Watched files changed in this PR:" >&2
printf '    %s\n' ${watched_changes} >&2
echo "" >&2
echo "  To resolve, either:" >&2
echo "    1. Update spec/*.md in the same PR (preferred — keeps spec authoritative)." >&2
echo "    2. Add a path pattern to .spec-allow for genuinely spec-irrelevant changes." >&2
exit 1
