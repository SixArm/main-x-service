//! GDPR Art. 17 erasure that survives an immutable, tamper-evident trail.
//!
//! Ported from the care-pathway reference implementation
//! (`agents/share/compliance-for-healthcare.md` §2.2). The right to
//! erasure and an append-only audit chain pull in opposite directions:
//! honouring one by deleting rows destroys the other. Neither "delete the
//! audit trail" nor "refuse the erasure" is acceptable, so the family
//! resolves it with **redaction**:
//!
//! 1. The person's personal data is destroyed and the record is
//!    soft-deleted — the data is gone, the identifier is not, so
//!    references from other services resolve to "erased" rather than
//!    dangling.
//! 2. Every audit row about the person has its value snapshots destroyed
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
//! A person is **relational**: names, identifiers, addresses, contacts,
//! documents, emergency contacts (and their telecom rows), photos, links,
//! and match scores each live in their own table, and the `persons` row
//! itself carries `gender`, `birth_date`, `tax_id`, `deceased_datetime`,
//! and `marital_status`. Tombstoning one column would leave the actual
//! personal data untouched in ten others.
//!
//! So erasure here **deletes** the child rows outright and **scrubs** the
//! parent row's own personal fields. Deletion rather than tombstoning is
//! right for the children because, unlike the audit rows, nothing hashes
//! or links them — there is no integrity property that their absence
//! would break, and a retained-but-blanked row would still leak how many
//! addresses or identifiers a subject had.
//!
//! One name row is written back as a tombstone. The service's read paths
//! assume a person has at least one name; a person with none would
//! deserialize into a shape later code treats as impossible, so an erased
//! record would become a landmine rather than degrading cleanly.

use sea_orm::{ConnectionTrait, Statement};
use serde::Serialize;
use uuid::Uuid;

use crate::compliance::disclosure::AccessContext;

/// The audit action verb an erasure appends.
pub const ACTION_ERASED: &str = "erased";

/// The family name an erased person carries. Not blank, so the row still
/// satisfies the service's own validation invariants, and unmistakable, so
/// an operator seeing it in a list cannot read it as real data.
pub const TOMBSTONE_NAME: &str = "(erased)";

/// The gender an erased person carries.
///
/// `unknown` rather than the stored value: gender is personal data, and
/// the column is `NOT NULL`, so it must hold *something*. The schema's
/// own check constraint admits `unknown`, which is also the honest value —
/// after erasure the service genuinely does not know.
pub const TOMBSTONE_GENDER: &str = "unknown";

/// Child tables holding a person's personal data, each keyed by
/// `person_id`. Every row is deleted on erasure.
///
/// Listed explicitly rather than discovered from the schema so that adding
/// a table without considering erasure is a compile-time-visible omission
/// in one place, not a silent leak. `person_match_scores` is included: a
/// score row names two person ids and asserts they may be the same human,
/// which is itself an inference about the subject.
const CHILD_TABLES: [&str; 8] = [
    "person_names",
    "person_identifiers",
    "person_addresses",
    "person_contacts",
    "person_documents",
    "person_photos",
    "person_links",
    "person_match_scores",
];

/// What an erasure did — returned to the caller and worth recording,
/// because "erased 0 audit rows" is a meaningfully different outcome from
/// "erased 40".
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErasureOutcome {
    /// The erased person's public id, which deliberately survives.
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
    /// Whether a `persons` row was found and scrubbed.
    pub payload_erased: bool,
    /// Restated in the response so a caller cannot mistake this for a
    /// reversible soft delete.
    pub irreversible: bool,
}

