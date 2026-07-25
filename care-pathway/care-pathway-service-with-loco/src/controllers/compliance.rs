//! Compliance evidence endpoints — the runtime surface an assessment
//! reads instead of reconstructing the deployment's configuration by hand.
//!
//! | Endpoint | Evidence |
//! |---|---|
//! | `GET /api/compliance` | Software identification, build provenance, IEC 62304 safety classification, which controls are **actually live**, the declared data-protection posture, and — deliberately — what each framework is **not** claimed to satisfy. |
//! | `GET /api/compliance/sbom` | `CycloneDX` 1.5 SBOM + SOUP register (IEC 62304 §8.1.2, FD&C §524B). |
//! | `GET /api/compliance/audit/verify` | Tamper-evidence: re-verifies the audit hash chain and reports every break (HIPAA §164.312(c)). |
//! | `GET /api/compliance/records/verify` | Row-level integrity: recomputes each record's content hash and names any row changed outside the service. |
//!
//! These sit under `/api/*`, so they are behind the blanket auth + ABAC
//! guard when `CARE_PATHWAY_REQUIRE_AUTH` is on. They are **reads**, so
//! the default ABAC policy admits any authenticated caller — appropriate,
//! since the whole point is that an auditor can read the posture, and
//! nothing here discloses pathway data.

use axum::extract::Query;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::compliance::{Posture, soup};
use crate::models::audit_logs::Model as AuditModel;
use crate::models::care_pathways::Model as PathwayModel;

/// Default number of trailing audit rows verified in one call.
pub const VERIFY_DEFAULT_LIMIT: u64 = 1000;

/// Hard cap on rows verified in one call. Verification is O(rows) with a
/// SHA-256 per row, so an unbounded `limit` is a CPU denial-of-service on
/// a large trail (the SEC-M1 bound-every-input invariant).
pub const VERIFY_MAX_LIMIT: u64 = 10_000;

/// Query string for the chain-verification endpoint.
#[derive(Debug, Deserialize)]
struct VerifyParams {
    /// How many trailing rows to verify; clamped to [`VERIFY_MAX_LIMIT`].
    limit: Option<u64>,
}

impl VerifyParams {
    /// The effective row limit: the default when absent, clamped to the
    /// cap, and never zero (a zero-row verification is vacuously "clean"
    /// and would be a misleading answer).
    fn limit(&self) -> u64 {
        self.limit
            .unwrap_or(VERIFY_DEFAULT_LIMIT)
            .clamp(1, VERIFY_MAX_LIMIT)
    }
}

/// What a clean verification does — and does **not** — attest to.
const INTERPRETATION_CLEAN: &str = "no break detected in the verified window; this attests to the audit trail only, \
     not to the care_pathways rows";

/// What a break means, so an operator does not have to find the
/// specification before acting on one.
const INTERPRETATION_BROKEN: &str = "a break means rows were inserted, deleted, reordered, or edited since they were written \
     — investigate the named ids; under CARE_PATHWAY_EVENT_TRANSPORT=memory a linkage break \
     may instead mean two concurrent audit writes raced";

/// The chain-verification response.
#[derive(Debug, Serialize)]
struct VerifyResponse {
    /// Rows requested for verification.
    limit: u64,
    /// The verification report.
    #[serde(flatten)]
    report: crate::compliance::audit_chain::ChainReport,
    /// What a failure means, so an operator reading a break does not have
    /// to go and find the specification first.
    interpretation: &'static str,
}

