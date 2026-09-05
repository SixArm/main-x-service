#!/usr/bin/env bash
# Spec-drift check (spec/13-tasks.md T-7).
#
# Fail with a non-zero exit code if any "watched" source file changed in
# the diff but the `spec/` directory did not — unless the changed paths
# match an allowlist pattern in `.spec-allow`.
#
# Usage: spec-drift-check.sh [base-ref] [head-ref]
#
#   base-ref  — the PR base (typically `main`).  Default: $GITHUB_BASE_REF
#               if set, else `main`.
#   head-ref  — the PR head commit.  Default: HEAD.
#
# The script uses only POSIX-compatible bash + git so it can also be run
# locally from a contributor's machine before pushing.
#
# Adapted from the reference implementation in
# ../person-matcher-rust-crate/scripts/spec-drift-check.sh — same
# discipline and structure, but with one fix: that script's
# `watched_pattern`/`spec_pattern` are anchored as if the crate were
# the repository root (`^src/matcher\.rs$`), which never matches here.
# `git diff --name-only` returns paths relative to the **monorepo**
# root regardless of the script's own working directory (verified:
# running the sibling script for real against this repo's history
# produces diffs like `person/person-matcher-rust-crate/src/matcher.rs`,
# which its own `^src/matcher\.rs$` pattern cannot match) — so both
# patterns below are anchored with this crate's own path prefix
# instead.
set -euo pipefail

base_ref="${1:-${GITHUB_BASE_REF:-main}}"
head_ref="${2:-HEAD}"

# This crate's path within the monorepo (`git diff --name-only` paths
# are always repo-root-relative, never relative to this script's own
# working directory).
crate_prefix='person/person-service-with-loco'

# Files whose changes REQUIRE a matching update to the spec/ directory
# (T-7: "src/matching/** or src/models/person.rs changes without a
# spec.md edit"). Conservative initial scope, matching the sibling
# matcher crate's own T-7 — extend as the spec/code-sync discipline
# beds in further.
watched_pattern="^${crate_prefix}/src/(matching/.*\\.rs|models/person\\.rs)\$"

allow_file=".spec-allow"
# The spec is a directory of numbered files (spec/01-*.md .. spec/18-*.md
# + spec/index.md), not a single spec.md. Any changed path under this
# crate's own spec/ counts as "the spec was updated".
spec_pattern="^${crate_prefix}/spec/"

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
  echo "spec-drift-check: watched files AND spec/ both changed — OK."
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

echo "spec-drift-check: FAIL — spec/ MUST be updated in the same PR as watched-file changes." >&2
echo "  Watched files changed in this PR:" >&2
printf '    %s\n' ${watched_changes} >&2
echo "" >&2
echo "  To resolve, either:" >&2
echo "    1. Update the relevant spec/*.md file in the same PR (preferred — keeps spec authoritative)." >&2
echo "    2. Add a path pattern to .spec-allow for genuinely spec-irrelevant changes." >&2
exit 1
