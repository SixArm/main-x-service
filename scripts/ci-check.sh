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
#               Set DB_SUITES_FORCE=1 to run an unenrolled crate anyway —
#               that is how a crate gets *observed* green before being
#               added to the allowlist. Never set it in CI: the point of
#               the allowlist is that CI starts green and stays meaningful.
#
# A local Postgres for any of this: scripts/test-db.sh up <crate-path>
# starts the container declared by <crate-path>/compose.test.yaml, with
# the same user/port CI provides.
#   deny        cargo deny check      (where a deny.toml exists)
#   evidence    IEC 62304 artefacts: SBOM + requirement->test traceability
#   fuzz        coverage-guided libFuzzer smoke run, FUZZ_SECONDS (default
#               30) per target, for crates with a fuzz_targets/ directory
#               (each matcher's fuzz/ sub-crate, plus authentication-verifier's
#               and person's own); a no-op for any other crate. Needs
#               nightly + cargo-fuzz on PATH — the CI job installs both;
#               locally: rustup toolchain install nightly && cargo install
#               cargo-fuzz. Short smoke, not exhaustive fuzzing: no corpus
#               is persisted between runs. See each crate's fuzz/README.md.
#   msrv        Minimum Supported Rust Version: assert the crate declares
#               `rust-version` equal to ci/msrv.txt, then `cargo +<msrv>
#               check --all-targets` against that toolchain. A no-op for
#               the nightly-only fuzz/ sub-crates. Needs the MSRV
#               toolchain installed: rustup toolchain install <msrv>
#               --profile minimal. See spec/rust-msrv-n-minus-2/index.md.
#   bench       cargo bench --no-run, for crates declaring a [[bench]].
#               Compiles and links the Criterion harnesses without
#               running them: a benchmark nobody runs still rots, and a
#               bench that no longer compiles is how you find out too
#               late. Measuring on shared CI hardware would produce
#               numbers not worth trusting, so this stage deliberately
#               does not.
#   docs        Repo-wide convention checks (runs once; ignores any crate
#               argument). Today: the agents directory is lowercase —
#               no tracked path and no tracked file content may name an
#               uppercase `AGENTS/` directory. Reads the git index rather
#               than the filesystem, because a case-insensitive
#               filesystem answers for either spelling and hides the
#               defect. See spec/agents-directory-name-is-lowercase/index.md.
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

