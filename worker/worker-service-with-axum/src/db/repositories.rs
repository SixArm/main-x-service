//! Repository layer translating between domain [`Worker`]s and their rows.
//!
//! [`WorkerRepository`] is the async trait the API depends on;
//! [`PgWorkerRepository`] is the `PostgreSQL` implementation over sqlx. The
//! implementation owns the mapping in both directions (`insert_children` /
//! `from_db_models`), runs multi-table writes in a transaction, and — when
//! configured — publishes [`crate::streaming::WorkerEvent`]s and writes audit
//! entries on every mutation. [`AuditContext`] carries the actor metadata
//! recorded alongside those audit entries.
//!
//! Every statement against the worker tables is here, as a `query!` /
//! `query_as!` / `query_scalar!` literal, including the record-integrity
//! reads behind `/api/records/verify` and the `integrity_resign` task.
//!
//! ## Record digests (INT-V1)
//!
//! Each write stamps the record's digests **over the record as stored**,
//! read back inside the write's own transaction ([`stamp_digests`]), not
//! over the value the caller sent. The two differ: storage drops an
//! address's `use_type` and truncates a time of death to microseconds, so
//! a digest over the caller's value made a fresh record fail
//! `/api/records/verify`.

use chrono::Utc;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::{
    AddressRow, ContactRow, DocumentRow, EmergencyContactRow, EmergencyTelecomRow, IdentifierRow,
    LinkRow, NameRow, WorkerRow,
};
use super::{not_updated, to_i64};
use crate::Result;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Address, ContactPoint, ContactPointSystem, DocumentType, EmergencyContact, HumanName,
    Identifier, IdentityDocument, Worker, WorkerLink, WorkerType,
};
use crate::streaming::{EventKind, EventTransport};

/// Serialize a fieldless enum to its canonical serde string tag.
///
/// Used to store domain enums (document type, contact-point system, etc.) as
/// their externally-tagged string form in a single text column. Returns `None`
/// if the value does not serialize to a bare JSON string (i.e. it is not a
/// fieldless/unit enum variant).
fn enum_to_tag<T: serde::Serialize>(value: &T) -> Option<String> {
    // Round-trip through serde_json so the stored tag matches the enum's
    // `#[serde(rename = ...)]` representation exactly.
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

/// Parse a stored string tag back into a fieldless enum, or `None`.
///
/// Inverse of [`enum_to_tag`]: wraps the stored tag as a JSON string and
/// deserializes it. Returns `None` for a missing tag or one that no longer
/// maps to a known variant.
fn tag_to_enum<T: serde::de::DeserializeOwned>(tag: Option<&str>) -> Option<T> {
    let t = tag?;
    serde_json::from_value(serde_json::Value::String(t.to_string())).ok()
}

/// The `worker_links.link_type` token for `link_type`, as the column's
/// CHECK constraint spells it (`replaced_by`, `replaces`, `refer`,
/// `seealso`).
///
/// Not [`enum_to_tag`]: the wire token for [`LinkType::ReplacedBy`] is
/// `replacedby` (`rename_all = "lowercase"`), and changing it would change
/// the JSON API. The column has its own spelling, so it gets its own map.
fn link_type_tag(link_type: &crate::models::LinkType) -> &'static str {
    use crate::models::LinkType;
    match link_type {
        LinkType::ReplacedBy => "replaced_by",
        LinkType::Replaces => "replaces",
        LinkType::Refer => "refer",
        LinkType::Seealso => "seealso",
    }
}

/// Inverse of [`link_type_tag`]. Case-insensitive, and takes the wire
/// spelling `replacedby` too, so a row from a schema without the CHECK
/// constraint still reads back.
fn link_type_from_tag(tag: &str) -> Option<crate::models::LinkType> {
    use crate::models::LinkType;
    match tag.to_ascii_lowercase().as_str() {
        "replaced_by" | "replacedby" => Some(LinkType::ReplacedBy),
        "replaces" => Some(LinkType::Replaces),
        "refer" => Some(LinkType::Refer),
        "seealso" => Some(LinkType::Seealso),
        _ => None,
    }
}

/// A stored lowercase tag parsed into its enum, ignoring case, so a row
/// written in the old `Debug` spelling (`Official`) by a schema without
/// the CHECK constraint still reads back.
fn lowercase_tag_to_enum<T: serde::de::DeserializeOwned>(tag: Option<&str>) -> Option<T> {
    tag_to_enum(tag.map(str::to_ascii_lowercase).as_deref())
}

