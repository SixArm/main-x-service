//! Integrity-verification endpoints.
//!
//! | Endpoint | What it checks |
//! |---|---|
//! | `GET /api/compliance/records/verify` | Recomputes each organization's three digests and names any row that differs. |
//! | `GET /api/compliance/audit/verify` | Recomputes each audit row's MAC and names any row whose content was altered. |
//!
//! Both sit under `/api/*`, so they are behind the blanket auth + ABAC
//! guard when `ORGANIZATION_REQUIRE_AUTH` is on. They are reads, so the
//! default policy admits any authenticated caller — appropriate, since
//! the point is that an auditor can check the posture, and neither
//! response discloses organization data beyond ids and names.
//!
//! ## Read the caveat in the response
//!
//! A `verified: true` from the audit endpoint attests that no examined
//! row's *content* was altered without the key. It does **not** attest
//! that no row was deleted — nothing in a row can prove its own continued
//! existence. Detecting deletion needs a hash chain plus external-witness
//! checkpoints, which this service does not yet have. The response
//! carries that caveat inline rather than leaving it to documentation,
//! because a bare `verified: true` reads as more than it means.

use axum::extract::Query;
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect};
use serde::Deserialize;

use crate::compliance::{audit_integrity, record_integrity};
use crate::models::_entities::{audit_logs, organizations};

/// Default rows examined in one call.
pub const VERIFY_DEFAULT_LIMIT: u64 = 1000;

/// Hard cap on rows examined in one call.
///
/// Verification is O(rows) with several digests per row, so an unbounded
/// `limit` is a CPU denial-of-service on a large table (the SEC-M1
/// bound-every-input invariant).
pub const VERIFY_MAX_LIMIT: u64 = 10_000;

/// Query string for both verification endpoints.
#[derive(Debug, Deserialize)]
struct VerifyParams {
    /// How many rows to verify; clamped to [`VERIFY_MAX_LIMIT`].
    limit: Option<u64>,
}

impl VerifyParams {
    /// The clamped limit.
    fn limit(&self) -> u64 {
        self.limit
            .unwrap_or(VERIFY_DEFAULT_LIMIT)
            .clamp(1, VERIFY_MAX_LIMIT)
    }
}

/// Verify row-level record integrity.
///
/// `GET /api/compliance/records/verify?limit=1000`
///
/// # Errors
///
/// When the query fails.
#[debug_handler]
async fn verify_records(
    State(ctx): State<AppContext>,
    Query(params): Query<VerifyParams>,
) -> Result<Response> {
    let rows = organizations::Entity::find()
        .order_by_desc(organizations::Column::UpdatedAt)
        .limit(params.limit())
        .all(&ctx.db)
        .await?;
    format::json(record_integrity::verify(&rows))
}

/// Verify audit-row integrity.
///
/// `GET /api/compliance/audit/verify?limit=1000`
///
/// # Errors
///
/// When the query fails.
#[debug_handler]
async fn verify_audit(
    State(ctx): State<AppContext>,
    Query(params): Query<VerifyParams>,
) -> Result<Response> {
    let rows = audit_logs::Entity::find()
        .order_by_desc(audit_logs::Column::Id)
        .limit(params.limit())
        .all(&ctx.db)
        .await?;
    format::json(audit_integrity::verify(&rows))
}

/// Route registration.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/compliance")
        .add("/records/verify", get(verify_records))
        .add("/audit/verify", get(verify_audit))
}

#[cfg(test)]
mod tests {
    use super::{VERIFY_DEFAULT_LIMIT, VERIFY_MAX_LIMIT, VerifyParams};

    /// The limit is clamped at both ends. An unbounded limit is a CPU
    /// denial-of-service: verification recomputes several digests per
    /// row, so a caller asking for everything can pin a core.
    #[test]
    fn the_limit_is_clamped_at_both_ends() {
        assert_eq!(
            VerifyParams { limit: None }.limit(),
            VERIFY_DEFAULT_LIMIT,
            "an absent limit uses the default"
        );
        assert_eq!(VerifyParams { limit: Some(0) }.limit(), 1, "zero is raised");
        assert_eq!(
            VerifyParams {
                limit: Some(u64::MAX)
            }
            .limit(),
            VERIFY_MAX_LIMIT,
            "an enormous limit is capped"
        );
        assert_eq!(VerifyParams { limit: Some(42) }.limit(), 42);
    }
}
