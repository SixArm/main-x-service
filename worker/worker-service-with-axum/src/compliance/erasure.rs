//! GDPR Art. 17 erasure that survives an immutable, tamper-evident trail.
//!
//! Ported from the care-pathway reference implementation
//! (`agents/share/compliance-for-healthcare.md` §2.2). The right to
//! erasure and an append-only audit chain pull in opposite directions:
//! honouring one by deleting rows destroys the other. Neither "delete the
//! audit trail" nor "refuse the erasure" is acceptable, so the family
//! resolves it with **redaction**:
//!
//! 1. The worker's personal data is destroyed and the record is
//!    soft-deleted — the data is gone, the identifier is not, so
//!    references from other services resolve to "erased" rather than
//!    dangling.
//! 2. Every audit row about the worker has its value snapshots destroyed
//!    and `redacted_at` stamped, while its `hash` and `prev_hash` are left
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
//! Erasure is **irreversible**: nothing retained can reconstruct the
//! payload, and no snapshot of it remains.
//!
//! ## Why this differs from the care-pathway reference
//!
//! care-pathway and case store their whole payload as one JSONB column,
//! so erasure there is a single `UPDATE` replacing it with a tombstone.
//! A worker is **relational**: names, identifiers, addresses, contacts,
//! documents, emergency contacts (and their telecom rows), photos, links,
//! match scores, and **assessments** each live in their own table, and
//! the `workers` row itself carries `gender`, `birth_date`, `tax_id`,
//! `worker_type`, `deceased_datetime`, and `marital_status`. Tombstoning
//! one column would leave the actual personal data untouched in eleven
//! others.
//!
//! `worker_assessments` is the table this service has and person does
//! not, and it is the most sensitive one here: aptitude, personality, and
//! psychometric results, with scores and score bands. An erasure that
//! swept names and addresses but left a psychometric profile keyed to the
//! worker id would miss the data a subject is most likely to be asking
//! about.
//!
//! So erasure here **deletes** the child rows outright and **scrubs** the
//! parent row's own personal fields. Deletion rather than tombstoning is
//! right for the children because, unlike the audit rows, nothing hashes
//! or links them — there is no integrity property that their absence
//! would break, and a retained-but-blanked row would still leak how many
//! addresses or identifiers a subject had.
//!
//! One name row is written back as a tombstone. The service's read paths
//! assume a worker has at least one name; a worker with none would
//! deserialize into a shape later code treats as impossible, so an erased
//! record would become a landmine rather than degrading cleanly.

use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::compliance::disclosure::AccessContext;

/// The audit action verb an erasure appends.
pub const ACTION_ERASED: &str = "erased";

/// The family name an erased worker carries. Not blank, so the row still
/// satisfies the service's own validation invariants, and unmistakable, so
/// an operator seeing it in a list cannot read it as real data.
pub const TOMBSTONE_NAME: &str = "(erased)";

/// The gender an erased worker carries.
///
/// `unknown` rather than the stored value: gender is personal data, and
/// the column is `NOT NULL`, so it must hold *something*. The schema's
/// own check constraint admits `unknown`, which is also the honest value —
/// after erasure the service genuinely does not know.
pub const TOMBSTONE_GENDER: &str = "unknown";

/// Child tables holding a worker's personal data, each keyed by
/// `worker_id`. Every row is deleted on erasure.
///
/// Listed explicitly rather than discovered from the schema so that adding
/// a table without considering erasure is a compile-time-visible omission
/// in one place, not a silent leak. `worker_match_scores` is included: a
/// score row names two worker ids and asserts they may be the same human,
/// which is itself an inference about the subject.
const CHILD_TABLES: [&str; 9] = [
    "worker_names",
    "worker_identifiers",
    "worker_addresses",
    "worker_contacts",
    "worker_documents",
    "worker_photos",
    "worker_links",
    "worker_match_scores",
    "worker_assessments",
];

