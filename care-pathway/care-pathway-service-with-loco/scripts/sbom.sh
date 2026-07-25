#!/usr/bin/env bash
#
# Generate the CycloneDX software bill of materials for this crate.
#
# IEC 62304 §8.1.2 wants every SOUP item identified; FD&C Act §524B makes
# the machine-readable form (an SBOM) a premarket requirement for cyber
# devices. See agents/share/compliance-for-healthcare.md §2.4.
#
# Two independent sources, deliberately:
#
#   1. `cargo cyclonedx` (when installed) — the richest output: licences,
#      hashes, and the full dependency tree, straight from cargo metadata.
#   2. The service's own `GET /api/compliance/sbom` — derived at compile
#      time from `Cargo.lock` plus `compliance/soup.tsv`, so it also
#      carries the §8.1.2 purpose and safety-relevance annotations that
#      cargo knows nothing about.
#
# The service endpoint is the authoritative artefact for an audit, because
# it is what the *running binary* reports. This script produces the
# offline copy without needing the service up.
#
# Usage:
#   scripts/sbom.sh [OUTPUT_DIR]        # defaults to target/sbom
#
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-${CRATE_DIR}/target/sbom}"
mkdir -p "${OUT_DIR}"

cd "${CRATE_DIR}"

echo "==> Supply-chain gate (advisories + licences)"
if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check
else
  echo "    cargo-deny not installed; skipping."
  echo "    Install with: cargo install cargo-deny   (deny.toml is already in this crate)"
fi

echo "==> CycloneDX SBOM from cargo metadata"
if command -v cargo-cyclonedx >/dev/null 2>&1; then
  cargo cyclonedx --format json --output-pattern package
  # cargo-cyclonedx writes beside the manifest; collect it.
  find . -maxdepth 1 -name '*.cdx.json' -exec mv {} "${OUT_DIR}/" \;
else
  echo "    cargo-cyclonedx not installed; skipping."
  echo "    Install with: cargo install cargo-cyclonedx"
fi

echo "==> Annotated SBOM from the crate's own SOUP register"
# `--nocapture` prints the rendered document; the same code path serves
# GET /api/compliance/sbom, so this cannot drift from the running service.
cargo run --quiet --bin sbom > "${OUT_DIR}/soup-annotated.cdx.json" 2>/dev/null || {
  echo "    (no sbom binary; fetch it from a running service instead:"
  echo "     curl -s localhost:5150/api/compliance/sbom > ${OUT_DIR}/soup-annotated.cdx.json)"
}

echo "==> SOUP register (human-readable)"
cp compliance/soup.tsv "${OUT_DIR}/soup.tsv"

echo
echo "Artefacts written to ${OUT_DIR}:"
ls -1 "${OUT_DIR}"
