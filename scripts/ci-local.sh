#!/usr/bin/env bash
#
# Run the CI gate on this machine — spec/local-ci/index.md, tasks.md LCI-1.
#
# Everything here shells out to the same entry points hosted CI uses
# (scripts/ci-check.sh, scripts/ci-front-ends.sh), so a local pass and a
# hosted pass run byte-identical commands. This script only chooses WHICH
# stages run and keeps the books; it adds no checks of its own.
#
# Usage:
#   scripts/ci-local.sh [fast|full]        (default: fast)
#
#   fast  fmt, docs, clippy, test on the crates the diff touches, plus
#         front-end check / vitest / lint on the front-ends it touches.
#   full  fast, plus deny, msrv, bench, test-db (needs Postgres), and for
#         front-ends build + the Playwright suites enrolled in
#         ci/front-end-e2e-suites.txt (run serially: two suites in
#         parallel share preview port 4173 and fail wholesale).
#
# Selection is change-aware through CI_CHANGED_SINCE, defaulting here to
# origin/main, including uncommitted and untracked edits (CI_INCLUDE_WORKTREE).
# Set CI_CHANGED_SINCE= (empty) for every crate.
# An unresolvable ref fails OPEN — every crate — never closed.
#
# A stage that cannot run is reported "SKIPPED: <reason>", never green.
# The final line says PASS only when nothing failed AND nothing was
# skipped; otherwise it says what was not proven. Exit status: 0 unless a
# stage failed; with CI_LOCAL_STRICT=1 a skip also exits 3.
#
# Each stage appends one JSON line to .ci-local/results.jsonl (git-ignored)
# — commit, whether the tree was clean, stage, target, outcome, seconds,
# tool versions, host — and its full output goes to .ci-local/logs/.
#
# Not a substitute for hosted CI: this host is not the CI host (see the
# Apple-Silicon-vs-Linux note in spec/local-ci/index.md §1).

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

TIER="${1:-fast}"
case "${TIER}" in
  fast | full) ;;
  *) echo "usage: scripts/ci-local.sh [fast|full]" >&2; exit 2 ;;
esac

export CI_CHANGED_SINCE="${CI_CHANGED_SINCE-origin/main}"
export CI=1
# Select uncommitted edits too (hosted CI never sets this).
export CI_INCLUDE_WORKTREE=1

OUT=".ci-local"
LOGS="${OUT}/logs"
mkdir -p "${LOGS}"
RUN_ID="$(date +%Y%m%dT%H%M%S)"

COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
if [[ -z "$(git status --porcelain 2>/dev/null)" ]]; then CLEAN=true; else CLEAN=false; fi
HOST="$(uname -sm)"
V_RUSTC="$(rustc -V 2>/dev/null || echo none)"
V_NODE="$(node -v 2>/dev/null || echo none)"
V_PNPM="$(pnpm -v 2>/dev/null || echo none)"

FAILED=()
SKIPPED=()
PASSED=0

record() { # stage target outcome seconds
  printf '{"run":"%s","tier":"%s","commit":"%s","clean":%s,"since":"%s","stage":"%s","target":"%s","outcome":"%s","seconds":%s,"rustc":"%s","node":"%s","pnpm":"%s","host":"%s"}\n' \
    "${RUN_ID}" "${TIER}" "${COMMIT}" "${CLEAN}" "${CI_CHANGED_SINCE}" "$1" "$2" "$3" "$4" \
    "${V_RUSTC}" "${V_NODE}" "${V_PNPM}" "${HOST}" >> "${OUT}/results.jsonl"
}

skip() { # stage target reason
  echo "==> $1 [$2]: SKIPPED: $3"
  SKIPPED+=("$1 [$2]: $3")
  record "$1" "$2" skipped 0
}

# run <stage> <target> <command...> — output teed to a per-stage log.
run() {
  local stage="$1" target="$2"; shift 2
  local log="${LOGS}/${RUN_ID}-${stage}-${target//\//_}.log"
  echo "==> ${stage} [${target}]"
  local start=${SECONDS} rc
  "$@" 2>&1 | tee "${log}"
  rc=${PIPESTATUS[0]}
  local secs=$((SECONDS - start))
  if [[ ${rc} -eq 0 ]]; then
    PASSED=$((PASSED + 1)); record "${stage}" "${target}" pass "${secs}"
  else
    echo "    FAILED (exit ${rc}, ${secs}s) — log: ${log}" >&2
    FAILED+=("${stage} [${target}]"); record "${stage}" "${target}" fail "${secs}"
  fi
}

