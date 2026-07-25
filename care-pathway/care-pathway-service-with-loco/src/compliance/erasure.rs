//! GDPR Art. 17 erasure that survives an immutable, tamper-evident trail.
//!
//! The right to erasure and an append-only audit chain pull in opposite
//! directions: honouring one by deleting rows destroys the other. Neither
//! "delete the audit trail" nor "refuse the erasure" is acceptable, so the
//! family resolves it with **redaction**
//! (`agents/share/compliance-for-healthcare.md` §2.2):
//!
//! 1. The entity row's payload is replaced with a **tombstone** and the
//!    record is soft-deleted — the data is gone, the identifier is not, so
//!    references from other services resolve to "erased" rather than
//!    dangling.
//! 2. Every audit row about the record has its `snapshot` destroyed and
//!    `redacted_at` stamped, while its `hash` and `prev_hash` are left
//!    intact — so [`super::audit_chain::verify`] still checks linkage
//!    across it and the chain as a whole keeps verifying.
//! 3. A final `erased` audit row is appended, chained normally, recording
//!    who erased what and when.
//!
//! What survives is the **fact** that a record existed and was erased, by
//! whom, and when — the controller's own accountability record under the
//! Art. 17(3)(b) legal-obligation carve-out — and nothing about the data
//! subject.
//!
//! Erasure is **irreversible**: the tombstone cannot reconstruct the
//! payload, and no snapshot of it remains.

use care_pathway_matcher::CarePathway;
use chrono::SubsecRound as _;
use loco_rs::prelude::ModelResult;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, IntoActiveModel};
use serde::Serialize;
use uuid::Uuid;

use crate::compliance::disclosure::AccessContext;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::care_pathways::Model as PathwayModel;

/// The audit action verb an erasure appends.
pub const ACTION_ERASED: &str = "erased";

/// The name an erased record carries. Not blank, so the row still
/// satisfies the service's own validation invariants, and unmistakable, so
/// an operator seeing it in a list cannot read it as real data.
pub const TOMBSTONE_NAME: &str = "(erased)";

/// The payload an erased record is left holding.
///
/// A structurally valid [`CarePathway`] rather than an empty object or a
/// `NULL`, so every read path that deserialises the stored payload keeps
/// working instead of erroring — an erased record degrades cleanly rather
/// than becoming a landmine.
#[must_use]
pub fn tombstone() -> CarePathway {
    CarePathway::new(TOMBSTONE_NAME)
}

/// What an erasure did — returned to the caller and worth recording,
/// because "erased 0 audit rows" is a meaningfully different outcome from
/// "erased 40".
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErasureOutcome {
    /// The erased record's public id, which deliberately survives.
    pub pid: String,
    /// Whether the record was still active when erasure ran (a
    /// soft-deleted record can still be erased — soft delete is not
    /// erasure, which is exactly why this endpoint exists).
    pub was_active: bool,
    /// How many audit rows had their content redacted.
    pub audit_rows_redacted: u64,
    /// Always `true` — the payload is replaced with the tombstone.
    pub payload_erased: bool,
    /// Restated in the response so a caller cannot mistake this for a
    /// reversible soft delete.
    pub irreversible: bool,
}

/// Erase one care pathway: tombstone the payload, soft-delete the record,
/// redact its audit content, and append a chained `erased` audit row.
///
/// Runs on the caller's connection or transaction; pass a
/// `&DatabaseTransaction` to make the three writes atomic.
///
/// # Errors
///
/// When any of the payload update, the redaction sweep, or the audit
/// append fails.
pub async fn erase<C: ConnectionTrait>(
    db: &C,
    model: PathwayModel,
    actor: Option<&str>,
    ctx: &AccessContext,
) -> ModelResult<ErasureOutcome> {
    let pid = model.pid;
    let was_active = model.active;

    // 1. Tombstone the payload and retire the record. The content hash is
    //    recomputed over the tombstone, so an erased row still verifies
    //    under `record_integrity` — erasure is a legitimate write, not a
    //    reason for the row to look tampered with.
    let payload =
        serde_json::to_value(tombstone()).map_err(|e| loco_rs::model::ModelError::Any(e.into()))?;
    let deleted_at: chrono::DateTime<chrono::FixedOffset> =
        chrono::Utc::now().trunc_subsecs(6).into();
    let mut active = model.into_active_model();
    active.content_hash = ActiveValue::set(Some(super::record_integrity::record_hash(
        &super::record_integrity::RecordInput {
            pid,
            name: TOMBSTONE_NAME,
            data: &payload,
            active: false,
            deleted_at_micros: Some(deleted_at.timestamp_micros()),
        },
    )));
    active.name = ActiveValue::set(TOMBSTONE_NAME.to_string());
    active.data = ActiveValue::set(payload);
    active.active = ActiveValue::set(false);
    active.deleted_at = ActiveValue::set(Some(deleted_at));
    active.update(db).await?;

    // 2. Destroy the audit content, keeping the chain linkage.
    let audit_rows_redacted = AuditModel::redact_for_entity(db, pid).await?;

    // 3. Append the accountability record for the erasure itself. It is
    //    chained normally and is *not* redacted — it is the controller's
    //    own record and holds no subject data.
    AuditModel::record_with_context(
        db,
        pid,
        ACTION_ERASED,
        actor,
        None,
        Some(erasure_context(ctx, audit_rows_redacted)),
        ctx.is_disclosure(),
    )
    .await?;

    Ok(ErasureOutcome {
        pid: pid.to_string(),
        was_active,
        audit_rows_redacted,
        payload_erased: true,
        irreversible: true,
    })
}