/// What an erasure did — returned to the caller and worth recording,
/// because "erased 0 audit rows" is a meaningfully different outcome from
/// "erased 40".
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErasureOutcome {
    /// The erased worker's public id, which deliberately survives.
    pub id: String,
    /// Whether the record was still active when erasure ran (a
    /// soft-deleted record can still be erased — soft delete is not
    /// erasure, which is exactly why this endpoint exists).
    pub was_active: bool,
    /// How many child rows (names, identifiers, addresses, …) were
    /// destroyed.
    pub child_rows_deleted: u64,
    /// How many audit rows had their content redacted.
    pub audit_rows_redacted: u64,
    /// How many cross-service links were withdrawn.
    pub links_withdrawn: u64,
    /// Whether a `workers` row was found and scrubbed.
    pub payload_erased: bool,
    /// Restated in the response so a caller cannot mistake this for a
    /// reversible soft delete.
    pub irreversible: bool,
}

/// Delete every row hanging off a worker, returning how many went.
///
/// Split out so [`erase`] stays readable now that it also clears three
/// digest columns. The table list is the security boundary — a table
/// missing from it is personal data that survives an erasure — so it
/// lives next to the constant that defines it.
///
/// # Errors
///
/// Propagates any delete error.
async fn destroy_child_rows(db: &mut PgConnection, id: Uuid) -> crate::Result<u64> {
    let mut deleted = 0;
    for table in CHILD_TABLES {
        deleted += crate::db::erasure::delete_child_rows(db, table, id).await?;
    }
    let (telecom, contacts) = crate::db::erasure::delete_emergency_contacts(db, id).await?;
    deleted += telecom + contacts;

    Ok(deleted)
}

/// Erase one worker: destroy the child rows, scrub the parent row, write
/// back a tombstone name, withdraw cross-service links, redact the audit
/// content, and append a chained `erased` audit row.
///
/// Runs on the caller's connection or transaction; pass a
/// `&DatabaseTransaction` to make the writes atomic. **Do**: the child
/// deletes and the parent scrub are separate statements, and a failure
/// between them would leave a record with no names and un-scrubbed
/// demographics.
///
/// Idempotent: erasing an already-erased or unknown id still sweeps any
/// audit content and links held about it, because a subject's right does
/// not lapse when the record is soft-deleted.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if any statement fails.
pub async fn erase(
    db: &mut PgConnection,
    id: Uuid,
    actor: Option<&str>,
    ctx: &AccessContext,
    audit: &crate::db::AuditLogRepository,
) -> crate::Result<ErasureOutcome> {
    // Read the pre-state we report on, before destroying it.
    let existing = crate::db::erasure::worker_activity(db, id).await?;
    let was_active = existing.unwrap_or(false);
    let found = existing.is_some();

    let child_rows_deleted = destroy_child_rows(db, id).await?;

    if found {
        // 2. Scrub the parent row's own personal fields and retire it.
        //    `content_hash` is set to NULL rather than recomputed: the
        //    hash covers the *assembled* record and the child rows are
        //    deleted in step 1, so there is no longer a record to hash.
        //    NULL is the column's existing "not hashed" value, which
        //    verification reports as a gap rather than a mismatch, so an
        //    erased record does not masquerade as tampered.
        //    `gender` is NOT NULL, so it takes the honest `unknown`.
        crate::db::erasure::scrub_worker(db, id, TOMBSTONE_GENDER, actor).await?;

        // 3. Write back one tombstone name. Read paths assume a worker has
        //    at least one; leaving none would make an erased record a
        //    landmine rather than a clean degradation.
        crate::db::erasure::insert_tombstone_name(db, id, TOMBSTONE_NAME).await?;
    }

    // 4. Withdraw cross-service links. A surviving `same_identity` edge
    //    would still assert that this erased record and a worker record
    //    are the same human.
    let links_withdrawn = crate::db::erasure::withdraw_links(db, id).await?;

    // 5. Destroy the audit content, keeping the chain linkage.
    let audit_rows_redacted = audit.redact_for_entity(&mut *db, id).await?;

    // 6. Append the accountability record for the erasure itself. It is
    //    chained normally and is *not* redacted — it is the controller's
    //    own record and holds no subject data.
    audit
        .log_erasure(
            &mut *db,
            id,
            actor,
            ctx.client,
            erasure_context(
                ctx,
                audit_rows_redacted,
                child_rows_deleted,
                links_withdrawn,
            ),
            ctx.is_disclosure(),
        )
        .await?;

    Ok(ErasureOutcome {
        id: id.to_string(),
        was_active,
        child_rows_deleted,
        audit_rows_redacted,
        links_withdrawn,
        payload_erased: found,
        irreversible: true,
    })
}

