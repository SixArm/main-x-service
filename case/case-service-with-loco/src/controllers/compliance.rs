//! Compliance evidence endpoints — the runtime surface an assessment
//! reads instead of reconstructing the deployment's configuration by hand.
//!
//! | Endpoint | Evidence |
//! |---|---|
//! | `GET /api/compliance` | Software identification and build provenance: version, source commit, whether the artefact carries reproducible-release evidence. |
//! | `GET /api/compliance/sbom` | `CycloneDX` 1.5 SBOM + SOUP register (IEC 62304 §8.1.2, ISO/IEC 27001 A.8). |
//!
//! The chain-, record-, and checkpoint-verification endpoints live under
//! `/api/cases/*` alongside the data they verify; this module carries only
//! the two service-level artefacts, which are about the *binary* rather
//! than about any case.
//!
//! ## Why these are guarded
//!
//! Both sit under `/api/*`, so they are behind the blanket auth + ABAC
//! guard when `CASE_REQUIRE_AUTH` is on (the guard is deny-unless-public
//! and neither path is on the allow-list). They are **reads**, so the
//! default ABAC policy admits any authenticated caller — appropriate,
//! since the point is that an auditor can read the posture, and nothing
//! here discloses case data.
//!
//! The SBOM is deliberately **not** public. It names the exact version of
//! every dependency in the running binary, which is precisely what an
//! attacker needs to match the deployment against published advisories.
//! Publishing an SBOM is a decision for the operator to make explicitly,
//! not a default this service makes for them.

use loco_rs::prelude::*;
use serde::Serialize;

use crate::compliance::{Build, soup};

/// The service-identification document served at `GET /api/compliance`.
#[derive(Debug, Clone, Serialize)]
pub struct Identification {
    /// Service name.
    pub service: &'static str,
    /// Build provenance.
    pub build: Build,
    /// Whether the artefact carries reproducible-release evidence.
    pub reproducible_release: bool,
    /// What this service does **not** claim, stated so an assessment does
    /// not have to infer it from silence.
    pub not_claimed: Vec<&'static str>,
}

impl Identification {
    /// Read the current identification.
    #[must_use]
    pub fn current() -> Self {
        let build = Build::current();
        Self {
            service: env!("CARGO_PKG_NAME"),
            build,
            reproducible_release: build.is_reproducible_release(),
            not_claimed: vec![
                "IEC 62304 safety classification: this is a governmental case registry, \
                 not health software, and no output drives an individual's treatment.",
                "FD&C Act §524B premarket SBOM: not a cyber device. The SBOM at \
                 /api/compliance/sbom is kept for ISO/IEC 27001 A.8 supply-chain and \
                 configuration-management purposes.",
            ],
        }
    }
}

/// Service identification and build provenance.
///
/// `GET /api/compliance`
///
/// # Errors
///
/// None beyond response serialization.
#[debug_handler]
async fn identification() -> Result<Response> {
    format::json(Identification::current())
}

/// The `CycloneDX` SBOM and SOUP register.
///
/// `GET /api/compliance/sbom` — derived from the crate's own `Cargo.lock`
/// at compile time, so it cannot drift from the running binary.
///
/// # Errors
///
/// None beyond response serialization.
#[debug_handler]
async fn sbom() -> Result<Response> {
    format::json(soup::sbom())
}

/// Route registration.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/compliance")
        .add("/", get(identification))
        .add("/sbom", get(sbom))
}

#[cfg(test)]
mod tests {
    use super::Identification;

    /// The identification names the service and carries build provenance.
    #[test]
    fn identification_carries_provenance() {
        let id = Identification::current();
        assert_eq!(id.service, "case-service");
        assert!(!id.build.version.is_empty());
    }

    /// What is *not* claimed is stated explicitly. An assessment reading
    /// this endpoint should not have to infer the absence of a
    /// medical-device classification from its absence in the JSON.
    #[test]
    fn states_what_is_not_claimed() {
        let id = Identification::current();
        assert!(
            id.not_claimed.iter().any(|c| c.contains("IEC 62304")),
            "the absent safety classification must be stated, not merely omitted"
        );
        assert!(id.not_claimed.iter().any(|c| c.contains("524B")));
    }
}