/// The context JSON recorded on the `erased` row: the caller's declared
/// access context plus what the erasure actually did.
fn erasure_context(ctx: &AccessContext, redacted: u64) -> serde_json::Value {
    let mut context = ctx.to_json();
    if let Some(map) = context.as_object_mut() {
        map.insert("erasure_basis".to_string(), "gdpr_art17".into());
        map.insert("audit_rows_redacted".to_string(), redacted.into());
        map.insert("irreversible".to_string(), true.into());
    }
    context
}

/// Erase a `pid` that has no live record — the idempotent re-erasure and
/// unknown-record path. Redacts any audit content still held about it and
/// appends the `erased` accountability row, without touching an entity
/// row that is not there.
///
/// This exists because a subject's right to erasure does not evaporate
/// once the record is soft-deleted: the audit content is still personal
/// data, and re-running an erasure must be safe.
///
/// # Errors
///
/// When the redaction sweep or the audit append fails.
pub async fn erase_audit_only<C: ConnectionTrait>(
    db: &C,
    pid: Uuid,
    actor: Option<&str>,
    ctx: &AccessContext,
) -> ModelResult<ErasureOutcome> {
    let audit_rows_redacted = AuditModel::redact_for_entity(db, pid).await?;
    AuditModel::record_with_context(
        db,
        pid,
        ACTION_ERASED,
        actor,
        None,
        Some(erasure_context(ctx, audit_rows_redacted)),
        ctx.is_disclosure(),
    )
    .await?;
    Ok(ErasureOutcome {
        pid: pid.to_string(),
        was_active: false,
        audit_rows_redacted,
        payload_erased: false,
        irreversible: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tombstone is a structurally valid pathway that passes the
    /// service's own validation, so an erased row cannot break a read path
    /// that deserialises and re-validates it.
    #[test]
    fn tombstone_is_a_valid_pathway() {
        let t = tombstone();
        assert_eq!(t.name, TOMBSTONE_NAME);
        assert!(
            crate::validation::problems(&t).is_empty(),
            "the tombstone must satisfy the service's validators"
        );
    }

    /// The tombstone carries no payload data — that is the whole point.
    #[test]
    fn tombstone_carries_no_data() {
        let t = tombstone();
        assert!(t.condition_codes.is_empty());
        assert!(t.identifiers.is_empty());
        assert!(t.alternate_names.is_empty());
        assert!(t.interventions.is_empty());
        assert!(t.keywords.is_empty());
    }

    /// The tombstone name is unmistakable — an operator scanning a list
    /// must not read it as a real pathway.
    #[test]
    fn tombstone_name_is_unmistakable() {
        assert!(TOMBSTONE_NAME.starts_with('('));
        assert!(TOMBSTONE_NAME.contains("erased"));
        assert!(!TOMBSTONE_NAME.trim().is_empty());
    }

    /// The erasure context records the legal basis, the scale of the
    /// redaction, and its irreversibility — on top of the caller's own
    /// declared context.
    #[test]
    fn erasure_context_records_basis_and_scale() {
        let ctx = AccessContext::from_parts(Some("care"), None, None);
        let json = erasure_context(&ctx, 42);
        assert_eq!(json["erasure_basis"], "gdpr_art17");
        assert_eq!(json["audit_rows_redacted"], 42);
        assert_eq!(json["irreversible"], true);
        assert_eq!(json["purpose_of_use"], "care");
    }

    /// The outcome always states irreversibility, so an API consumer
    /// cannot mistake erasure for the reversible soft delete.
    #[test]
    fn outcome_always_states_irreversibility() {
        let outcome = ErasureOutcome {
            pid: Uuid::nil().to_string(),
            was_active: true,
            audit_rows_redacted: 0,
            payload_erased: true,
            irreversible: true,
        };
        let json = serde_json::to_value(&outcome).expect("serialize");
        assert_eq!(json["irreversible"], true);
    }
}