have() { command -v "$1" >/dev/null 2>&1; }

pg_up() {
  (exec 3<>"/dev/tcp/${POSTGRES_HOST:-localhost}/${POSTGRES_PORT:-5432}") 2>/dev/null
}

echo "ci-local: tier=${TIER} since='${CI_CHANGED_SINCE}' commit=${COMMIT:0:8} clean=${CLEAN}"
if [[ "${CLEAN}" == false ]]; then
  echo "ci-local: working tree is dirty; uncommitted edits are selected, and this" >&2
  echo "          run is recorded clean=false (not attestable)." >&2
fi

# ---- Rust / repo-wide stages ------------------------------------------------
run fmt   all scripts/ci-check.sh fmt
run docs  all scripts/ci-check.sh docs
run clippy all scripts/ci-check.sh clippy
run test  all scripts/ci-check.sh test

if [[ "${TIER}" == full ]]; then
  if have cargo-deny; then run deny all scripts/ci-check.sh deny
  else skip deny all "cargo-deny not installed (cargo install cargo-deny)"; fi

  # ci/msrv.txt is one bare `major.minor` line plus comments.
  MSRV="$(grep -v '^[[:space:]]*#' ci/msrv.txt | tr -d '[:space:]')"
  if have rustup && rustup toolchain list | grep -q "^${MSRV}"; then
    run msrv all scripts/ci-check.sh msrv
  else
    skip msrv all "toolchain ${MSRV} not installed (rustup toolchain install ${MSRV} --profile minimal)"
  fi

  run bench all scripts/ci-check.sh bench

  if pg_up; then run test-db all scripts/ci-check.sh test-db
  else skip test-db all "no Postgres on ${POSTGRES_HOST:-localhost}:${POSTGRES_PORT:-5432} (scripts/test-db.sh up <crate>)"; fi
fi

# ---- Front-ends -------------------------------------------------------------
if ! have pnpm; then
  skip front-end all "pnpm not installed"
else
  while IFS= read -r fe; do
    [[ -z "${fe}" ]] && continue
    if [[ ! -d "${fe}/node_modules" ]]; then
      run fe-install "${fe}" bash -c "cd '${fe}' && pnpm install --frozen-lockfile"
    fi
    run fe-check  "${fe}" bash -c "cd '${fe}' && pnpm run check"
    run fe-vitest "${fe}" bash -c "cd '${fe}' && pnpm exec vitest run"
    run fe-lint   "${fe}" bash -c "cd '${fe}' && pnpm run lint"
    if [[ "${TIER}" == full ]]; then
      run fe-build "${fe}" bash -c "cd '${fe}' && pnpm run build"
      if grep -Fxq "${fe}" ci/front-end-e2e-suites.txt; then
        # Serial by construction: this loop never backgrounds a suite.
        run fe-e2e "${fe}" bash -c "cd '${fe}' && pnpm exec playwright test"
      else
        echo "==> fe-e2e [${fe}]: not enrolled in ci/front-end-e2e-suites.txt (by design, not a skip)"
      fi
    fi
  done < <(scripts/ci-front-ends.sh)
fi

# ---- Verdict ----------------------------------------------------------------
echo
echo "ci-local: ${PASSED} passed, ${#FAILED[@]} failed, ${#SKIPPED[@]} skipped  (tier=${TIER}, record: ${OUT}/results.jsonl)"
for f in ${FAILED[@]+"${FAILED[@]}"}; do echo "  FAILED  ${f}"; done 2>/dev/null
for s in ${SKIPPED[@]+"${SKIPPED[@]}"}; do echo "  SKIPPED ${s}"; done 2>/dev/null

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo "ci-local: FAIL"; exit 1
elif [[ ${#SKIPPED[@]} -gt 0 ]]; then
  echo "ci-local: PASS WITH SKIPS — the skipped stages above were NOT proven"
  [[ "${CI_LOCAL_STRICT:-0}" == 1 ]] && exit 3
  exit 0
elif [[ "${CLEAN}" == false ]]; then
  echo "ci-local: PASS (dirty tree — not attestable)"; exit 0
else
  echo "ci-local: PASS (${TIER}, clean tree, ${COMMIT:0:8})"; exit 0
fi