/// The context JSON recorded on the `erased` row: the caller's declared
/// access context plus what the erasure actually did.
fn erasure_context(
    ctx: &AccessContext,
    redacted: u64,
    children: u64,
    links: u64,
) -> serde_json::Value {
    let mut context = ctx.to_json();
    if let Some(map) = context.as_object_mut() {
        map.insert("erasure_basis".to_string(), "gdpr_art17".into());
        map.insert("audit_rows_redacted".to_string(), redacted.into());
        map.insert("child_rows_deleted".to_string(), children.into());
        map.insert("links_withdrawn".to_string(), links.into());
        map.insert("irreversible".to_string(), true.into());
    }
    context
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tombstone name is unmistakable — an operator scanning a list
    /// must not read it as a real worker.
    #[test]
    fn tombstone_name_is_unmistakable() {
        assert!(TOMBSTONE_NAME.starts_with('('));
        assert!(TOMBSTONE_NAME.contains("erased"));
        assert!(!TOMBSTONE_NAME.trim().is_empty());
    }

    /// The tombstone gender is one the schema's check constraint admits,
    /// and is the honest value: after erasure the service does not know.
    #[test]
    fn tombstone_gender_is_a_valid_unknown() {
        assert_eq!(TOMBSTONE_GENDER, "unknown");
    }

    /// Every table naming a worker is swept, including
    /// `worker_assessments` — psychometric results are the most sensitive
    /// data this service holds. The list is the security
    /// boundary — a table missing from it is personal data that survives
    /// an erasure — so its contents are pinned rather than left to drift.
    #[test]
    fn every_worker_owned_table_is_swept() {
        for table in [
            "worker_names",
            "worker_identifiers",
            "worker_addresses",
            "worker_contacts",
            "worker_documents",
            "worker_photos",
            "worker_links",
            "worker_match_scores",
            "worker_assessments",
        ] {
            assert!(
                CHILD_TABLES.contains(&table),
                "{table} holds personal data but is not swept on erasure"
            );
        }
        // The emergency-contact pair is swept separately (they key off the
        // contact, not the worker), so they must NOT be in this list or
        // the generated `WHERE worker_id = $1` would be invalid SQL.
        assert!(!CHILD_TABLES.contains(&"worker_emergency_contact_telecom"));
        assert!(!CHILD_TABLES.contains(&"worker_emergency_contacts"));
    }

    /// The erasure context records the legal basis, the scale of the
    /// destruction, and its irreversibility — on top of the caller's own
    /// declared context.
    #[test]
    fn erasure_context_records_basis_and_scale() {
        let ctx = AccessContext::from_parts(Some("care"), None, None);
        let json = erasure_context(&ctx, 42, 7, 1);
        assert_eq!(json["erasure_basis"], "gdpr_art17");
        assert_eq!(json["audit_rows_redacted"], 42);
        assert_eq!(json["child_rows_deleted"], 7);
        assert_eq!(json["links_withdrawn"], 1);
        assert_eq!(json["irreversible"], true);
        assert_eq!(json["purpose_of_use"], "care");
    }

    /// The outcome always states irreversibility, so an API consumer
    /// cannot mistake erasure for the reversible soft delete.
    #[test]
    fn outcome_always_states_irreversibility() {
        let outcome = ErasureOutcome {
            id: Uuid::nil().to_string(),
            was_active: true,
            child_rows_deleted: 3,
            audit_rows_redacted: 0,
            links_withdrawn: 0,
            payload_erased: true,
            irreversible: true,
        };
        let json = serde_json::to_value(&outcome).expect("serialize");
        assert_eq!(json["irreversible"], true);
    }
}
