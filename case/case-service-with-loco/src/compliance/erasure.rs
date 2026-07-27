//! GDPR Art. 17 erasure that survives an immutable, tamper-evident trail.
//!
//! Ported from the care-pathway reference implementation
//! (`agents/share/compliance-for-healthcare.md` §2.2). The right to
//! erasure and an append-only audit chain pull in opposite directions:
//! honouring one by deleting rows destroys the other. Neither "delete the
//! audit trail" nor "refuse the erasure" is acceptable, so the family
//! resolves it with **redaction**:
//!
//! 1. The case row's payload is replaced with a **tombstone** and the
//!    record is soft-deleted — the data is gone, the identifier is not, so
//!    references from other services resolve to "erased" rather than
//!    dangling.
//! 2. Every audit row about the case has its `snapshot` destroyed and
//!    `redacted_at` stamped, while its `hash` and `prev_hash` are left
//!    intact — so [`super::audit_chain::verify`] still checks linkage
//!    across it and the chain as a whole keeps verifying.
//! 3. A final `erased` audit row is appended, chained normally, recording
//!    who erased what and when.
//!
//! What survives is the **fact** that a case existed and was erased, by
//! whom, and when — the controller's own accountability record under the
//! Art. 17(3)(b) legal-obligation carve-out — and nothing about the data
//! subject.
//!
//! Erasure is **irreversible**: the tombstone cannot reconstruct the
//! payload, and no snapshot of it remains.
//!
//! ## Why this matters more for a case than for a pathway
//!
//! A care pathway is a clinical *template*; a case names a person and
//! asserts they are the subject of a benefits, legal, or investigative
//! proceeding. The `subject_of` edge to a person is the family's
//! highest-governance link (`agents/share/cross-service-linking.md` §10),
//! and an audit trail of who read that case is itself sensitive. Erasure
//! here therefore also sweeps the case's **cross-service links**: leaving
//! a `subject_of` edge behind would preserve exactly the assertion the
//! subject asked to have erased, even with the case payload gone.

use case_matcher::Case;
use chrono::SubsecRound as _;
use loco_rs::prelude::ModelResult;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, IntoActiveModel};
use serde::Serialize;
use uuid::Uuid;

use crate::compliance::disclosure::AccessContext;
use crate::models::_entities::cases::Model as CaseModel;
use crate::models::audit_logs::Model as AuditModel;

/// The audit action verb an erasure appends.
pub const ACTION_ERASED: &str = "erased";

/// The title an erased case carries. Not blank, so the row still
/// satisfies the service's own validation invariants, and unmistakable, so
/// an operator seeing it in a list cannot read it as real data.
pub const TOMBSTONE_TITLE: &str = "(erased)";

/// The payload an erased case is left holding.
///
/// A structurally valid [`Case`] rather than an empty object or a `NULL`,
/// so every read path that deserialises the stored payload keeps working
/// instead of erroring — an erased record degrades cleanly rather than
/// becoming a landmine.
#[must_use]
pub fn tombstone() -> Case {
    Case::new(TOMBSTONE_TITLE)
}

/// What an erasure did — returned to the caller and worth recording,
/// because "erased 0 audit rows" is a meaningfully different outcome from
/// "erased 40".
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErasureOutcome {
    /// The erased case's public id, which deliberately survives.
    pub pid: String,
    /// Whether the case was still active when erasure ran (a soft-deleted
    /// case can still be erased — soft delete is not erasure, which is
    /// exactly why this endpoint exists).
    pub was_active: bool,
    /// How many audit rows had their content redacted.
    pub audit_rows_redacted: u64,
    /// How many cross-service links were withdrawn. Counted separately
    /// because a surviving `subject_of` edge would preserve the very
    /// assertion the subject asked to erase.
    pub links_withdrawn: u64,
    /// Always `true` — the payload is replaced with the tombstone.
    pub payload_erased: bool,
    /// Restated in the response so a caller cannot mistake this for a
    /// reversible soft delete.
    pub irreversible: bool,
}