/// Erase one person: destroy the child rows, scrub the parent row, write
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
pub async fn erase<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    actor: Option<&str>,
    ctx: &AccessContext,
    audit: &crate::db::AuditLogRepository,
) -> crate::Result<ErasureOutcome> {
    // Read the pre-state we report on, before destroying it.
    let existing = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT active FROM persons WHERE id = $1",
            [id.into()],
        ))
        .await?;
    let was_active = match existing.as_ref() {
        Some(row) => row.try_get::<bool>("", "active").unwrap_or(false),
        None => false,
    };
    let found = existing.is_some();

    // 1. Destroy the child rows. These *are* the personal data; nothing
    //    hashes or links them, so deletion breaks no integrity property.
    let mut child_rows_deleted = 0;
    for table in CHILD_TABLES {
        let result = db
            .execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                format!("DELETE FROM {table} WHERE person_id = $1"),
                [id.into()],
            ))
            .await?;
        child_rows_deleted += result.rows_affected();
    }
    // Emergency-contact telecom rows hang off the emergency contacts, not
    // off the person, so they are swept through their parent first.
    let telecom = db
        .execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM person_emergency_contact_telecom WHERE emergency_contact_id IN \
             (SELECT id FROM person_emergency_contacts WHERE person_id = $1)",
            [id.into()],
        ))
        .await?;
    child_rows_deleted += telecom.rows_affected();
    let contacts = db
        .execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM person_emergency_contacts WHERE person_id = $1",
            [id.into()],
        ))
        .await?;
    child_rows_deleted += contacts.rows_affected();

    if found {
        // 2. Scrub the parent row's own personal fields and retire it.
        //    `gender` is NOT NULL, so it takes the honest `unknown`.
        //
        //    `content_hash` is set to NULL rather than recomputed. Erasure
        //    is the one write that cannot leave a verifiable digest: the
        //    hash covers the *assembled* record, and the child rows are
        //    deleted in step 1 above, so there is no longer a record to
        //    hash — recomputing one here would mean re-reading the
        //    half-destroyed state mid-transaction and certifying it.
        //    NULL is the column's existing "not hashed" value, which
        //    verification already reports as a gap rather than a mismatch,
        //    so an erased record does not masquerade as tampered. The
        //    erasure itself is accounted for by the chained `erased` audit
        //    row, which is the stronger evidence anyway.
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE persons SET \
               gender = $2, birth_date = NULL, tax_id = NULL, \
               deceased = FALSE, deceased_datetime = NULL, \
               marital_status = NULL, multiple_birth = NULL, \
               managing_organization_id = NULL, \
               created_by = NULL, updated_by = NULL, \
               content_hash = NULL, \
               active = FALSE, deleted_at = NOW(), deleted_by = $3, \
               updated_at = NOW() \
             WHERE id = $1",
            [id.into(), TOMBSTONE_GENDER.into(), actor.into()],
        ))
        .await?;

        // 3. Write back one tombstone name. Read paths assume a person has
        //    at least one; leaving none would make an erased record a
        //    landmine rather than a clean degradation.
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO person_names (id, person_id, use_type, family, given, prefix, suffix) \
             VALUES ($1, $2, NULL, $3, ARRAY[]::text[], ARRAY[]::text[], ARRAY[]::text[])",
            [Uuid::new_v4().into(), id.into(), TOMBSTONE_NAME.into()],
        ))
        .await?;
    }

    // 4. Withdraw cross-service links. A surviving `same_identity` edge
    //    would still assert that this erased record and a worker record
    //    are the same human.
    let links = db
        .execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE entity_links SET deleted_at = NOW() \
             WHERE from_pid = $1 AND deleted_at IS NULL",
            [id.into()],
        ))
        .await?;
    let links_withdrawn = links.rows_affected();

    // 5. Destroy the audit content, keeping the chain linkage.
    let audit_rows_redacted = audit.redact_for_entity(db, id).await?;

    // 6. Append the accountability record for the erasure itself. It is
    //    chained normally and is *not* redacted — it is the controller's
    //    own record and holds no subject data.
    audit
        .log_erasure(
            db,
            id,
            actor,
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
    /// must not read it as a real person.
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

    /// Every table naming a person is swept. The list is the security
    /// boundary — a table missing from it is personal data that survives
    /// an erasure — so its contents are pinned rather than left to drift.
    #[test]
    fn every_person_owned_table_is_swept() {
        for table in [
            "person_names",
            "person_identifiers",
            "person_addresses",
            "person_contacts",
            "person_documents",
            "person_photos",
            "person_links",
            "person_match_scores",
        ] {
            assert!(
                CHILD_TABLES.contains(&table),
                "{table} holds personal data but is not swept on erasure"
            );
        }
        // The emergency-contact pair is swept separately (they key off the
        // contact, not the person), so they must NOT be in this list or
        // the generated `WHERE person_id = $1` would be invalid SQL.
        assert!(!CHILD_TABLES.contains(&"person_emergency_contact_telecom"));
        assert!(!CHILD_TABLES.contains(&"person_emergency_contacts"));
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