/// The declared compliance posture.
///
/// `GET /api/compliance`.
///
/// # Errors
///
/// None beyond response serialization.
#[debug_handler]
async fn posture() -> Result<Response> {
    format::json(Posture::current())
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

/// Verify the audit hash chain.
///
/// `GET /api/compliance/audit/verify?limit=1000` — recomputes the trailing
/// `limit` rows and reports every linkage or content break. A `verified:
/// true` response is positive evidence that the trail has not been
/// rewritten over that window; a break names the row where the evidence
/// fails.
///
/// # Errors
///
/// Propagates DB query errors.
#[debug_handler]
async fn verify_audit_chain(
    Query(params): Query<VerifyParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let limit = params.limit();
    let report = AuditModel::verify_chain(&ctx.db, limit).await?;
    format::json(VerifyResponse {
        limit,
        interpretation: if report.verified {
            INTERPRETATION_CLEAN
        } else {
            INTERPRETATION_BROKEN
        },
        report,
    })
}

/// What a clean record verification does — and does **not** — attest to.
const RECORDS_CLEAN: &str = "no record was modified outside the service in the verified window; this attests to row \
     content, not to rows that were deleted outright — the audit chain covers those";

/// What a record mismatch means.
const RECORDS_MISMATCHED: &str = "a mismatch means the row was changed without going through the service — investigate the \
     named records; a legitimate write always rehashes";

/// The record-verification response.
#[derive(Debug, Serialize)]
struct RecordVerifyResponse {
    /// Records requested for verification.
    limit: u64,
    /// The verification report.
    #[serde(flatten)]
    report: crate::compliance::record_integrity::RecordIntegrityReport,
    /// What the result means, so an operator need not find the spec first.
    interpretation: &'static str,
}

/// Verify row-level record integrity.
///
/// `GET /api/compliance/records/verify?limit=1000` — recomputes each
/// record's content hash and reports every row that was changed outside
/// the service. Complements the audit-chain check: that one proves the
/// trail was not rewritten, this one proves the records were not.
///
/// Soft-deleted and erased rows are included deliberately — a retired row
/// is where an edit is least likely to be noticed.
///
/// # Errors
///
/// Propagates DB query errors.
#[debug_handler]
async fn verify_records(
    Query(params): Query<VerifyParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let limit = params.limit();
    let report = PathwayModel::verify_records(&ctx.db, limit).await?;
    format::json(RecordVerifyResponse {
        limit,
        interpretation: if report.verified {
            RECORDS_CLEAN
        } else {
            RECORDS_MISMATCHED
        },
        report,
    })
}

/// Compliance-evidence routes, mounted under `/api/compliance`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/compliance")
        .add("/", get(posture))
        .add("/sbom", get(sbom))
        .add("/audit/verify", get(verify_audit_chain))
        .add("/records/verify", get(verify_records))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The verification limit is defaulted, capped, and never zero.
    #[test]
    fn verify_limit_is_bounded() {
        assert_eq!(VerifyParams { limit: None }.limit(), VERIFY_DEFAULT_LIMIT);
        assert_eq!(VerifyParams { limit: Some(0) }.limit(), 1);
        assert_eq!(VerifyParams { limit: Some(50) }.limit(), 50);
        assert_eq!(
            VerifyParams {
                limit: Some(u64::MAX)
            }
            .limit(),
            VERIFY_MAX_LIMIT
        );
    }

    /// The posture endpoint's body serializes and carries the four
    /// framework statuses plus the honest control state.
    #[test]
    fn posture_serializes_with_frameworks_and_controls() {
        let json = serde_json::to_value(Posture::current()).expect("serialize");
        assert_eq!(json["service"], "care-pathway-service");
        assert_eq!(json["frameworks"].as_array().map(Vec::len), Some(4));
        assert!(json["controls"]["audit_chain"].as_bool().unwrap_or(false));
        assert!(
            json["safety_rationale"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
    }

    /// A clean verification must state the chain's **scope limit** rather
    /// than implying the entity rows were verified too — the honest-limit
    /// rule from the entity spec §12.5.
    #[test]
    fn clean_interpretation_states_the_scope_limit() {
        assert!(INTERPRETATION_CLEAN.contains("audit trail only"));
        assert!(INTERPRETATION_CLEAN.contains("care_pathways"));
    }

    /// A break must name both plausible causes — tampering, and the
    /// documented `memory`-transport append race — so an operator is not
    /// sent chasing an intrusion that was a concurrency artefact.
    #[test]
    fn break_interpretation_names_both_causes() {
        assert!(INTERPRETATION_BROKEN.contains("deleted"));
        assert!(INTERPRETATION_BROKEN.contains("memory"));
        assert!(INTERPRETATION_BROKEN.contains("raced"));
        assert_ne!(INTERPRETATION_CLEAN, INTERPRETATION_BROKEN);
    }

    /// The verification response serializes with the report flattened, so
    /// a client reads `verified` at the top level rather than nested.
    #[test]
    fn verify_response_flattens_the_report() {
        let response = VerifyResponse {
            limit: 10,
            report: crate::compliance::audit_chain::verify(&[]),
            interpretation: INTERPRETATION_CLEAN,
        };
        let json = serde_json::to_value(&response).expect("serialize");
        assert_eq!(json["limit"], 10);
        assert_eq!(json["verified"], true);
        assert_eq!(json["rows"], 0);
    }
}
