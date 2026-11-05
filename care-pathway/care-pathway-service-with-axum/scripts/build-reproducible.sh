#!/usr/bin/env bash
#
# Build a release artefact that can be tied back to its source.
#
# IEC 62304 §8 requires a release's configuration to be identifiable and
# *reconstructible*. A build nobody can reproduce is a build whose
# provenance rests on trust rather than evidence. See
# agents/share/compliance-for-healthcare.md §2.4.
#
# What this script fixes:
#
#   * the toolchain      — pinned by the repository's rust-toolchain.toml
#   * the timestamp      — SOURCE_DATE_EPOCH, taken from the commit itself
#                          rather than from wall-clock time
#   * the commit         — BUILD_SHA, compiled into the binary and reported
#                          at GET /api/compliance
#   * path remapping     — absolute build paths are stripped from the
#                          binary, so two checkouts in different
#                          directories produce identical output
#
# The resulting binary reports `reproducible_release: true` at
# GET /api/compliance; a build without these inputs reports `false`, so
# the claim is evidence rather than assertion.
#
# Usage:
#   scripts/build-reproducible.sh              # build + record provenance
#   scripts/build-reproducible.sh --verify     # build twice, compare hashes
#
# Signing is deliberately NOT done here: signing keys are a deployment
# secret and must not be handled by a repository script
# (agents/share/security.md §5). Sign the artefact in the release pipeline,
# e.g. `cosign sign-blob --key <kms-ref> <artefact>`, and publish the
# signature alongside the SBOM from scripts/sbom.sh.
#
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${CRATE_DIR}"

VERIFY=0
[[ "${1:-}" == "--verify" ]] && VERIFY=1

# Provenance inputs. Derive the timestamp from the commit so that
# rebuilding the same commit tomorrow yields the same bytes.
BUILD_SHA="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
SOURCE_DATE_EPOCH="$(git log -1 --pretty=%ct 2>/dev/null || echo 0)"
export BUILD_SHA SOURCE_DATE_EPOCH

# Strip absolute paths so the artefact does not depend on where it was built.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CRATE_DIR}")"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${REPO_ROOT}=/build --remap-path-prefix=${HOME}/.cargo=/cargo"

echo "==> Toolchain"
rustc --version
echo "    (pinned by rust-toolchain.toml at the repository root)"

echo "==> Provenance"
echo "    BUILD_SHA=${BUILD_SHA}"
echo "    SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}  ($(date -u -r "${SOURCE_DATE_EPOCH}" 2>/dev/null || echo 'epoch 0'))"
if [[ "${BUILD_SHA}" == "unknown" ]]; then
  echo "    WARNING: not a git checkout — the artefact will report"
  echo "             reproducible_release: false at GET /api/compliance."
fi
if ! git diff --quiet 2>/dev/null; then
  echo "    WARNING: the working tree is dirty; ${BUILD_SHA} does not describe these bytes."
fi

build() {
  cargo build --release --locked
}

echo "==> Building"
build
ARTEFACT="target/release/care-pathway-service"
HASH_1="$(shasum -a 256 "${ARTEFACT}" | cut -d' ' -f1)"
echo "    ${ARTEFACT}"
echo "    sha256 ${HASH_1}"

if [[ "${VERIFY}" == "1" ]]; then
  echo "==> Rebuilding from scratch to verify reproducibility"
  cargo clean --release
  build
  HASH_2="$(shasum -a 256 "${ARTEFACT}" | cut -d' ' -f1)"
  echo "    sha256 ${HASH_2}"
  if [[ "${HASH_1}" == "${HASH_2}" ]]; then
    echo "    REPRODUCIBLE: both builds produced identical bytes."
  else
    echo "    NOT REPRODUCIBLE: the two builds differ."
    echo "    Investigate before treating this artefact as IEC 62304 §8 evidence."
    exit 1
  fi
fi

echo "==> Recording the evidence bundle"
scripts/sbom.sh "target/release-evidence"
{
  echo "artefact: ${ARTEFACT}"
  echo "sha256: ${HASH_1}"
  echo "build_sha: ${BUILD_SHA}"
  echo "source_date_epoch: ${SOURCE_DATE_EPOCH}"
  echo "rustc: $(rustc --version)"
} > "target/release-evidence/provenance.txt"

echo
echo "Evidence written to target/release-evidence/"
