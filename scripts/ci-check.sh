#!/usr/bin/env bash
#
# Run one CI stage across one crate (or every crate).
#
# Shared by the GitHub Actions and Woodpecker pipelines so both run
# byte-identical commands — a check that only fails on one platform is a
# check nobody trusts.
#
# Usage:
#   scripts/ci-check.sh <stage> [crate-path]
#
# Stages:
#   fmt         cargo fmt --check
#   clippy      cargo clippy --all-targets -- -D warnings
#   test        cargo test            (DB-gated suites stay skipped)
#   test-db     cargo test -- --ignored --test-threads=1, for crates
#               enrolled in ci/db-suites.txt; a no-op for any other
#               crate. Serial because the suites share one database.
#   deny        cargo deny check      (where a deny.toml exists)
#   evidence    IEC 62304 artefacts: SBOM + requirement->test traceability
#
# With no crate path, the stage runs across every crate from
# scripts/ci-crates.sh.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

STAGE="${1:?usage: ci-check.sh <stage> [crate-path]}"
CRATE="${2:-}"

# Postgres connection CI provides. Each crate gets its own database so a
# truncating test harness in one cannot corrupt another's run.
PG_HOST="${POSTGRES_HOST:-localhost}"
PG_PORT="${POSTGRES_PORT:-5432}"
PG_USER="${POSTGRES_USER:-loco}"
PG_PASSWORD="${POSTGRES_PASSWORD:-loco}"

# Is this crate enrolled for DB-gated tests? (see ci/db-suites.txt)
enrolled_for_db() {
  local crate="$1"
  grep -v '^[[:space:]]*#' ci/db-suites.txt \
    | grep -v '^[[:space:]]*$' \
    | grep -Fxq "${crate}"
}

# A Postgres-safe database name derived from the crate path.
db_name_for() {
  printf 'ci_%s' "$(printf '%s' "$1" | tr '/-' '__' | cut -c1-55)"
}

# `--locked` only where a lockfile is actually committed.
#
# The `fuzz` sub-crates gitignore their `Cargo.lock`, so on a fresh CI
# checkout there is nothing to lock against and `--locked` fails outright
# ("the lock file needs to be updated but --locked was passed"). Passing it
# unconditionally would have made CI red on its first run for every fuzz
# crate. Where a lockfile *is* committed, `--locked` is what stops a
# dependency drifting silently between a local run and CI.
locked_flag() {
  local crate="$1"
  if git ls-files --error-unmatch "${crate}/Cargo.lock" >/dev/null 2>&1; then
    printf -- '--locked'
  fi
}

run_stage() {
  local crate="$1"
  case "${STAGE}" in
    fmt)
      ( cd "${crate}" && cargo fmt --check )
      ;;
    clippy)
      # `-D warnings` turns the crate-root `#![warn(clippy::pedantic)]`
      # into a hard failure, which is the only way the lint stays at zero.
      ( cd "${crate}" && cargo clippy --all-targets $(locked_flag "${crate}") -- -D warnings )
      ;;
    test)
      ( cd "${crate}" && cargo test $(locked_flag "${crate}") )
      ;;
    test-db)
      if ! enrolled_for_db "${crate}"; then
        echo "  (not enrolled for DB-gated tests — see ci/db-suites.txt)"
        return 0
      fi
      local db
      db="$(db_name_for "${crate}")"
      echo "  database: ${db}"
      # Recreate from scratch. These suites assert on whole-table state
      # (row counts, `MIN(seq)`, an entire verified audit chain), so a
      # database left over from an earlier run is not a neutral starting
      # point — it is the difference between a real failure and a stale
      # one. The name is `ci_*`, owned by this script alone.
      PGPASSWORD="${PG_PASSWORD}" dropdb --if-exists -h "${PG_HOST}" \
        -p "${PG_PORT}" -U "${PG_USER}" "${db}"
      PGPASSWORD="${PG_PASSWORD}" createdb -h "${PG_HOST}" -p "${PG_PORT}" \
        -U "${PG_USER}" "${db}"
      PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" \
        -U "${PG_USER}" -d "${db}" -q -c \
        'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
         CREATE EXTENSION IF NOT EXISTS citext;
         CREATE EXTENSION IF NOT EXISTS unaccent;' || true
      # Crates that carry hand-written SQL migrations need them applied:
      # unlike the loco-idiomatic services, they do not migrate on boot,
      # so their suite would otherwise run against an empty schema and
      # fail with `relation ... does not exist`. Directory names are
      # timestamp-prefixed, so lexicographic order is apply order.
      if ls "${crate}"/migrations/*/up.sql >/dev/null 2>&1; then
        local up
        for up in $(ls "${crate}"/migrations/*/up.sql | sort); do
          PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" \
            -U "${PG_USER}" -d "${db}" -q -v ON_ERROR_STOP=1 -f "${up}"
        done
        echo "  applied $(ls "${crate}"/migrations/*/up.sql | wc -l | tr -d ' ') SQL migrations"
      fi
      # `--test-threads=1` is required, not a preference. Every DB-gated
      # suite in a crate shares one database, and many assert on whole-table
      # state: the audit-chain tests verify the *entire* `audit_log` and
      # count its rows, so any other test writing an audit row concurrently
      # breaks them. Running them in parallel produced failures that looked
      # like chain defects but were only test interference.
      ( cd "${crate}" \
        && DATABASE_URL="postgres://${PG_USER}:${PG_PASSWORD}@${PG_HOST}:${PG_PORT}/${db}" \
           cargo test $(locked_flag "${crate}") -- --ignored --test-threads=1 )
      ;;
    deny)
      if [[ ! -f "${crate}/deny.toml" ]]; then
        echo "  (no deny.toml)"
        return 0
      fi
      ( cd "${crate}" && cargo deny check )
      ;;
    evidence)
      # IEC 62304 §8.1.2 / FD&C §524B. Only crates carrying the evidence
      # artefacts participate; the rest are a no-op.
      if [[ ! -f "${crate}/compliance/traceability.tsv" ]]; then
        echo "  (no compliance/ artefacts)"
        return 0
      fi
      # The traceability check and the SOUP-annotation gate are ordinary
      # tests, so `test` already runs them. What this stage adds is the
      # rendered SBOM, kept as a build artefact.
      ( cd "${crate}" && cargo run $(locked_flag "${crate}") --quiet --bin sbom > /tmp/sbom.json \
        && echo "  SBOM: $(wc -c < /tmp/sbom.json) bytes" )
      ;;
    *)
      echo "unknown stage: ${STAGE}" >&2
      exit 2
      ;;
  esac
}

if [[ -n "${CRATE}" ]]; then
  echo "==> ${STAGE}: ${CRATE}"
  run_stage "${CRATE}"
else
  failed=0
  while IFS= read -r crate; do
    [[ -z "${crate}" ]] && continue
    echo "==> ${STAGE}: ${crate}"
    if ! run_stage "${crate}"; then
      echo "    FAILED: ${crate}" >&2
      failed=1
    fi
  done < <(scripts/ci-crates.sh)
  exit "${failed}"
fi