/// A list position as the `INTEGER` column takes it.
fn position(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

/// The `gender` column's value, lowercased for the `workers_gender_check`
/// CHECK constraint (`male`/`female`/`other`/`unknown`).
fn gender_tag(worker: &Worker) -> String {
    format!("{:?}", worker.gender).to_lowercase()
}

/// The `worker_type` column's value: its `Display` form.
fn worker_type_tag(worker: &Worker) -> Option<String> {
    worker
        .worker_type
        .as_ref()
        .map(std::string::ToString::to_string)
}

/// Insert the `workers` parent row for `worker`, with `created_at` and
/// `updated_at` both the transaction's `CURRENT_TIMESTAMP` and no digests:
/// [`stamp_digests`] writes those once the child rows are in.
///
/// The transaction's time, not this process's clock, because the digest
/// stamp is an `UPDATE`, and the `update_workers_updated_at` trigger sets
/// `updated_at` to `CURRENT_TIMESTAMP` as it lands. Stamping `created_at`
/// from the same clock keeps the two equal on a fresh record.
async fn insert_worker_row(conn: &mut PgConnection, worker: &Worker) -> Result<()> {
    sqlx::query!(
        "INSERT INTO workers
             (id, active, worker_type, gender, birth_date, tax_id, deceased, deceased_datetime,
              marital_status, multiple_birth, managing_organization_id, created_at, updated_at,
              created_by, updated_by, deleted_at, deleted_by, content_hash, content_hash_sha3,
              content_mac)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP,
                 CURRENT_TIMESTAMP, NULL, NULL, NULL, NULL, NULL, NULL, NULL)",
        worker.id,
        worker.active,
        worker_type_tag(worker),
        gender_tag(worker),
        worker.birth_date,
        worker.tax_id,
        worker.deceased,
        worker.deceased_datetime,
        worker.marital_status,
        worker.multiple_birth,
        worker.managing_organization,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Insert one `worker_names` row. The name use goes in its lowercase wire
/// token (see [`insert_children`]).
async fn insert_name(
    conn: &mut PgConnection,
    worker_id: Uuid,
    name: &HumanName,
    is_primary: bool,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO worker_names
             (id, worker_id, use_type, family, given, prefix, suffix, is_primary, created_at,
              updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        Uuid::new_v4(),
        worker_id,
        name.use_type.as_ref().and_then(enum_to_tag),
        name.family,
        &name.given,
        &name.prefix,
        &name.suffix,
        is_primary,
        Utc::now(),
        Utc::now(),
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Insert every child row of `worker` on `conn`: the primary name
/// (flagged `is_primary = true`) and additional names, identifiers,
/// addresses and contacts (the first of each flagged primary by
/// position), links, documents, emergency contacts with their telecom
/// rows, and photos.
///
/// The name, identifier and contact uses, the identifier type and the
/// contact system are stored as their serde wire tokens ([`enum_to_tag`]:
/// `official`, `OTHER`, `phone`), which are the spellings the tables'
/// CHECK constraints take, and the link type as [`link_type_tag`] spells
/// it. [`PgWorkerRepository::from_db_models`] parses them back. They used
/// to be written in their Rust `Debug` form (`Official`, `Other`,
/// `Phone`, `Seealso`), which the constraints refuse, so any write
/// carrying one answered `500` (WK-D1). An address's `use_type` is not
/// stored (it is written `NULL`).
async fn insert_children(conn: &mut PgConnection, worker: &Worker) -> Result<()> {
    insert_name(conn, worker.id, &worker.name, true).await?;
    for name in &worker.additional_names {
        insert_name(conn, worker.id, name, false).await?;
    }
    for id in &worker.identifiers {
        sqlx::query!(
            "INSERT INTO worker_identifiers
                 (id, worker_id, use_type, identifier_type, system, value, assigner, created_at,
                  updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            Uuid::new_v4(),
            worker.id,
            id.use_type.as_ref().and_then(enum_to_tag),
            enum_to_tag(&id.identifier_type).unwrap_or_default(),
            id.system,
            id.value,
            id.assigner,
            Utc::now(),
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (idx, addr) in worker.addresses.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO worker_addresses
                 (id, worker_id, use_type, line1, line2, city, state, postal_code, country,
                  is_primary, created_at, updated_at)
             VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            Uuid::new_v4(),
            worker.id,
            addr.line1,
            addr.line2,
            addr.city,
            addr.state,
            addr.postal_code,
            addr.country,
            idx == 0,
            Utc::now(),
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (idx, cp) in worker.telecom.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO worker_contacts
                 (id, worker_id, system, value, use_type, is_primary, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            Uuid::new_v4(),
            worker.id,
            enum_to_tag(&cp.system).unwrap_or_default(),
            cp.value,
            cp.use_type.as_ref().and_then(enum_to_tag),
            idx == 0,
            Utc::now(),
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
    }
    for link in &worker.links {
        sqlx::query!(
            "INSERT INTO worker_links
                 (id, worker_id, other_worker_id, link_type, created_at, created_by)
             VALUES ($1, $2, $3, $4, $5, NULL)",
            Uuid::new_v4(),
            worker.id,
            link.other_worker_id,
            link_type_tag(&link.link_type),
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
    }
    insert_extra_collections(conn, worker).await
}

/// Insert the normalized document / emergency-contact (+ telecom) / photo
/// child rows for `worker` on connection `conn`.
///
/// These three collections live in their own tables and preserve list
/// order via a `position` column. Each child row gets a fresh UUID;
/// emergency-contact telecom rows reference the parent contact's UUID.
async fn insert_extra_collections(conn: &mut PgConnection, worker: &Worker) -> Result<()> {
    for (i, doc) in worker.documents.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO worker_documents
                 (id, worker_id, document_type, number, issuing_country, issuing_authority,
                  issue_date, expiry_date, verified, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            Uuid::new_v4(),
            worker.id,
            enum_to_tag(&doc.document_type).unwrap_or_default(),
            doc.number,
            doc.issuing_country,
            doc.issuing_authority,
            doc.issue_date,
            doc.expiry_date,
            doc.verified,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, ec) in worker.emergency_contacts.iter().enumerate() {
        let ec_id = Uuid::new_v4();
        let addr = ec.address.as_ref();
        sqlx::query!(
            "INSERT INTO worker_emergency_contacts
                 (id, worker_id, name, relationship, is_primary, address_use_type,
                  address_line1, address_line2, address_city, address_state,
                  address_postal_code, address_country, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            ec_id,
            worker.id,
            ec.name,
            ec.relationship,
            ec.is_primary,
            addr.and_then(|a| a.use_type.as_ref().and_then(enum_to_tag)),
            addr.and_then(|a| a.line1.clone()),
            addr.and_then(|a| a.line2.clone()),
            addr.and_then(|a| a.city.clone()),
            addr.and_then(|a| a.state.clone()),
            addr.and_then(|a| a.postal_code.clone()),
            addr.and_then(|a| a.country.clone()),
            position(i),
        )
        .execute(&mut *conn)
        .await?;
        for (j, cp) in ec.telecom.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO worker_emergency_contact_telecom
                     (id, emergency_contact_id, system, value, use_type, position)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                Uuid::new_v4(),
                ec_id,
                enum_to_tag(&cp.system).unwrap_or_default(),
                cp.value,
                cp.use_type.as_ref().and_then(enum_to_tag),
                position(j),
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    for (i, url) in worker.photo.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO worker_photos (id, worker_id, url, position) VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            worker.id,
            url,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Delete every child row of worker `id` on `conn`. Deleting the
/// emergency contacts cascades to their telecom rows.
async fn delete_children(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!("DELETE FROM worker_names WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM worker_identifiers WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM worker_addresses WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM worker_contacts WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM worker_links WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!(
        "DELETE FROM worker_emergency_contacts WHERE worker_id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM worker_documents WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM worker_photos WHERE worker_id = $1", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Apply a worker's parent-row update and wholesale child-row replacement
/// on `conn`, then stamp its digests over the record as stored. Shared by
/// [`WorkerRepository::update`] and [`WorkerRepository::merge`]: writes the
/// demographic columns, stamps `updated_at` to now and `updated_by` to
/// `NULL`, leaves `created_*` and the soft-delete stamp alone, deletes every
/// child row for the worker, then re-inserts them from the domain model.
/// Runs on the caller's transaction so it shares that commit boundary.
///
/// An id that matches no row is an error, as it was under `SeaORM`.
async fn apply_update_rows(conn: &mut PgConnection, worker: &Worker) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE workers
         SET active = $2, worker_type = $3, gender = $4, birth_date = $5, tax_id = $6,
             deceased = $7, deceased_datetime = $8, marital_status = $9,
             multiple_birth = $10, managing_organization_id = $11, updated_at = $12,
             updated_by = NULL
         WHERE id = $1",
        worker.id,
        worker.active,
        worker_type_tag(worker),
        gender_tag(worker),
        worker.birth_date,
        worker.tax_id,
        worker.deceased,
        worker.deceased_datetime,
        worker.marital_status,
        worker.multiple_birth,
        worker.managing_organization,
        Utc::now(),
    )
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }

    delete_children(conn, worker.id).await?;
    insert_children(conn, worker).await?;
    stamp_digests(conn, worker.id).await
}

/// Write every digest of worker `id` over the record **as stored**, read
/// back on `conn` inside the write's transaction (INT-V1), with the row's
/// own soft-delete stamp bound in.
///
/// The digest leaves out `created_at` and `updated_at`
/// ([`crate::compliance::record_integrity`]), so the `updated_at` the
/// trigger writes as this `UPDATE` lands cannot disturb it.
///
/// # Errors
///
/// When the record cannot be read back or serialized, or the update fails.
async fn stamp_digests(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    let row = fetch_worker_row(conn, id, true).await?.ok_or_else(|| {
        crate::Error::Validation("worker not found while stamping digests".into())
    })?;
    let deleted_at = row.deleted_at.map(|d| d.timestamp_micros());
    let stored = assemble(conn, row).await?;
    let d = crate::compliance::record_integrity::digests_with_deleted_at(&stored, deleted_at)?;
    sqlx::query!(
        "UPDATE workers SET content_hash = $2, content_hash_sha3 = $3, content_mac = $4
         WHERE id = $1",
        id,
        d.sha256,
        d.sha3,
        d.mac,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Lock a merge's two rows `FOR UPDATE`, in id order so two merges of the
/// same pair in opposite directions cannot deadlock, then require both to
/// be active (SEC-B5). Returns [`crate::Error::Conflict`] when either was
/// deleted since the caller read it, and the caller's transaction then
/// writes nothing.
///
/// # Errors
/// [`crate::Error::Conflict`] as above, or a database error.
async fn lock_merge_participants(
    conn: &mut PgConnection,
    survivor_id: Uuid,
    duplicate_id: Uuid,
) -> Result<()> {
    let mut ids = [survivor_id, duplicate_id];
    ids.sort();
    for id in ids {
        sqlx::query_scalar!("SELECT id FROM workers WHERE id = $1 FOR UPDATE", id)
            .fetch_optional(&mut *conn)
            .await?;
    }
    let active = sqlx::query_scalar!(
        r#"SELECT count(*) AS "active!" FROM workers WHERE id = ANY($1) AND deleted_at IS NULL"#,
        &ids[..],
    )
    .fetch_one(&mut *conn)
    .await?;
    if active == 2 {
        Ok(())
    } else {
        Err(crate::Error::Conflict(format!(
            "worker {survivor_id} or {duplicate_id} is no longer active (merged or deleted meanwhile)"
        )))
    }
}

/// Soft-delete worker `id` on `conn` by stamping `deleted_at` and
/// `deleted_by = 'system'`, leaving every other column (and the child
/// rows) in place. Shared by [`WorkerRepository::delete`] and
/// [`WorkerRepository::merge`].
///
/// A soft delete changes the lifecycle state the digest binds, so it is
/// rehashed, or every deleted record would read as tampered: `worker` is
/// the assembled record the caller already loaded for the audit trail,
/// and all three digest columns are rewritten over it. When it is absent
/// (the row was gone before we looked) there is nothing to rehash and the
/// columns are left as they were.
///
/// An id that matches no row is an error, as it was under `SeaORM`.
async fn apply_soft_delete_row(
    conn: &mut PgConnection,
    id: &Uuid,
    worker: Option<&Worker>,
) -> Result<()> {
    let deleted_at = Utc::now();
    let tombstone = worker
        .map(|w| {
            crate::compliance::record_integrity::digests_with_deleted_at(
                w,
                Some(deleted_at.timestamp_micros()),
            )
        })
        .transpose()?;
    let rehash = tombstone.is_some();
    let (sha256, sha3, mac) = tombstone.map_or((None, None, None), |d| {
        (Some(d.sha256), Some(d.sha3), d.mac)
    });
    let done = sqlx::query!(
        "UPDATE workers
         SET deleted_at = $2, deleted_by = 'system',
             content_hash = CASE WHEN $3 THEN $4 ELSE content_hash END,
             content_hash_sha3 = CASE WHEN $3 THEN $5 ELSE content_hash_sha3 END,
             content_mac = CASE WHEN $3 THEN $6 ELSE content_mac END
         WHERE id = $1",
        id,
        deleted_at,
        rehash,
        sha256,
        sha3,
        mac,
    )
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }
    Ok(())
}

/// Read one `workers` row; a soft-deleted row only when `include_deleted`.
async fn fetch_worker_row(
    conn: &mut PgConnection,
    id: Uuid,
    include_deleted: bool,
) -> Result<Option<WorkerRow>> {
    let row = sqlx::query_as!(
        WorkerRow,
        "SELECT id, active, worker_type, gender, birth_date, tax_id, deceased,
                deceased_datetime, marital_status, multiple_birth, managing_organization_id,
                created_at, updated_at, created_by, updated_by, deleted_at, deleted_by,
                content_hash, content_hash_sha3, content_mac
         FROM workers
         WHERE id = $1 AND ($2 OR deleted_at IS NULL)
         LIMIT 1",
        id,
        include_deleted,
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row)
}

/// Load every child row of `row` on `conn` and reassemble the domain
/// [`Worker`].
async fn assemble(conn: &mut PgConnection, row: WorkerRow) -> Result<Worker> {
    let id = row.id;
    let names = sqlx::query_as!(
        NameRow,
        "SELECT id, worker_id, use_type, family, given, prefix, suffix, is_primary
         FROM worker_names WHERE worker_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let identifiers = sqlx::query_as!(
        IdentifierRow,
        "SELECT id, worker_id, use_type, identifier_type, system, value, assigner
         FROM worker_identifiers WHERE worker_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let addresses = sqlx::query_as!(
        AddressRow,
        "SELECT id, worker_id, line1, line2, city, state, postal_code, country
         FROM worker_addresses WHERE worker_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let contacts = sqlx::query_as!(
        ContactRow,
        "SELECT id, worker_id, system, value, use_type
         FROM worker_contacts WHERE worker_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let links = sqlx::query_as!(
        LinkRow,
        "SELECT id, worker_id, other_worker_id, link_type
         FROM worker_links WHERE worker_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut worker = PgWorkerRepository::from_db_models(
        row,
        &names,
        &identifiers,
        &addresses,
        &contacts,
        &links,
    )?;
    load_extra_collections(conn, &mut worker).await?;
    Ok(worker)
}

/// Read and assemble worker `id` on `conn`; a soft-deleted one only when
/// `include_deleted`.
async fn read_worker(
    conn: &mut PgConnection,
    id: Uuid,
    include_deleted: bool,
) -> Result<Option<Worker>> {
    match fetch_worker_row(conn, id, include_deleted).await? {
        Some(row) => Ok(Some(assemble(conn, row).await?)),
        None => Ok(None),
    }
}

/// Load the normalized document / emergency-contact / photo child rows
/// for `worker.id` and populate them onto `worker`.
///
/// The companion to [`insert_extra_collections`]: each collection is read
/// back ordered by its `position` column so list order is preserved, and
/// emergency-contact telecom rows are fetched per contact. String tags are
/// decoded with [`tag_to_enum`], falling back to the `Other` variant for an
/// unknown tag. An emergency-contact address is reconstructed only when at
/// least one flattened address column is present.
async fn load_extra_collections(conn: &mut PgConnection, worker: &mut Worker) -> Result<()> {
    let id = worker.id;

    let doc_rows = sqlx::query_as!(
        DocumentRow,
        "SELECT document_type, number, issuing_country, issuing_authority, issue_date,
                expiry_date, verified
         FROM worker_documents WHERE worker_id = $1 ORDER BY position ASC",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    worker.documents = doc_rows
        .into_iter()
        .map(|r| IdentityDocument {
            document_type: tag_to_enum(Some(r.document_type.as_str()))
                .unwrap_or(DocumentType::Other),
            number: r.number,
            issuing_country: r.issuing_country,
            issuing_authority: r.issuing_authority,
            issue_date: r.issue_date,
            expiry_date: r.expiry_date,
            verified: r.verified,
        })
        .collect();

    let ec_rows = sqlx::query_as!(
        EmergencyContactRow,
        "SELECT id, name, relationship, is_primary, address_use_type, address_line1,
                address_line2, address_city, address_state, address_postal_code,
                address_country
         FROM worker_emergency_contacts WHERE worker_id = $1 ORDER BY position ASC",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut emergency_contacts = Vec::with_capacity(ec_rows.len());
    for ec in ec_rows {
        let tel_rows = sqlx::query_as!(
            EmergencyTelecomRow,
            "SELECT system, value, use_type
             FROM worker_emergency_contact_telecom
             WHERE emergency_contact_id = $1 ORDER BY position ASC",
            ec.id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let telecom = tel_rows
            .into_iter()
            .map(|t| ContactPoint {
                system: tag_to_enum(Some(t.system.as_str())).unwrap_or(ContactPointSystem::Other),
                value: t.value,
                use_type: tag_to_enum(t.use_type.as_deref()),
            })
            .collect();
        let has_address = ec.address_use_type.is_some()
            || ec.address_line1.is_some()
            || ec.address_line2.is_some()
            || ec.address_city.is_some()
            || ec.address_state.is_some()
            || ec.address_postal_code.is_some()
            || ec.address_country.is_some();
        let address = has_address.then(|| Address {
            use_type: tag_to_enum(ec.address_use_type.as_deref()),
            line1: ec.address_line1,
            line2: ec.address_line2,
            city: ec.address_city,
            state: ec.address_state,
            postal_code: ec.address_postal_code,
            country: ec.address_country,
        });
        emergency_contacts.push(EmergencyContact {
            name: ec.name,
            relationship: ec.relationship,
            telecom,
            address,
            is_primary: ec.is_primary,
        });
    }
    worker.emergency_contacts = emergency_contacts;

    let photos = sqlx::query_scalar!(
        "SELECT url FROM worker_photos WHERE worker_id = $1 ORDER BY position ASC",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    worker.photo = photos;

    Ok(())
}

/// Actor metadata attached to audit-log entries for a mutation.
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Acting user's identifier (defaults to `"system"`).
    pub user_id: Option<String>,
    /// Originating client IP address, if known.
    pub ip_address: Option<String>,
    /// Originating client user-agent string, if known.
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    /// Returns a context attributed to the `"system"` user with no IP or
    /// user-agent — used for internal/automated operations.
    fn default() -> Self {
        Self {
            user_id: Some("system".to_string()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// Persistence operations for worker records. Implementors must be `Send +
/// Sync` so the repository can be shared across async tasks behind an `Arc`.
#[async_trait::async_trait]
pub trait WorkerRepository: Send + Sync {
    /// Persists a new worker (and its names/identifiers/addresses/contacts/
    /// links), returning the stored record.
    ///
    /// # Errors
    ///
    /// Returns an error if the transactional insert fails, or if reloading the
    /// just-written record fails (e.g. a missing primary name).
    async fn create(&self, worker: &Worker, ctx: &AuditContext) -> Result<Worker>;

    /// Fetches a non-deleted worker by ID, or `None` if absent/soft-deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if a database query fails, or if a found row cannot be
    /// reassembled into a domain [`Worker`].
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Worker>>;

    /// Replaces a worker and its associated rows, returning the new state.
    ///
    /// # Errors
    ///
    /// Returns an error if the transactional update fails, or if the worker is
    /// not found after the update.
    async fn update(&self, worker: &Worker, ctx: &AuditContext) -> Result<Worker>;

    /// Merges the `duplicate_id` record into `survivor`: applies the
    /// survivor's parent + child row updates and soft-deletes the
    /// duplicate **in one transaction**, so the whole merge commits (or
    /// rolls back) atomically. Under the outbox transport this enqueues a
    /// `Merged` outbox row for the survivor (carrying the duplicate's pid
    /// via `merged_from`) and a `Deleted` outbox row for the duplicate on
    /// the same transaction. Returns the reloaded survivor.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction or any query fails, or
    /// [`crate::Error::Validation`] if the survivor cannot be re-read after
    /// the merge.
    async fn merge(
        &self,
        survivor: &Worker,
        duplicate_id: &Uuid,
        ctx: &AuditContext,
    ) -> Result<Worker>;

    /// Soft-deletes a worker by stamping its `deleted_at` column.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    async fn delete(&self, id: &Uuid, ctx: &AuditContext) -> Result<()>;

    /// Returns a page of active, non-deleted workers.
    ///
    /// # Errors
    ///
    /// Returns an error if a database query fails.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Worker>>;
}

/// `PostgreSQL` implementation of [`WorkerRepository`] over sqlx.
///
/// Optionally wired with an event publisher and an audit-log repository via
/// the `with_*` builder methods; when present, every mutation publishes an
/// event and records an audit entry.
pub struct PgWorkerRepository {
    /// The connection pool (cheap to clone).
    db: PgPool,
    /// Optional event publisher; mutations emit events when set.
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    /// Optional audit repository; mutations log entries when set.
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the legacy post-commit
    /// ring-buffer publish; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl PgWorkerRepository {
    /// Builds a repository over `db` with no event publisher or audit log,
    /// and the default [`EventTransport::Memory`] transport (behaviour
    /// unchanged from before the outbox landed).
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            event_publisher: None,
            audit_log: None,
            transport: EventTransport::Memory,
        }
    }

    /// Builder: select the event transport (see [`EventTransport`]).
    /// `AppState` wires this from `WORKER_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `worker` **on `conn`** — pass the open
    /// transaction so the row commits with the entity write (the outbox
    /// atomicity guarantee). A no-op under [`EventTransport::Memory`].
    ///
    /// # Errors
    ///
    /// Returns an error if the outbox `INSERT` fails (e.g. a duplicate
    /// `event_id`) or the envelope cannot be built.
    async fn enqueue_outbox(
        &self,
        conn: &mut PgConnection,
        worker: &Worker,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(worker, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Builder method attaching an event publisher; returns `self` for chaining.
    #[must_use]
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Builder method attaching an audit-log repository; returns `self` for chaining.
    #[must_use]
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publishes `event` if a publisher is configured; logs (but swallows) any
    /// publish error so a streaming failure never aborts the database write.
    fn publish_event(&self, event: crate::streaming::WorkerEvent) {
        if let Some(ref publisher) = self.event_publisher
            && let Err(e) = publisher.publish(event)
        {
            tracing::error!("Failed to publish event: {}", e);
        }
    }

    /// Writes a `CREATE`/`UPDATE`/`DELETE` audit entry for a `Worker` if an
    /// audit log is configured. Unknown actions are no-ops; errors are logged
    /// but not propagated.
    ///
    /// Dispatches on `action` to the matching typed helper on the audit
    /// repository, passing the JSON snapshots and actor metadata. A `None`
    /// snapshot is substituted with JSON `null` for the column. Like
    /// [`Self::publish_event`], an audit failure is logged but never surfaced,
    /// so the audit trail is best-effort and decoupled from the primary write.
    async fn log_audit(
        &self,
        action: &str,
        entity_id: uuid::Uuid,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        context: &AuditContext,
    ) {
        if let Some(ref audit_log) = self.audit_log {
            let actor = super::audit::AuditActor {
                user_id: context.user_id.as_deref(),
                ip_address: context.ip_address.as_deref(),
                user_agent: context.user_agent.as_deref(),
            };
            let result = match action {
                "CREATE" => {
                    audit_log
                        .log_create(
                            "Worker",
                            entity_id,
                            new_values.unwrap_or(serde_json::Value::Null),
                            &actor,
                        )
                        .await
                }
                "UPDATE" => {
                    audit_log
                        .log_update(
                            "Worker",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            new_values.unwrap_or(serde_json::Value::Null),
                            &actor,
                        )
                        .await
                }
                "DELETE" => {
                    audit_log
                        .log_delete(
                            "Worker",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            &actor,
                        )
                        .await
                }
                _ => Ok(()),
            };

            if let Err(e) = result {
                tracing::error!("Failed to log audit: {}", e);
            }
        }
    }

    /// Maps one `worker_names` row back into a domain [`HumanName`], parsing
    /// the string-stored name-use tag.
    fn human_name_from_db(n: &NameRow) -> HumanName {
        HumanName {
            use_type: lowercase_tag_to_enum(n.use_type.as_deref()),
            family: n.family.clone(),
            given: n.given.clone(),
            prefix: n.prefix.clone(),
            suffix: n.suffix.clone(),
        }
    }

    /// Maps the `worker_identifiers` rows back into domain [`Identifier`]s,
    /// parsing the string-stored type / use-type tags.
    fn identifiers_from_db(db_identifiers: &[IdentifierRow]) -> Vec<Identifier> {
        use crate::models::IdentifierType;

        db_identifiers
            .iter()
            .map(|id| {
                // UPPERCASE on the wire and in the column; an unknown
                // token is `Other`, as `#[serde(other)]` makes it.
                let identifier_type =
                    tag_to_enum(Some(id.identifier_type.to_ascii_uppercase().as_str()))
                        .unwrap_or(IdentifierType::Other);
                let use_type = lowercase_tag_to_enum(id.use_type.as_deref());

                Identifier {
                    identifier_type,
                    use_type,
                    system: id.system.clone(),
                    value: id.value.clone(),
                    assigner: id.assigner.clone(),
                }
            })
            .collect()
    }

    /// Maps the `worker_contacts` rows back into domain [`ContactPoint`]s,
    /// dropping any row whose system tag is unrecognized.
    fn telecom_from_db(db_contacts: &[ContactRow]) -> Vec<ContactPoint> {
        db_contacts
            .iter()
            .filter_map(|cp| {
                let system = lowercase_tag_to_enum(Some(cp.system.as_str()))?;
                let use_type = lowercase_tag_to_enum(cp.use_type.as_deref());

                Some(ContactPoint {
                    system,
                    value: cp.value.clone(),
                    use_type,
                })
            })
            .collect()
    }

    /// Reassembles a domain [`Worker`] from its database rows, parsing the
    /// string-stored enums (gender, worker type, name/identifier/contact/link
    /// use types) back into their typed forms. Requires a primary name row;
    /// returns a [`crate::Error::Validation`] if none is present. Unparseable
    /// contact/link tags are dropped (`filter_map`) rather than erroring. The
    /// `documents` / `emergency_contacts` / `photo` collections are left empty
    /// here and populated separately by [`load_extra_collections`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Validation`] if `db_names` contains no primary
    /// name row.
    fn from_db_models(
        db_worker: WorkerRow,
        db_names: &[NameRow],
        db_identifiers: &[IdentifierRow],
        db_addresses: &[AddressRow],
        db_contacts: &[ContactRow],
        db_links: &[LinkRow],
    ) -> Result<Worker> {
        use crate::models::Gender;

        // Parse gender. The DB stores lowercase per the CHECK constraint
        // ('male'/'female'/'other'/'unknown'); accept PascalCase too, so
        // rows written by the older `format!("{:?}", …)` path (which
        // reached deployments whose schema lacked the constraint) still
        // round-trip instead of silently reading back as `Unknown`.
        let gender = match db_worker.gender.to_lowercase().as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            "other" => Gender::Other,
            _ => Gender::Unknown,
        };

        // Parse worker_type
        let worker_type = db_worker.worker_type.as_deref().and_then(|wt| match wt {
            "doctor" => Some(WorkerType::Doctor),
            "nurse" => Some(WorkerType::Nurse),
            "carer" => Some(WorkerType::Carer),
            "staff" => Some(WorkerType::Staff),
            "employee" => Some(WorkerType::Employee),
            "manager" => Some(WorkerType::Manager),
            "supervisor" => Some(WorkerType::Supervisor),
            "consultant" => Some(WorkerType::Consultant),
            "other" => Some(WorkerType::Other),
            _ => None,
        });

        // Get primary name
        let primary_name = db_names
            .iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Worker has no primary name".to_string()))?;

        let name = Self::human_name_from_db(primary_name);

        // Additional names
        let additional_names = db_names
            .iter()
            .filter(|n| !n.is_primary)
            .map(Self::human_name_from_db)
            .collect();

        let identifiers = Self::identifiers_from_db(db_identifiers);

        // Addresses
        let addresses = db_addresses
            .iter()
            .map(|addr| Address {
                use_type: None,
                line1: addr.line1.clone(),
                line2: addr.line2.clone(),
                city: addr.city.clone(),
                state: addr.state.clone(),
                postal_code: addr.postal_code.clone(),
                country: addr.country.clone(),
            })
            .collect();

        let telecom = Self::telecom_from_db(db_contacts);

        // Links
        let links = db_links
            .iter()
            .filter_map(|link| {
                let link_type = link_type_from_tag(&link.link_type)?;

                Some(WorkerLink {
                    other_worker_id: link.other_worker_id,
                    link_type,
                })
            })
            .collect();

        Ok(Worker {
            id: db_worker.id,
            identifiers,
            active: db_worker.active,
            name,
            additional_names,
            telecom,
            gender,
            worker_type,
            birth_date: db_worker.birth_date,
            deceased: db_worker.deceased,
            deceased_datetime: db_worker.deceased_datetime,
            addresses,
            marital_status: db_worker.marital_status,
            multiple_birth: db_worker.multiple_birth,
            tax_id: db_worker.tax_id,
            // Loaded separately from child tables by `load_extra_collections`.
            documents: vec![],
            emergency_contacts: vec![],
            photo: vec![],
            managing_organization: db_worker.managing_organization_id,
            links,
            created_at: db_worker.created_at,
            updated_at: db_worker.updated_at,
        })
    }
}

// sqlx implementation of the repository trait. Each mutation runs its
// multi-table writes inside a transaction, then (best-effort) publishes an
// event and records an audit entry after the commit.
#[async_trait::async_trait]
impl WorkerRepository for PgWorkerRepository {
    async fn create(&self, worker: &Worker, ctx: &AuditContext) -> Result<Worker> {
        // All inserts share one transaction so a partial worker is never left.
        let mut txn = self.db.begin().await?;

        insert_worker_row(&mut txn, worker).await?;
        insert_children(&mut txn, worker).await?;
        // Digests over the record as stored, in this transaction (INT-V1).
        stamp_digests(&mut txn, worker.id).await?;

        // Durable event bus (Phase 2): under the outbox transport, write the
        // `event_outbox` row **inside this same transaction**, before the
        // commit, so the entity rows and the event commit atomically (or roll
        // back together). A no-op under the memory transport, which keeps the
        // post-commit ring-buffer publish below.
        self.enqueue_outbox(&mut txn, worker, EventKind::Created)
            .await?;

        // Commit closes the transaction boundary: everything above is now
        // durable. The reload + event + audit steps below run post-commit and
        // are best-effort (a streaming/audit failure must not undo the write).
        txn.commit().await?;

        // Reload from the DB so the returned record reflects exactly what was
        // persisted (server-assigned timestamps, normalized child rows).
        let mut conn = self.db.acquire().await?;
        let result = read_worker(&mut conn, worker.id, true)
            .await?
            .ok_or_else(|| crate::Error::Validation("Worker not found after create".to_string()))?;
        drop(conn);

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Created {
            worker: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Ok(worker_json) = serde_json::to_value(&result) {
            self.log_audit("CREATE", result.id, None, Some(worker_json), ctx)
                .await;
        }

        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Worker>> {
        let mut conn = self.db.acquire().await?;
        read_worker(&mut conn, *id, false).await
    }

    async fn update(&self, worker: &Worker, ctx: &AuditContext) -> Result<Worker> {
        // Capture the pre-update state for the audit diff.
        let old_worker = self.get_by_id(&worker.id).await?;

        // Update is implemented as "update parent row + replace all child
        // rows" within a single transaction, then the digest stamp.
        let mut txn = self.db.begin().await?;
        apply_update_rows(&mut txn, worker).await?;

        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&mut txn, worker, EventKind::Updated)
            .await?;

        txn.commit().await?;

        // Fetch and return updated worker
        let result = self
            .get_by_id(&worker.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Worker not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Updated {
            worker: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_worker
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
            && let Ok(new_json) = serde_json::to_value(&result)
        {
            self.log_audit("UPDATE", result.id, Some(old_json), Some(new_json), ctx)
                .await;
        }

        Ok(result)
    }

    async fn merge(
        &self,
        survivor: &Worker,
        duplicate_id: &Uuid,
        ctx: &AuditContext,
    ) -> Result<Worker> {
        // Snapshots for the audit trail: the survivor's pre-image for the
        // UPDATE row, the duplicate's pre-image for the DELETE row (and to
        // build the duplicate's `Deleted` outbox envelope).
        let old_survivor = self.get_by_id(&survivor.id).await?;
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let mut txn = self.db.begin().await?;
        // SEC-B5: the merge was computed from unlocked reads, so lock both
        // rows before writing and check under the lock that both are still
        // active. Otherwise two concurrent merges of one duplicate both
        // apply, and "B into A" racing "A into B" deletes both. The loser
        // gets `Conflict` (409) and writes nothing.
        lock_merge_participants(&mut txn, survivor.id, *duplicate_id).await?;

        // Apply the survivor's parent + child rows (parent update + wholesale
        // child-row replacement + digest stamp), shared with `update`.
        // The duplicate's identifiers move to the survivor, and
        // `worker_identifiers` is unique on `(system, value)` across the
        // table, so the duplicate's rows must go first: written with them
        // still in place, every merge of a duplicate holding an identifier
        // violated the key and failed. Its pre-image, identifiers and all,
        // stays in the audit trail's DELETE row below.
        sqlx::query!(
            "DELETE FROM worker_identifiers WHERE worker_id = $1",
            duplicate_id
        )
        .execute(&mut *txn)
        .await?;
        apply_update_rows(&mut txn, survivor).await?;

        // Soft-delete the duplicate on the same transaction, rehashed over
        // the pre-image loaded above for the audit trail (shared with
        // `delete`).
        // The tombstone's digests cover it as stored: without the
        // identifiers just moved to the survivor.
        let tombstone = old_duplicate.as_ref().map(|d| {
            let mut d = d.clone();
            d.identifiers.clear();
            d
        });
        apply_soft_delete_row(&mut txn, duplicate_id, tombstone.as_ref()).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the survivor
        // (carrying the duplicate's pid via `merged_from`, so a merge-repointing
        // consumer can move edges off the duplicate) and a `Deleted` outbox row
        // for the duplicate — both inside this transaction. A no-op under the
        // memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&mut txn)
                .await?;
            if let Some(dup) = old_duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&mut txn)
                    .await?;
            }
        }

        txn.commit().await?;

        let result = self
            .get_by_id(&survivor.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Worker not found after merge".to_string()))?;

        // Post-commit in-memory stream (memory transport keeps this): a `Merged`
        // for the survivor and a `Deleted` for the duplicate.
        self.publish_event(crate::streaming::WorkerEvent::Merged {
            source_id: *duplicate_id,
            target_id: result.id,
            timestamp: chrono::Utc::now(),
        });
        self.publish_event(crate::streaming::WorkerEvent::Deleted {
            worker_id: *duplicate_id,
            timestamp: chrono::Utc::now(),
        });

        // Audit rows: an UPDATE trail for the survivor and a DELETE trail for
        // the duplicate.
        if let Some(old_json) = old_survivor
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
            && let Ok(new_json) = serde_json::to_value(&result)
        {
            self.log_audit("UPDATE", result.id, Some(old_json), Some(new_json), ctx)
                .await;
        }
        if let Some(dup) = old_duplicate
            && let Ok(dup_json) = serde_json::to_value(&dup)
        {
            self.log_audit("DELETE", *duplicate_id, Some(dup_json), None, ctx)
                .await;
        }

        Ok(result)
    }

    async fn delete(&self, id: &Uuid, ctx: &AuditContext) -> Result<()> {
        // Capture the pre-delete state for the audit record.
        let old_worker = self.get_by_id(id).await?;

        // Soft delete: stamp deleted_at/deleted_by rather than removing rows,
        // rehashed over the pre-image (see `apply_soft_delete_row`). Unlike
        // create/update, this is a single row update with no existing
        // transaction. Under the outbox transport we open one here so the
        // tombstone write and the `deleted` outbox row commit atomically;
        // the memory transport keeps the plain, tx-free update.
        if self.transport.is_outbox() {
            let mut txn = self.db.begin().await?;
            apply_soft_delete_row(&mut txn, id, old_worker.as_ref()).await?;
            if let Some(old) = old_worker.as_ref() {
                self.enqueue_outbox(&mut txn, old, EventKind::Deleted)
                    .await?;
            }
            txn.commit().await?;
        } else {
            let mut conn = self.db.acquire().await?;
            apply_soft_delete_row(&mut conn, id, old_worker.as_ref()).await?;
        }

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Deleted {
            worker_id: *id,
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_worker) = old_worker
            && let Ok(old_json) = serde_json::to_value(&old_worker)
        {
            self.log_audit("DELETE", *id, Some(old_json), None, ctx)
                .await;
        }

        Ok(())
    }

    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Worker>> {
        // `ORDER BY id` is load-bearing, not cosmetic: without an explicit
        // ORDER BY, Postgres does not guarantee a stable row order across
        // repeated `LIMIT`/`OFFSET` queries, so a caller paginating through
        // every page (the link-graph suggestion job's worker enumeration,
        // T-31) could silently skip or duplicate rows between pages.
        // Ordering by the primary key is cheap (already indexed) and gives
        // every page a deterministic, non-overlapping slice. Mirrors the
        // identical fix in person-service's `list_active` (see that crate's
        // `CHANGELOG.md`).
        let ids = sqlx::query_scalar!(
            "SELECT id FROM workers
             WHERE deleted_at IS NULL AND active = true
             ORDER BY id ASC
             LIMIT $1 OFFSET $2",
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(&self.db)
        .await?;

        // Page the parent rows in SQL (limit/offset), then hydrate each into a
        // full domain worker via `get_by_id`.
        let mut workers = Vec::new();
        for id in ids {
            if let Some(worker) = self.get_by_id(&id).await? {
                workers.push(worker);
            }
        }

        Ok(workers)
    }
}

/// One `workers` row's stored digests and soft-delete stamp, as
/// `/api/records/verify` and `integrity_resign` read them.
#[derive(Debug, Clone)]
pub struct StoredDigests {
    /// The record's id.
    pub id: Uuid,
    /// The stored SHA-256 content hash.
    pub content_hash: Option<String>,
    /// The stored SHA-3 content hash.
    pub content_hash_sha3: Option<String>,
    /// The stored keyed MAC.
    pub content_mac: Option<String>,
    /// The soft-delete stamp.
    pub deleted_at: Option<chrono::DateTime<Utc>>,
}

/// One record **as stored**, soft-deleted or not, with its stored
/// digests. `None` when no row has `id`. The deleted rows the live reads
/// skip are read through this, an erased one among them (WK-E3).
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if a query fails, or
/// [`crate::Error::Validation`] if the row does not assemble.
pub async fn stored_record(db: &PgPool, id: Uuid) -> Result<Option<(Worker, StoredDigests)>> {
    let mut conn = db.acquire().await?;
    let Some(row) = fetch_worker_row(&mut conn, id, true).await? else {
        return Ok(None);
    };
    let digests = StoredDigests {
        id: row.id,
        content_hash: row.content_hash.clone(),
        content_hash_sha3: row.content_hash_sha3.clone(),
        content_mac: row.content_mac.clone(),
        deleted_at: row.deleted_at,
    };
    let worker = assemble(&mut conn, row).await?;
    Ok(Some((worker, digests)))
}

/// The `limit` most recently updated `workers` rows' stored digests, for
/// `/api/records/verify`.
///
/// Ordered by `updated_at DESC, id DESC`: `id` is the primary key, so the
/// order is total and a capped run samples the same rows every time even
/// when rows share an `updated_at` (WK-V1).
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn rows_for_verification(db: &PgPool, limit: i64) -> Result<Vec<StoredDigests>> {
    let rows = sqlx::query_as!(
        StoredDigests,
        "SELECT id, content_hash, content_hash_sha3, content_mac, deleted_at
         FROM workers
         ORDER BY updated_at DESC, id DESC
         LIMIT $1",
        limit,
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Up to `limit` `workers` rows carrying a MAC, in id order — the
/// `integrity_resign` task's walk.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn rows_with_content_mac(db: &PgPool, limit: u64) -> Result<Vec<StoredDigests>> {
    let rows = sqlx::query_as!(
        StoredDigests,
        "SELECT id, content_hash, content_hash_sha3, content_mac, deleted_at
         FROM workers
         WHERE content_mac IS NOT NULL
         ORDER BY id ASC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Replace one worker's MAC, the only column `integrity_resign` writes.
/// No matching row is an error, as it was under `SeaORM`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails or matches no
/// row.
pub async fn set_content_mac(db: &PgPool, id: Uuid, mac: Option<String>) -> Result<()> {
    let done = sqlx::query!("UPDATE workers SET content_mac = $2 WHERE id = $1", id, mac)
        .execute(db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }
    Ok(())
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path. They
/// require a migrated `PostgreSQL` via `DB_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DB_URL=… cargo test --lib -- --ignored`. They must COMPILE under a
/// bare `cargo test --lib`.
#[cfg(test)]
mod tests {
    use super::{AuditContext, PgWorkerRepository, WorkerRepository, gender_tag};
    /// The `workers.gender` CHECK constraint's vocabulary, copied from
    /// `migrations/2024122800000002_create_workers/up.sql`. A value
    /// outside this set is rejected by `PostgreSQL` at insert time.
    const DB_GENDER_TOKENS: [&str; 4] = ["male", "female", "other", "unknown"];

    /// Every [`Gender`] variant is persisted as a token the
    /// `workers_gender_check` CHECK constraint accepts.
    ///
    /// Regression pin: the write paths previously stored the bare `Debug` form — `"Unknown"`,
    /// `"Male"` — which `PostgreSQL` rejects, so **every** create and
    /// update against a constrained schema failed with
    /// `violates check constraint "workers_gender_check"`. Pure and
    /// DB-free, so the vocabulary is pinned on a bare `cargo test --lib`
    /// rather than only when a database happens to be reachable.
    #[test]
    fn gender_is_persisted_as_a_constraint_legal_token() {
        for gender in [Gender::Male, Gender::Female, Gender::Other, Gender::Unknown] {
            let worker = Worker::new(
                HumanName {
                    use_type: None,
                    family: "Doe".into(),
                    given: vec!["Jane".into()],
                    prefix: vec![],
                    suffix: vec![],
                },
                gender,
            );
            let stored = gender_tag(&worker);
            assert!(
                DB_GENDER_TOKENS.contains(&stored.as_str()),
                "{gender:?} persisted as {stored:?}, which the CHECK constraint \
                 rejects (allowed: {DB_GENDER_TOKENS:?})"
            );
            // And it is the serde wire token, so the DB, the search index,
            // and the FHIR surface all agree on one spelling.
            assert_eq!(
                stored,
                serde_json::to_value(gender)
                    .expect("gender serializes")
                    .as_str()
                    .expect("as a string")
            );
        }
    }

    use crate::db::outbox::Model as OutboxRow;
    use crate::models::{Gender, HumanName, Worker};
    use crate::streaming::EventTransport;
    use sqlx::PgPool;

    async fn connect() -> PgPool {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for DB tests");
        PgPool::connect(&url).await.expect("connect to DB_URL")
    }

    fn a_worker(family: &str) -> Worker {
        let name = HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec!["Test".into()],
            prefix: vec![],
            suffix: vec![],
        };
        Worker::new(name, Gender::Unknown)
    }

    /// A merge's audit rows carry the caller's context on both records, rather
    /// than the `system` default the repository used to build for itself.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn merge_audit_rows_carry_the_callers_context() {
        use crate::db::audit::AuditLogRepository;

        let db = connect().await;
        let audit = std::sync::Arc::new(AuditLogRepository::new(db.clone()));
        let repo = PgWorkerRepository::new(db.clone()).with_audit_log(audit.clone());
        let system = AuditContext::default();
        let survivor = repo.create(&a_worker("Survivor"), &system).await.unwrap();
        let duplicate = repo.create(&a_worker("Duplicate"), &system).await.unwrap();

        let caller = AuditContext {
            user_id: Some("operator-merge-42".to_string()),
            ip_address: Some("198.51.100.7".to_string()),
            user_agent: None,
        };
        repo.merge(&survivor, &duplicate.id, &caller).await.unwrap();

        for (id, action) in [(survivor.id, "UPDATE"), (duplicate.id, "DELETE")] {
            let row = audit
                .rows_for(None, id, Some(action))
                .await
                .unwrap()
                .into_iter()
                .next()
                .expect("the merge writes an audit row");
            assert_eq!(
                row.user_id.as_deref(),
                Some("operator-merge-42"),
                "{action}"
            );
            assert_eq!(row.ip_address.as_deref(), Some("198.51.100.7"), "{action}");
        }
    }

    /// Create writes, in one transaction: exactly one `created` outbox row
    /// for the new worker under the outbox transport.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = PgWorkerRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let worker = repo
            .create(&a_worker("Created"), &AuditContext::default())
            .await
            .unwrap();

        let rows = OutboxRow::rows_for(&db, worker.id, Some("created"))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new worker");
        assert_eq!(rows[0].entity, "worker");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = PgWorkerRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo
            .create(&a_worker("Survivor"), &AuditContext::default())
            .await
            .unwrap();
        let duplicate = repo
            .create(&a_worker("Duplicate"), &AuditContext::default())
            .await
            .unwrap();

        let merged = repo
            .merge(&survivor, &duplicate.id, &AuditContext::default())
            .await
            .unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = OutboxRow::rows_for(&db, survivor.id, Some("merged"))
            .await
            .unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = OutboxRow::rows_for(&db, duplicate.id, Some("deleted"))
            .await
            .unwrap();
        assert_eq!(
            deleted_rows.len(),
            1,
            "one deleted outbox row for duplicate"
        );
    }

    /// SEC-B5: a merge locks both rows and rechecks them under the lock.
    /// Another transaction is soft-deleting the duplicate and holds its
    /// row; the merge waits for it, then finds the duplicate gone and
    /// answers `Conflict` without writing the survivor.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_merge_waits_for_a_concurrent_delete_and_then_refuses() {
        let db = connect().await;
        let repo = PgWorkerRepository::new(db.clone());
        let system = AuditContext::default();
        let survivor = repo
            .create(&a_worker("RaceSurvivor"), &system)
            .await
            .unwrap();
        let duplicate = repo
            .create(&a_worker("RaceDuplicate"), &system)
            .await
            .unwrap();

        let mut other = db.begin().await.unwrap();
        super::apply_soft_delete_row(&mut other, &duplicate.id, Some(&duplicate))
            .await
            .unwrap();

        let mut renamed = survivor.clone();
        renamed.name.family = "RaceSurvivorMerged".into();
        let merger = PgWorkerRepository::new(db.clone());
        let dup_id = duplicate.id;
        let merge = tokio::spawn(async move {
            merger
                .merge(&renamed, &dup_id, &AuditContext::default())
                .await
        });

        // Wait, up to ten seconds, until Postgres reports a backend waiting
        // on a lock held by `other`.
        let mut blocked = false;
        for _ in 0..200 {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity
                 WHERE datname = current_database() AND wait_event_type = 'Lock'",
            )
            .fetch_one(&db)
            .await
            .unwrap();
            if waiting > 0 {
                blocked = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        other.commit().await.unwrap();

        let result = merge.await.unwrap();
        assert!(
            blocked,
            "the merge never waited on the duplicate's row lock"
        );
        assert!(
            matches!(result, Err(crate::Error::Conflict(_))),
            "expected Conflict, got {result:?}"
        );
        let stored = repo.get_by_id(&survivor.id).await.unwrap().unwrap();
        assert_eq!(
            stored.name.family, "RaceSurvivor",
            "the survivor was not written"
        );
    }
}