/// Erase one case: tombstone the payload, soft-delete the record, withdraw
/// its cross-service links, redact its audit content, and append a chained
/// `erased` audit row.
///
/// Runs on the caller's connection or transaction; pass a
/// `&DatabaseTransaction` to make the writes atomic.
///
/// # Errors
///
/// When any of the payload update, the link withdrawal, the redaction
/// sweep, or the audit append fails.
pub async fn erase<C: ConnectionTrait>(
    db: &C,
    model: CaseModel,
    actor: Option<&str>,
    ctx: &AccessContext,
) -> ModelResult<ErasureOutcome> {
    let pid = model.pid;
    let was_active = model.active;

    // 1. Tombstone the payload and retire the record.
    let payload =
        serde_json::to_value(tombstone()).map_err(|e| loco_rs::model::ModelError::Any(e.into()))?;
    let deleted_at: chrono::DateTime<chrono::FixedOffset> =
        chrono::Utc::now().trunc_subsecs(6).into();
    let mut active = model.into_active_model();
    // The tombstone is recomputed into the digest, not cleared. This
    // service stores its whole payload in one JSONB column, so an erased
    // record is still a *complete* record and can be hashed — an erased
    // case therefore keeps verifying rather than dropping into the
    // `unhashed` bucket. (person and worker null theirs instead, because
    // their child rows are deleted by then and no assembled record
    // remains to hash.) Erasure is a legitimate write, not a reason for
    // the row to look tampered with.
    let d = super::record_integrity::digests(&super::record_integrity::RecordInput {
        pid,
        title: TOMBSTONE_TITLE,
        data: &payload,
        active: false,
        deleted_at_micros: Some(deleted_at.timestamp_micros()),
    });
    active.content_hash = ActiveValue::set(Some(d.sha256));
    active.content_hash_sha3 = ActiveValue::set(Some(d.sha3));
    active.content_mac = ActiveValue::set(d.mac);
    active.title = ActiveValue::set(TOMBSTONE_TITLE.to_string());
    active.data = ActiveValue::set(payload);
    active.active = ActiveValue::set(false);
    active.deleted_at = ActiveValue::set(Some(deleted_at));
    active.update(db).await?;

    // 2. Withdraw the cross-service links. A `subject_of` edge asserts
    //    that a named person is the subject of this case — the assertion
    //    the erasure exists to remove — so tombstoning the payload while
    //    leaving the edge would erase the details and keep the accusation.
    let links_withdrawn = withdraw_links(db, pid).await?;

    // 3. Destroy the audit content, keeping the chain linkage.
    let audit_rows_redacted = AuditModel::redact_for_entity(db, pid).await?;

    // 4. Append the accountability record for the erasure itself. It is
    //    chained normally and is *not* redacted — it is the controller's
    //    own record and holds no subject data.
    AuditModel::record_with_context(
        db,
        pid,
        ACTION_ERASED,
        actor,
        None,
        Some(erasure_context(ctx, audit_rows_redacted, links_withdrawn)),
        ctx.is_disclosure(),
    )
    .await?;

    Ok(ErasureOutcome {
        pid: pid.to_string(),
        was_active,
        audit_rows_redacted,
        links_withdrawn,
        payload_erased: true,
        irreversible: true,
    })
}

/// Soft-delete every outbound cross-service link for a case, returning how
/// many were withdrawn.
///
/// Soft delete rather than `DELETE`: the link aggregator reconciles
/// against this table (`agents/share/cross-service-linking.md` §8), and a
/// row that vanishes without trace is indistinguishable from one that was
/// never written, which would let a dropped event resurrect the edge.
async fn withdraw_links<C: ConnectionTrait>(db: &C, pid: Uuid) -> ModelResult<u64> {
    let result = db
        .execute(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE entity_links SET deleted_at = NOW() \
             WHERE from_pid = $1 AND deleted_at IS NULL",
            [pid.into()],
        ))
        .await?;
    Ok(result.rows_affected())
}

