//! Print this binary's `CycloneDX` SBOM to stdout.
//!
//! The same document `GET /api/compliance/sbom` serves, available without
//! a running service — so `scripts/sbom.sh` can produce the IEC 62304
//! §8.1.2 / FD&C §524B evidence bundle in CI or on a build machine.
//!
//! Output is deterministic (no timestamp, no serial number), so a
//! reproducible build produces a byte-identical SBOM.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

use care_pathway_service::compliance::soup;

/// Render the SBOM as pretty-printed JSON.
///
/// # Errors
///
/// When serialization fails (in practice: never — the document is plain
/// owned data).
fn main() -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(&soup::sbom())?);
    Ok(())
}