# The declared MSRV, from its single source of truth. Every crate's
# `rust-version` must equal this; keeping the number in one file is what
# stops ~50 hand-edited manifests drifting apart.
# (spec/rust-msrv-n-minus-2/index.md)
msrv_version() {
  grep -v '^[[:space:]]*#' ci/msrv.txt \
    | grep -v '^[[:space:]]*$' \
    | head -1 \
    | tr -d '[:space:]'
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
        if [[ "${DB_SUITES_FORCE:-}" == "1" ]]; then
          echo "  (not enrolled — running anyway because DB_SUITES_FORCE=1)"
        else
          echo "  (not enrolled for DB-gated tests — see ci/db-suites.txt;"
          echo "   to try it locally: DB_SUITES_FORCE=1 $0 test-db ${crate})"
          return 0
        fi
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
    fuzz)
      # SEC-I2. `${crate}` here is a fuzz/ sub-crate itself (ci-crates.sh
      # lists it as its own Cargo.toml root); only those have a
      # fuzz_targets/ directory, so every non-fuzz crate is a no-op — same
      # pattern as `deny`/`evidence`. `cargo fuzz` must run from the
      # *parent* crate directory, not from inside fuzz/.
      if [[ ! -d "${crate}/fuzz_targets" ]]; then
        echo "  (no fuzz_targets/)"
        return 0
      fi
      local parent seconds target
      parent="$(dirname "${crate}")"
      seconds="${FUZZ_SECONDS:-30}"
      for target in $(cd "${crate}/fuzz_targets" && ls -- *.rs | sed 's/\.rs$//'); do
        echo "  fuzz target: ${target} (${seconds}s)"
        ( cd "${parent}" && cargo +nightly fuzz run "${target}" -- \
            -max_total_time="${seconds}" -rss_limit_mb=4096 )
      done
      ;;
    bench)
      # Same no-op shape as `deny` / `evidence`: a crate without a
      # [[bench]] target has nothing to compile, and `cargo bench
      # --no-run` on one would build the default libtest harness for
      # every target instead, which is slow and proves nothing.
      if ! grep -q '^\[\[bench\]\]' "${crate}/Cargo.toml"; then
        echo "  (no [[bench]] target)"
        return 0
      fi
      ( cd "${crate}" && cargo bench --no-run $(locked_flag "${crate}") )
      ;;
    msrv)
      # The fuzz/ sub-crates are exempt: they are publish=false scaffolding
      # that only ever builds under `cargo +nightly fuzz` with sanitizer
      # instrumentation, so a stable floor would be a claim nothing checks.
      if [[ "${crate}" == */fuzz ]]; then
        echo "  (fuzz sub-crate — nightly only, exempt from the MSRV)"
        return 0
      fi
      local msrv declared
      msrv="$(msrv_version)"
      # Consistency first, because it is instant and catches the common
      # failure: a crate added without the field at all.
      declared="$(sed -n 's/^rust-version *= *"\([^"]*\)".*/\1/p' \
                    "${crate}/Cargo.toml" | head -1)"
      if [[ -z "${declared}" ]]; then
        echo "  ${crate}/Cargo.toml declares no rust-version (expected ${msrv})" >&2
        return 1
      fi
      if [[ "${declared}" != "${msrv}" ]]; then
        echo "  ${crate}/Cargo.toml rust-version = ${declared}, but ci/msrv.txt says ${msrv}" >&2
        return 1
      fi
      if ! rustup toolchain list 2>/dev/null | grep -q "^${msrv}"; then
        echo "  Rust ${msrv} is not installed:" >&2
        echo "    rustup toolchain install ${msrv} --profile minimal" >&2
        return 1
      fi
      # `--all-targets` on purpose: benches and tests reach for APIs the
      # library does not, and an MSRV that only holds for src/ is not one
      # a consumer can rely on.
      ( cd "${crate}" && cargo "+${msrv}" check --all-targets $(locked_flag "${crate}") )
      ;;
    *)
      echo "unknown stage: ${STAGE}" >&2
      exit 2
      ;;
  esac
}

# Repo-wide stages run once and ignore any crate argument: they check the
# git index and tracked content, which have no per-crate slicing.
if [[ "${STAGE}" == "docs" ]]; then
  echo "==> docs: agents-directory-name-is-lowercase"
  # Read the *index*, not the filesystem. On a case-insensitive
  # filesystem `ls` happily answers for either spelling, which is exactly
  # how 878 references to a path that did not exist survived three weeks
  # unnoticed. See spec/agents-directory-name-is-lowercase/index.md.
  docs_failed=0
  bad_paths="$(git ls-files | grep -E '(^|/)AGENTS/' || true)"
  if [[ -n "${bad_paths}" ]]; then
    echo "  tracked paths with an uppercase AGENTS/ directory segment:" >&2
    printf '    %s\n' ${bad_paths} >&2
    echo "  rename via a temporary name (a case-only git mv is a silent" >&2
    echo "  no-op otherwise) — see the spec, §3." >&2
    docs_failed=1
  fi
  # A reference to a path that does not exist is the failure the rule
  # exists to prevent, and it can reappear with no directory renamed —
  # someone simply types the old spelling into a new link.
  # Two files are excluded because they must spell the forbidden form in
  # order to forbid it: this checker and the spec that defines the rule.
  # Excluding anything else would be a hole rather than a base case.
  bad_refs="$(git grep -lI 'AGENTS/' -- . \
      ':(exclude)scripts/ci-check.sh' \
      ':(exclude)spec/agents-directory-name-is-lowercase/index.md' || true)"
  if [[ -n "${bad_refs}" ]]; then
    echo "  files referencing an uppercase AGENTS/ directory:" >&2
    printf '    %s\n' ${bad_refs} >&2
    echo "  the directory is 'agents/'; AGENTS.md the *file* is unchanged." >&2
    docs_failed=1
  fi
  [[ "${docs_failed}" == 0 ]] && echo "  ok: no uppercase AGENTS/ directory path or reference"
  exit "${docs_failed}"
fi

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