/// The context JSON recorded on the `erased` row: the caller's declared
/// access context plus what the erasure actually did.
fn erasure_context(ctx: &AccessContext, redacted: u64, links: u64) -> serde_json::Value {
    let mut context = ctx.to_json();
    if let Some(map) = context.as_object_mut() {
        map.insert("erasure_basis".to_string(), "gdpr_art17".into());
        map.insert("audit_rows_redacted".to_string(), redacted.into());
        map.insert("links_withdrawn".to_string(), links.into());
        map.insert("irreversible".to_string(), true.into());
    }
    context
}

/// Erase a `pid` that has no live record — the idempotent re-erasure and
/// unknown-record path. Redacts any audit content still held about it,
/// withdraws any surviving links, and appends the `erased` accountability
/// row, without touching an entity row that is not there.
///
/// This exists because a subject's right to erasure does not evaporate
/// once the record is soft-deleted: the audit content is still personal
/// data, and re-running an erasure must be safe.
///
/// # Errors
///
/// When the link withdrawal, the redaction sweep, or the audit append
/// fails.
pub async fn erase_audit_only<C: ConnectionTrait>(
    db: &C,
    pid: Uuid,
    actor: Option<&str>,
    ctx: &AccessContext,
) -> ModelResult<ErasureOutcome> {
    let links_withdrawn = withdraw_links(db, pid).await?;
    let audit_rows_redacted = AuditModel::redact_for_entity(db, pid).await?;
    AuditModel::record_with_context(
        db,
        pid,
        ACTION_ERASED,
        actor,
        None,
        Some(erasure_context(ctx, audit_rows_redacted, links_withdrawn)),
        ctx.is_disclosure(),
    )
    .await?;
    Ok(ErasureOutcome {
        pid: pid.to_string(),
        was_active: false,
        audit_rows_redacted,
        links_withdrawn,
        payload_erased: false,
        irreversible: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tombstone is a structurally valid case that passes the
    /// service's own validation, so an erased row cannot break a read path
    /// that deserialises and re-validates it.
    #[test]
    fn tombstone_is_a_valid_case() {
        let t = tombstone();
        assert_eq!(t.title, TOMBSTONE_TITLE);
        assert!(
            crate::validation::problems(&t).is_empty(),
            "the tombstone must satisfy the service's validators"
        );
    }

    /// The tombstone carries no payload data — that is the whole point.
    #[test]
    fn tombstone_carries_no_data() {
        let t = tombstone();
        assert!(t.identifiers.is_empty());
        assert!(t.subjects.is_empty());
        assert!(t.keywords.is_empty());
        assert!(t.case_number.is_none());
        assert!(t.agency_id.is_none());
        assert!(t.agency_name.is_none());
        assert!(t.alternate_titles.is_empty());
    }

    /// The tombstone title is unmistakable — an operator scanning a list
    /// must not read it as a real case.
    #[test]
    fn tombstone_title_is_unmistakable() {
        assert!(TOMBSTONE_TITLE.starts_with('('));
        assert!(TOMBSTONE_TITLE.contains("erased"));
        assert!(!TOMBSTONE_TITLE.trim().is_empty());
    }

    /// The erasure context records the legal basis, the scale of the
    /// redaction, the link withdrawal, and its irreversibility — on top of
    /// the caller's own declared context.
    #[test]
    fn erasure_context_records_basis_and_scale() {
        let ctx = AccessContext::from_parts(Some("legal"), None, None);
        let json = erasure_context(&ctx, 42, 3);
        assert_eq!(json["erasure_basis"], "gdpr_art17");
        assert_eq!(json["audit_rows_redacted"], 42);
        assert_eq!(json["links_withdrawn"], 3);
        assert_eq!(json["irreversible"], true);
        assert_eq!(json["purpose_of_use"], "legal");
    }

    /// The outcome always states irreversibility, so an API consumer
    /// cannot mistake erasure for the reversible soft delete.
    #[test]
    fn outcome_always_states_irreversibility() {
        let outcome = ErasureOutcome {
            pid: Uuid::nil().to_string(),
            was_active: true,
            audit_rows_redacted: 0,
            links_withdrawn: 0,
            payload_erased: true,
            irreversible: true,
        };
        let json = serde_json::to_value(&outcome).expect("serialize");
        assert_eq!(json["irreversible"], true);
    }
}
