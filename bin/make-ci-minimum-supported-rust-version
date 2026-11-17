#!/usr/bin/env bash
# Check and test the tree on Rust N-2 and N-1.
#
# N-2 is the declared MSRV in ci/msrv.txt (spec/rust-msrv-n-minus-2). N-1 is
# the next minor version up, so both ends of the supported range are proven,
# not just the floor. For each toolchain this runs, on every crate (or the
# crates named as arguments):
#
#   1. `ci-check.sh msrv` — rust-version matches ci/msrv.txt, then
#      `cargo +<toolchain> check --all-targets`
#   2. `ci-check.sh test` — `cargo test`, run on that toolchain through
#      RUSTUP_TOOLCHAIN (which overrides rust-toolchain.toml)
#
# DB-gated suites stay skipped, as in the `test` stage. Each toolchain must
# be installed already: rustup toolchain install <version> --profile minimal
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

n2="$(grep -v '^[[:space:]]*#' ci/msrv.txt | grep -v '^[[:space:]]*$' | head -1 | tr -d '[:space:]')"
if [[ ! "${n2}" =~ ^1\.([0-9]+)$ ]]; then
  echo "ci/msrv.txt must hold a bare 1.<minor> version, got '${n2}'" >&2
  exit 2
fi
n1="1.$(( BASH_REMATCH[1] + 1 ))"

failed=0
for toolchain in "${n2}" "${n1}"; do
  label="N-2"; [[ "${toolchain}" == "${n1}" ]] && label="N-1"
  if ! rustup toolchain list | grep -q "^${toolchain}"; then
    echo "Rust ${toolchain} (${label}) is not installed:" >&2
    echo "  rustup toolchain install ${toolchain} --profile minimal" >&2
    failed=1
    continue
  fi
  echo "==> ${label}: Rust ${toolchain}"
  MSRV_TOOLCHAIN="${toolchain}" scripts/ci-check.sh msrv "$@" || failed=1
  RUSTUP_TOOLCHAIN="${toolchain}" scripts/ci-check.sh test "$@" || failed=1
done
exit "${failed}"
