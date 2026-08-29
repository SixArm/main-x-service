//! Integrity-verification endpoint for the audit trail.
//!
//! `GET /api/compliance/audit/verify` recomputes each row's SHA-256,
//! SHA-3, and MAC over authentication events, naming any row whose content was
//! altered.
//!
//! The unkeyed digests are checked too, not just the MAC. They are
//! written even when no key is configured, so on a default deployment
//! they are the only integrity these rows have — verifying only the MAC
//! would report such a deployment as entirely unverified when it is not.
//!
//! ## Read the caveat in the response
//!
//! A `verified: true` attests that no examined row's **content** was
//! altered. It does **not** attest that no row was deleted: nothing in a
//! row can prove its own continued existence. The response carries that
//! caveat inline rather than leaving it to documentation, because a bare
//! `verified: true` reads as more than it means.
//!
//! ## Authorization — requires a bearer token (spec §16, decided)
//!
//! Unlike the equivalent endpoint on sibling loco-idiomatic services
//! (e.g. case-service, behind its blanket `CASE_REQUIRE_AUTH` guard),
//! this crate has **no blanket `/api/*` guard at all** — every other
//! route here is gated per-handler (session cookie, PASETO bearer, or
//! the admin handler's own `access=admin` check).
//!
//! This handler is gated on the same basis: it requires a valid PASETO
//! bearer ([`AuthUser`]), `401` without one. **Not** admin-gated — the
//! response discloses no PII (row counts and row ids only, never an
//! email), so the elevated `access=admin` bar `recent_audit` applies for
//! its email-carrying rows would be stricter than the disclosure
//! warrants.
//!
//! The reason to gate at all is **not** disclosure — it is cost. The
//! handler does real work on every call: a DB read of up to
//! [`VERIFY_MAX_LIMIT`] rows, then three digest recomputations
//! (SHA-256, SHA-3, HMAC) per row. That is CPU + DB load an anonymous
//! caller could trigger repeatedly at no cost to themselves — an
//! unauthenticated denial-of-service surface even though the boolean
//! `verified` output itself is harmless. Requiring a bearer removes the
//! anonymous-internet version of that abuse, matching how every other
//! bearer-gated route in this crate is protected (no dedicated rate
//! limit beyond authentication). See `spec/index.md` §16 for the full
//! reasoning and the rejected alternatives.

use axum::extract::Query;
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::compliance::audit_integrity;
use crate::models::_entities::auth_events;

/// Default rows examined in one call.
pub const VERIFY_DEFAULT_LIMIT: u64 = 1000;

/// Hard cap on rows examined in one call.
///
/// Verification is O(rows) with three digests per row, so an unbounded
/// `limit` is a CPU denial-of-service on a large trail (the SEC-M1
/// bound-every-input invariant).
pub const VERIFY_MAX_LIMIT: u64 = 10_000;

/// Query string for the verification endpoint.
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

/// Verify audit-row integrity. Requires a valid PASETO bearer (any
/// authenticated caller — not admin-gated; see the module docs).
///
/// # Errors
///
/// When the query fails.
#[debug_handler]
async fn verify_audit(
    _auth: AuthUser,
    State(ctx): State<AppContext>,
    Query(params): Query<VerifyParams>,
) -> Result<Response> {
    let rows = auth_events::Entity::find()
        .order_by_desc(auth_events::Column::Id)
        .limit(params.limit())
        .all(&ctx.db)
        .await?;
    format::json(audit_integrity::verify(&rows))
}

/// Route registration.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/compliance")
        .add("/audit/verify", get(verify_audit))
}

#[cfg(test)]
mod tests {
    use super::{VERIFY_DEFAULT_LIMIT, VERIFY_MAX_LIMIT, VerifyParams};

    /// The limit is clamped at both ends. Verification recomputes three
    /// digests per row, so an unbounded limit can pin a core.
    #[test]
    fn the_limit_is_clamped_at_both_ends() {
        assert_eq!(VerifyParams { limit: None }.limit(), VERIFY_DEFAULT_LIMIT);
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
