//! Person persistence via the repository pattern.
//!
//! [`PersonRepository`] is the storage-agnostic trait the rest of the
//! service depends on; [`PgPersonRepository`] is the PostgreSQL
//! implementation. The implementation maps the rich domain [`Person`](crate::models::Person)
//! to/from the normalized child tables (names, identifiers, addresses,
//! contacts, links, documents, emergency contacts, photos), wraps
//! multi-table writes in a transaction, performs soft deletes, and — when
//! configured — publishes a [`PersonEvent`](crate::streaming::PersonEvent)
//! and writes an audit row for every mutation. [`AuditContext`] carries
//! who/where provenance into those audit rows.
//!
//! Every statement against the person tables is here, as a `query!` /
//! `query_as!` / `query_scalar!` literal, including the record-integrity
//! reads behind `/api/records/verify` and the `integrity_resign` task.
//!
//! ## Record digests (INT-V1)
//!
//! Each write stamps the record's digests **over the record as stored**,
//! read back inside the write's own transaction ([`stamp_digests`]), not
//! over the value the caller sent. The two differ: storage drops an
//! address's `use`, truncates a time of death to microseconds, and skips a
//! telecom entry it cannot tag, so a digest over the caller's value made
//! a fresh record fail `/api/records/verify`.

// SEC-G4: user text in a `LIKE` pattern is escaped so it matches literally.
use axa::cursor_pagination::escape_like;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::models::{
    AddressRow, ContactRow, DocumentRow, EmergencyContactRow, EmergencyTelecomRow, IdentifierRow,
    LinkRow, NameRow, PersonRow,
};
use super::{not_updated, to_i64};
use crate::Result;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Address, ContactPoint, ContactPointSystem, DocumentType, EmergencyContact, HumanName,
    Identifier, IdentityDocument, Person, PersonLink,
};
use crate::streaming::{EventKind, EventTransport};

/// Serialize a fieldless enum to its canonical serde string tag (used for
/// the document/telecom/address enum columns).
fn enum_to_tag<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

/// Parse a stored string tag back into a fieldless enum, or `None` if it is
/// absent or unrecognized.
fn tag_to_enum<T: serde::de::DeserializeOwned>(tag: Option<&String>) -> Option<T> {
    let t = tag?;
    serde_json::from_value(serde_json::Value::String(t.clone())).ok()
}

/// A list position as the `INTEGER` column takes it.
fn position(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

/// The `gender` column's value: the DB CHECK constraint enforces lowercase
/// (`male`/`female`/`other`/`unknown`); `Gender`'s serde
/// `rename_all = "lowercase"` produces the same shape.
fn gender_tag(person: &Person) -> String {
    format!("{:?}", person.gender).to_lowercase()
}

/// Insert the `persons` parent row for `person`, with `created_at` and
/// `updated_at` both the transaction's `CURRENT_TIMESTAMP` and no digests:
/// [`stamp_digests`] writes those once the child rows are in.
///
/// The transaction's time, not this process's clock, because the digest
/// stamp is an `UPDATE`, and the `update_patients_updated_at` trigger sets
/// `updated_at` to `CURRENT_TIMESTAMP` as it lands. Stamping `created_at`
/// from the same clock keeps the two equal on a fresh record, as they were.
async fn insert_person_row(
    conn: &mut PgConnection,
    person: &Person,
    created_by: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO persons
             (id, active, gender, birth_date, tax_id, deceased, deceased_datetime,
              marital_status, multiple_birth, managing_organization_id, created_at, updated_at,
              created_by, updated_by, deleted_at, deleted_by, content_hash, content_hash_sha3,
              content_mac)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP,
                 $11, NULL, NULL, NULL, NULL, NULL, NULL)",
        person.id,
        person.active,
        gender_tag(person),
        person.birth_date,
        person.tax_id,
        person.deceased,
        person.deceased_datetime,
        person.marital_status,
        person.multiple_birth,
        person.managing_organization,
        created_by,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Insert one `person_names` row.
async fn insert_name(
    conn: &mut PgConnection,
    person_id: Uuid,
    name: &HumanName,
    is_primary: bool,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO person_names
             (id, person_id, use_type, family, given, prefix, suffix, is_primary, created_at,
              updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        Uuid::new_v4(),
        person_id,
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

/// Insert every child row of `person` on `conn`: the primary name
/// (flagged `is_primary = true`) and additional names, identifiers,
/// addresses and contacts (the first of each flagged primary by
/// position), links, documents, emergency contacts with their telecom
/// rows, and photos.
///
/// `NameUse` / `IdentifierUse` / `ContactPointSystem` / `ContactPointUse`
/// / `LinkType` go through [`enum_to_tag`] (their own
/// `#[serde(rename_all = "lowercase")]`). PERSON-CONTACT-CASE found that
/// these five once used `format!("{:?}")` (`PascalCase`: `"Old"`,
/// `"Phone"`, `"Replaces"`), which every one of these columns'
/// lowercase-only CHECK constraints rejects with a `500 DATABASE_ERROR`.
/// An address's `use` is not stored (`use_type` is written `NULL`).
async fn insert_children(conn: &mut PgConnection, person: &Person) -> Result<()> {
    insert_name(conn, person.id, &person.name, true).await?;
    for name in &person.additional_names {
        insert_name(conn, person.id, name, false).await?;
    }
    for id in &person.identifiers {
        sqlx::query!(
            "INSERT INTO person_identifiers
                 (id, person_id, use_type, identifier_type, system, value, assigner, created_at,
                  updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            Uuid::new_v4(),
            person.id,
            id.use_type.as_ref().and_then(enum_to_tag),
            // `IdentifierType`'s `#[serde(rename_all = "UPPERCASE")]`
            // matches every named variant (`MRN`, `SSN`, …) and `Other`
            // (`OTHER`), which the CHECK constraint wants.
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
    for (idx, addr) in person.addresses.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO person_addresses
                 (id, person_id, use_type, line1, line2, city, state, postal_code, country,
                  is_primary, created_at, updated_at)
             VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            Uuid::new_v4(),
            person.id,
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
    for (idx, cp) in person.telecom.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO person_contacts
                 (id, person_id, system, value, use_type, is_primary, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            Uuid::new_v4(),
            person.id,
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
    for link in &person.links {
        sqlx::query!(
            "INSERT INTO person_links
                 (id, person_id, other_person_id, link_type, created_at, created_by)
             VALUES ($1, $2, $3, $4, $5, NULL)",
            Uuid::new_v4(),
            person.id,
            link.other_person_id,
            // `LinkType`'s `#[serde(rename_all = "lowercase")]` matches the
            // CHECK constraint for `Replaces`/`Refer`/`Seealso`;
            // `ReplacedBy` carries its own `#[serde(rename = "replaced_by")]`
            // (PRO-P2, 2026-08-29).
            enum_to_tag(&link.link_type).unwrap_or_default(),
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
    }
    insert_extra_collections(conn, person).await
}

/// Insert the normalized document / emergency-contact (+ telecom) / photo
/// child rows for `person` on connection `conn`.
async fn insert_extra_collections(conn: &mut PgConnection, person: &Person) -> Result<()> {
    for (i, doc) in person.documents.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO person_documents
                 (id, person_id, document_type, number, issuing_country, issuing_authority,
                  issue_date, expiry_date, verified, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            Uuid::new_v4(),
            person.id,
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
    for (i, ec) in person.emergency_contacts.iter().enumerate() {
        let ec_id = Uuid::new_v4();
        let addr = ec.address.as_ref();
        sqlx::query!(
            "INSERT INTO person_emergency_contacts
                 (id, person_id, name, relationship, is_primary, address_use_type,
                  address_line1, address_line2, address_city, address_state,
                  address_postal_code, address_country, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            ec_id,
            person.id,
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
                "INSERT INTO person_emergency_contact_telecom
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
    for (i, url) in person.photo.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO person_photos (id, person_id, url, position) VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            person.id,
            url,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Delete every child row of person `id` on `conn`. Deleting the
/// emergency contacts cascades to their telecom rows.
async fn delete_children(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!("DELETE FROM person_names WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM person_identifiers WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM person_addresses WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM person_contacts WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM person_links WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!(
        "DELETE FROM person_emergency_contacts WHERE person_id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM person_documents WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM person_photos WHERE person_id = $1", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Apply a person's parent-row update and wholesale child-row replacement
/// on `conn`, then stamp its digests over the record as stored. Shared by
/// [`PersonRepository::update`] and [`PersonRepository::merge`]: preserves
/// `created_*`, stamps `updated_at` to now and `updated_by` to `ctx`'s
/// actor, deletes every child row for the person, then re-inserts them
/// from the domain model. Runs on the caller's transaction so it shares
/// that commit boundary.
///
/// An id that matches no row is an error, as it was under `SeaORM`.
async fn apply_update_rows(
    conn: &mut PgConnection,
    person: &Person,
    ctx: &AuditContext,
) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE persons
         SET active = $2, gender = $3, birth_date = $4, tax_id = $5, deceased = $6,
             deceased_datetime = $7, marital_status = $8, multiple_birth = $9,
             managing_organization_id = $10, updated_at = $11, updated_by = $12
         WHERE id = $1",
        person.id,
        person.active,
        gender_tag(person),
        person.birth_date,
        person.tax_id,
        person.deceased,
        person.deceased_datetime,
        person.marital_status,
        person.multiple_birth,
        person.managing_organization,
        Utc::now(),
        // SEC-B8: the real caller when one is known.
        ctx.user_id,
    )
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }

    delete_children(conn, person.id).await?;
    insert_children(conn, person).await?;
    stamp_digests(conn, person.id).await
}

/// Write every digest of person `id` over the record **as stored**, read
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
    let row = fetch_person_row(conn, id, true).await?.ok_or_else(|| {
        crate::Error::Validation("person not found while stamping digests".into())
    })?;
    let deleted_at = row.deleted_at.map(|d| d.timestamp_micros());
    let stored = assemble(conn, row).await?;
    let d = crate::compliance::record_integrity::digests_with_deleted_at(&stored, deleted_at)?;
    sqlx::query!(
        "UPDATE persons SET content_hash = $2, content_hash_sha3 = $3, content_mac = $4
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

/// Soft-delete the person `id` on `conn` by stamping `deleted_at` /
/// `deleted_by` (from `ctx`'s actor), leaving every other column (and the
/// child rows) in place, then restamping every digest over the row as now
/// stored. Factored from [`PersonRepository::delete`]'s soft-delete so
/// [`PersonRepository::merge`] can reuse it on its own transaction.
///
/// PE-D1: `deleted_at` is part of the record's pre-image, so a soft
/// delete changes all three digests. It used to rewrite only
/// `content_hash`, from the caller's pre-loaded copy, leaving
/// `content_hash_sha3` and `content_mac` describing the live record.
/// [`stamp_digests`] now writes all three over the stored row, on `conn`,
/// so a caller passing its transaction gets them in the same write.
///
/// An id that matches no row is an error, as it was under `SeaORM`.
async fn apply_soft_delete_row(
    conn: &mut PgConnection,
    id: &Uuid,
    ctx: &AuditContext,
) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE persons SET deleted_at = $2, deleted_by = $3 WHERE id = $1",
        id,
        Utc::now(),
        // SEC-B8: the real caller when one is known.
        ctx.user_id.clone().or_else(|| Some("system".to_string())),
    )
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }
    stamp_digests(conn, *id).await
}

/// Read one `persons` row; a soft-deleted row only when `include_deleted`.
async fn fetch_person_row(
    conn: &mut PgConnection,
    id: Uuid,
    include_deleted: bool,
) -> Result<Option<PersonRow>> {
    let row = sqlx::query_as!(
        PersonRow,
        "SELECT id, active, gender, birth_date, tax_id, deceased, deceased_datetime,
                marital_status, multiple_birth, managing_organization_id, created_at,
                updated_at, created_by, updated_by, deleted_at, deleted_by, content_hash,
                content_hash_sha3, content_mac
         FROM persons
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
/// [`Person`].
async fn assemble(conn: &mut PgConnection, row: PersonRow) -> Result<Person> {
    let id = row.id;
    let names = sqlx::query_as!(
        NameRow,
        "SELECT id, person_id, use_type, family, given, prefix, suffix, is_primary
         FROM person_names WHERE person_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let identifiers = sqlx::query_as!(
        IdentifierRow,
        "SELECT id, person_id, use_type, identifier_type, system, value, assigner
         FROM person_identifiers WHERE person_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let addresses = sqlx::query_as!(
        AddressRow,
        "SELECT id, person_id, line1, line2, city, state, postal_code, country
         FROM person_addresses WHERE person_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let contacts = sqlx::query_as!(
        ContactRow,
        "SELECT id, person_id, system, value, use_type
         FROM person_contacts WHERE person_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let links = sqlx::query_as!(
        LinkRow,
        "SELECT id, person_id, other_person_id, link_type
         FROM person_links WHERE person_id = $1",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut person = PgPersonRepository::from_db_models(
        row,
        &names,
        &identifiers,
        &addresses,
        &contacts,
        &links,
    )?;
    load_extra_collections(conn, &mut person).await?;
    Ok(person)
}

/// Read and assemble person `id` on `conn`; a soft-deleted one only when
/// `include_deleted`.
async fn read_person(
    conn: &mut PgConnection,
    id: Uuid,
    include_deleted: bool,
) -> Result<Option<Person>> {
    match fetch_person_row(conn, id, include_deleted).await? {
        Some(row) => Ok(Some(assemble(conn, row).await?)),
        None => Ok(None),
    }
}

/// Load the normalized document / emergency-contact / photo child rows
/// for `person.id` and populate them onto `person`.
async fn load_extra_collections(conn: &mut PgConnection, person: &mut Person) -> Result<()> {
    let id = person.id;

    let doc_rows = sqlx::query_as!(
        DocumentRow,
        "SELECT document_type, number, issuing_country, issuing_authority, issue_date,
                expiry_date, verified
         FROM person_documents WHERE person_id = $1 ORDER BY position ASC",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    person.documents = doc_rows
        .into_iter()
        .map(|r| IdentityDocument {
            document_type: tag_to_enum(Some(&r.document_type)).unwrap_or(DocumentType::Other),
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
         FROM person_emergency_contacts WHERE person_id = $1 ORDER BY position ASC",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut emergency_contacts = Vec::with_capacity(ec_rows.len());
    for ec in ec_rows {
        let tel_rows = sqlx::query_as!(
            EmergencyTelecomRow,
            "SELECT system, value, use_type
             FROM person_emergency_contact_telecom
             WHERE emergency_contact_id = $1 ORDER BY position ASC",
            ec.id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let telecom = tel_rows
            .into_iter()
            .map(|t| ContactPoint {
                system: tag_to_enum(Some(&t.system)).unwrap_or(ContactPointSystem::Other),
                value: t.value,
                use_type: tag_to_enum(t.use_type.as_ref()),
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
            use_type: tag_to_enum(ec.address_use_type.as_ref()),
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
    person.emergency_contacts = emergency_contacts;

    let photos = sqlx::query_scalar!(
        "SELECT url FROM person_photos WHERE person_id = $1 ORDER BY position ASC",
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    person.photo = photos;

    Ok(())
}

/// Request provenance attached to audit-log writes.
///
/// Defaults to a `"system"` actor with no network metadata; handlers
/// populate it from the inbound HTTP request when available.
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Authenticated user id, or `None` for anonymous/system actions.
    pub user_id: Option<String>,
    /// Originating client IP address, if known.
    pub ip_address: Option<String>,
    /// Originating `User-Agent` header, if known.
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    /// A `"system"` actor with no IP or user-agent.
    fn default() -> Self {
        Self {
            user_id: Some("system".to_string()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// The side effects of a create or update made on a caller's
/// transaction ([`PersonRepository::create_on`] /
/// [`PersonRepository::update_on`]): the in-memory event and the
/// best-effort audit row. Run them with
/// [`PersonRepository::after_commit`] once that transaction has
/// committed; dropping this on a rollback is correct, since an event or
/// audit row for a write that never landed would describe a record that
/// does not exist.
#[must_use = "run it with `after_commit` once the transaction has committed"]
pub struct AfterCommit {
    /// The event to publish.
    event: crate::streaming::PersonEvent,
    /// The audit row to write: `(action, id, old values, new values)`.
    audit: Option<AuditEntry>,
    /// The acting caller, for the audit row.
    ctx: AuditContext,
}

/// One pending audit row: `(action, id, old values, new values)`.
type AuditEntry = (
    &'static str,
    Uuid,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
);

/// Storage-agnostic CRUD + search interface for [`Person`] records.
///
/// `Send + Sync` so it can be shared as `Arc<dyn PersonRepository>`
/// across async handlers.
#[async_trait::async_trait]
pub trait PersonRepository: Send + Sync {
    /// Persist a new person (and its child rows) and return the stored
    /// form. `ctx` is the acting caller (SEC-B8): it is stamped onto the
    /// row's `created_by` and carried into the `CREATE` audit row, instead
    /// of the previously-hard-coded `"system"` actor.
    async fn create(&self, person: &Person, ctx: &AuditContext) -> Result<Person>;

    /// Fetch a non-deleted person by id, or `None` if absent/soft-deleted.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Person>>;

    /// [`Self::get_by_id`] on the caller's connection `conn`, so a caller
    /// holding a transaction (and a lock on it) reads without taking a
    /// second pooled connection.
    async fn get_by_id_on(&self, conn: &mut PgConnection, id: &Uuid) -> Result<Option<Person>>;

    /// [`Self::create`]'s writes on the caller's open transaction `conn`:
    /// the rows, the digests and (under the outbox transport) the outbox
    /// row. Returns the stored form and the [`AfterCommit`] side effects,
    /// which the caller runs once it has committed. This is what lets a
    /// caller hold a lock across the write: the bulk import takes its
    /// stable-key advisory lock (SEC-B3) on `conn` and writes on it too, so
    /// one pooled connection is enough (PE-B1).
    async fn create_on(
        &self,
        conn: &mut PgConnection,
        person: &Person,
        ctx: &AuditContext,
    ) -> Result<(Person, AfterCommit)>;

    /// [`Self::update`]'s writes on the caller's open transaction `conn`,
    /// as [`Self::create_on`].
    async fn update_on(
        &self,
        conn: &mut PgConnection,
        person: &Person,
        ctx: &AuditContext,
    ) -> Result<(Person, AfterCommit)>;

    /// Publish the event and write the audit row of a
    /// [`Self::create_on`] / [`Self::update_on`] whose transaction has
    /// committed.
    async fn after_commit(&self, after: AfterCommit);

    /// Replace a person and all its child rows, returning the new state.
    /// `ctx` is the acting caller (SEC-B8): stamped onto `updated_by` and
    /// carried into the `UPDATE` audit row.
    async fn update(&self, person: &Person, ctx: &AuditContext) -> Result<Person>;

    /// Merge the `duplicate_id` record into `survivor`: apply the
    /// survivor's parent + child row updates and soft-delete the duplicate
    /// **in one transaction**, so the whole merge commits (or rolls back)
    /// atomically. Under the outbox transport this enqueues a `Merged`
    /// outbox row for the survivor (carrying the duplicate's pid via
    /// `merged_from`) and a `Deleted` outbox row for the duplicate on the
    /// same transaction. Returns the reloaded survivor. `ctx` is the
    /// acting caller (SEC-B8): the one operator who initiated the merge,
    /// stamped onto both the survivor's `updated_by` and the duplicate's
    /// `deleted_by`, and carried into both the `UPDATE` and `DELETE`
    /// audit rows.
    async fn merge(
        &self,
        survivor: &Person,
        duplicate_id: &Uuid,
        ctx: &AuditContext,
    ) -> Result<Person>;

    /// Soft-delete a person (sets `deleted_at`; row is retained). `ctx` is
    /// the acting caller (SEC-B8): stamped onto `deleted_by` and carried
    /// into the `DELETE` audit row.
    async fn delete(&self, id: &Uuid, ctx: &AuditContext) -> Result<()>;

    /// Find persons whose family name matches `query` (case-insensitive).
    async fn search(&self, query: &str) -> Result<Vec<Person>>;

    /// Page through active, non-deleted persons via `limit`/`offset`.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Person>>;
}

/// `PostgreSQL` [`PersonRepository`] backed by sqlx.
///
/// Optionally fans out to an event publisher and audit log; both are set
/// via the `with_*` builder methods and are no-ops when absent.
pub struct PgPersonRepository {
    /// The connection pool (cheap to clone).
    db: PgPool,
    /// Optional sink for [`PersonEvent`](crate::streaming::PersonEvent)s.
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    /// Optional audit-trail writer.
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the legacy post-commit
    /// in-memory publish; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl PgPersonRepository {
    /// Build a repository over `db` with no event/audit sinks attached and
    /// the default [`EventTransport::Memory`] transport (behaviour
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
    /// `AppState` wires this from `PERSON_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `person` **on `conn`** — pass the open
    /// transaction so the row commits with the entity write (the outbox
    /// atomicity guarantee). A no-op under [`EventTransport::Memory`].
    async fn enqueue_outbox(
        &self,
        conn: &mut PgConnection,
        person: &Person,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(person, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Attach an event publisher; returns `self` for chaining.
    #[must_use]
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Attach an audit-log repository; returns `self` for chaining.
    #[must_use]
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publish `event` if a publisher is set; log and swallow any error.
    fn publish_event(&self, event: crate::streaming::PersonEvent) {
        if let Some(ref publisher) = self.event_publisher
            && let Err(e) = publisher.publish(event)
        {
            tracing::error!("Failed to publish event: {}", e);
        }
    }

    /// Write an audit row for `action` if an audit log is configured.
    ///
    /// Dispatches to the matching `log_create`/`log_update`/`log_delete`
    /// helper; unknown actions are ignored. Errors are logged, not raised.
    async fn log_audit(
        &self,
        action: &str,
        entity_id: uuid::Uuid,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        context: &AuditContext,
    ) {
        if let Some(ref audit_log) = self.audit_log {
            let result = match action {
                "CREATE" => {
                    audit_log
                        .log_create(
                            "Person",
                            entity_id,
                            new_values.unwrap_or(serde_json::Value::Null),
                            context,
                        )
                        .await
                }
                "UPDATE" => {
                    audit_log
                        .log_update(
                            "Person",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            new_values.unwrap_or(serde_json::Value::Null),
                            context,
                        )
                        .await
                }
                "DELETE" => {
                    audit_log
                        .log_delete(
                            "Person",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            context,
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

    /// Rebuild a domain [`HumanName`] from a stored `person_names` row,
    /// parsing the stored lowercase `use_type` tag back to its
    /// [`NameUse`](crate::models::NameUse) variant via [`tag_to_enum`].
    fn human_name_from_db(n: &NameRow) -> HumanName {
        let use_type = tag_to_enum(n.use_type.as_ref());
        HumanName {
            use_type,
            family: n.family.clone(),
            given: n.given.clone(),
            prefix: n.prefix.clone(),
            suffix: n.suffix.clone(),
        }
    }

    /// Rebuild a domain [`Identifier`] from a stored `person_identifiers`
    /// row, parsing the stringified `identifier_type` / `use_type`.
    fn identifier_from_db(id: &IdentifierRow) -> Identifier {
        use crate::models::IdentifierType;
        // Falls back to `Other` for an unrecognized tag — covers both the
        // literal `'OTHER'` row and any forward-compatible unknown value.
        let identifier_type =
            tag_to_enum(Some(&id.identifier_type)).unwrap_or(IdentifierType::Other);
        let use_type = tag_to_enum(id.use_type.as_ref());
        Identifier {
            identifier_type,
            use_type,
            system: id.system.clone(),
            value: id.value.clone(),
            assigner: id.assigner.clone(),
        }
    }

    /// Reassemble a domain [`Person`] from its parent row and child rows.
    ///
    /// Parses stringified enums back to their domain variants (unknown
    /// values fall back to `Other`/`Unknown` or are dropped). Errors if no
    /// primary name is present. `documents`, `emergency_contacts` and
    /// `photo` come back empty and are loaded by `load_extra_collections`.
    fn from_db_models(
        db_person: PersonRow,
        db_names: &[NameRow],
        db_identifiers: &[IdentifierRow],
        db_addresses: &[AddressRow],
        db_contacts: &[ContactRow],
        db_links: &[LinkRow],
    ) -> Result<Person> {
        use crate::models::{ContactPointSystem, Gender, LinkType};

        // Parse gender. DB stores lowercase per CHECK constraint
        // ('male'/'female'/'other'/'unknown'); accept PascalCase too
        // so rows written by older code (when persisted via the
        // legacy `format!("{:?}", …)` path) still round-trip.
        let gender = match db_person.gender.to_lowercase().as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            "other" => Gender::Other,
            _ => Gender::Unknown,
        };

        // Get primary name
        let primary_name = db_names
            .iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Person has no primary name".to_string()))?;
        let name = Self::human_name_from_db(primary_name);

        // Additional names
        let additional_names = db_names
            .iter()
            .filter(|n| !n.is_primary)
            .map(Self::human_name_from_db)
            .collect();

        // Identifiers
        let identifiers = db_identifiers
            .iter()
            .map(Self::identifier_from_db)
            .collect();

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

        // Telecom
        let telecom = db_contacts
            .iter()
            .filter_map(|cp| {
                let system: ContactPointSystem = tag_to_enum(Some(&cp.system))?;
                let use_type = tag_to_enum(cp.use_type.as_ref());

                Some(ContactPoint {
                    system,
                    value: cp.value.clone(),
                    use_type,
                })
            })
            .collect();

        // Links. A row this crate itself wrote always has a recognized
        // tag; `filter_map` only guards against a hand-edited row.
        let links = db_links
            .iter()
            .filter_map(|link| {
                let link_type: LinkType = tag_to_enum(Some(&link.link_type))?;
                Some(PersonLink {
                    other_person_id: link.other_person_id,
                    link_type,
                })
            })
            .collect();

        Ok(Person {
            id: db_person.id,
            identifiers,
            active: db_person.active,
            name,
            additional_names,
            telecom,
            gender,
            birth_date: db_person.birth_date,
            deceased: db_person.deceased,
            deceased_datetime: db_person.deceased_datetime,
            addresses,
            marital_status: db_person.marital_status,
            multiple_birth: db_person.multiple_birth,
            tax_id: db_person.tax_id,
            // Loaded separately from child tables by `load_extra_collections`.
            documents: vec![],
            emergency_contacts: vec![],
            photo: vec![],
            managing_organization: db_person.managing_organization_id,
            links,
            created_at: db_person.created_at,
            updated_at: db_person.updated_at,
        })
    }
}

#[async_trait::async_trait]
impl PersonRepository for PgPersonRepository {
    /// Insert the person and all child rows in one transaction, stamp its
    /// digests over the stored record, then publish a `Created` event and
    /// write a CREATE audit row ([`Self::create_on`], then
    /// [`Self::after_commit`]).
    async fn create(&self, person: &Person, ctx: &AuditContext) -> Result<Person> {
        let mut txn = self.db.begin().await?;
        let (result, after) = self.create_on(&mut txn, person, ctx).await?;
        txn.commit().await?;
        self.after_commit(after).await;
        Ok(result)
    }

    /// Load a person and its associations, skipping soft-deleted rows.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Person>> {
        let mut conn = self.db.acquire().await?;
        read_person(&mut conn, *id, false).await
    }

    async fn get_by_id_on(&self, conn: &mut PgConnection, id: &Uuid) -> Result<Option<Person>> {
        read_person(conn, *id, false).await
    }

    async fn create_on(
        &self,
        conn: &mut PgConnection,
        person: &Person,
        ctx: &AuditContext,
    ) -> Result<(Person, AfterCommit)> {
        // SEC-B8: stamp the real caller when one is known.
        insert_person_row(conn, person, ctx.user_id.as_deref()).await?;
        insert_children(conn, person).await?;
        // Digests over the record as stored, in this transaction (INT-V1).
        stamp_digests(conn, person.id).await?;

        // Durable event bus (Phase 2): under the outbox transport, write
        // the `event_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the post-commit in-memory publish.
        self.enqueue_outbox(conn, person, EventKind::Created)
            .await?;

        let result = read_person(conn, person.id, true)
            .await?
            .ok_or_else(|| crate::Error::Validation("Person not found after create".to_string()))?;
        let audit = serde_json::to_value(&result)
            .ok()
            .map(|json| ("CREATE", result.id, None, Some(json)));
        let after = AfterCommit {
            event: crate::streaming::PersonEvent::Created {
                person: result.clone(),
                timestamp: Utc::now(),
            },
            audit,
            ctx: ctx.clone(),
        };
        Ok((result, after))
    }

    /// Update the parent row, then delete-and-reinsert all child rows in
    /// one transaction (a simple full-replace), publish an `Updated`
    /// event, and write an UPDATE audit row with the before/after JSON
    /// ([`Self::update_on`], then [`Self::after_commit`]).
    async fn update(&self, person: &Person, ctx: &AuditContext) -> Result<Person> {
        let mut txn = self.db.begin().await?;
        let (result, after) = self.update_on(&mut txn, person, ctx).await?;
        txn.commit().await?;
        self.after_commit(after).await;
        Ok(result)
    }

    async fn update_on(
        &self,
        conn: &mut PgConnection,
        person: &Person,
        ctx: &AuditContext,
    ) -> Result<(Person, AfterCommit)> {
        // Old values for audit.
        let old_person = read_person(conn, person.id, false).await?;

        // Parent-row update + wholesale child-row replacement + digest
        // stamp, shared with `merge` (see `apply_update_rows`).
        apply_update_rows(conn, person, ctx).await?;

        // Outbox row shares the update transaction (see `create_on`).
        self.enqueue_outbox(conn, person, EventKind::Updated)
            .await?;

        let result = read_person(conn, person.id, false)
            .await?
            .ok_or_else(|| crate::Error::Validation("Person not found after update".to_string()))?;
        let audit = old_person
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
            .zip(serde_json::to_value(&result).ok())
            .map(|(old, new)| ("UPDATE", result.id, Some(old), Some(new)));
        let after = AfterCommit {
            event: crate::streaming::PersonEvent::Updated {
                person: result.clone(),
                timestamp: Utc::now(),
            },
            audit,
            ctx: ctx.clone(),
        };
        Ok((result, after))
    }

    async fn after_commit(&self, after: AfterCommit) {
        self.publish_event(after.event);
        if let Some((action, id, old, new)) = after.audit {
            self.log_audit(action, id, old, new, &after.ctx).await;
        }
    }

    /// Merge `duplicate_id` into `survivor` atomically: apply the
    /// survivor's parent + child updates and soft-delete the duplicate in
    /// one transaction. Under the outbox transport, a `Merged` outbox row
    /// for the survivor (carrying the duplicate's pid via `merged_from`)
    /// and a `Deleted` outbox row for the duplicate are enqueued on the
    /// same transaction, so the whole merge and both events commit (or
    /// roll back) together. The UPDATE/DELETE audit rows are written in
    /// the same transaction (SEC-B10); post-commit it publishes the
    /// in-memory `Merged` + `Deleted` events.
    async fn merge(
        &self,
        survivor: &Person,
        duplicate_id: &Uuid,
        ctx: &AuditContext,
    ) -> Result<Person> {
        // Snapshots for the audit trail: the survivor's pre-image for the
        // UPDATE row, the duplicate's pre-image for the DELETE row (and to
        // build the duplicate's `Deleted` outbox envelope).
        let old_survivor = self.get_by_id(&survivor.id).await?;
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let mut txn = self.db.begin().await?;

        // SEC-B5 (TOCTOU): the merged payload was computed from unlocked
        // reads, so lock both participant rows `FOR UPDATE` before writing
        // and re-validate under the lock. Two concurrent merges of the same
        // duplicate would otherwise both see it active and fan its data into
        // two different survivors; the row lock serialises them and the
        // active re-check makes the loser fail closed instead of
        // double-applying. Lock in id order so the two directions cannot
        // deadlock.
        let mut lock_ids = [survivor.id, *duplicate_id];
        lock_ids.sort();
        for id in lock_ids {
            sqlx::query_scalar!(
                "SELECT id FROM persons WHERE id = $1 LIMIT 1 FOR UPDATE",
                id
            )
            .fetch_optional(&mut *txn)
            .await?;
        }
        let duplicate_still_active = sqlx::query_scalar!(
            "SELECT id FROM persons WHERE id = $1 AND deleted_at IS NULL LIMIT 1",
            duplicate_id,
        )
        .fetch_optional(&mut *txn)
        .await?
        .is_some();
        if !duplicate_still_active {
            txn.rollback().await?;
            return Err(crate::Error::Validation(format!(
                "duplicate person {duplicate_id} is no longer active (already merged or deleted)"
            )));
        }

        // Apply the survivor's parent + child rows (shared with `update`).
        // The duplicate's identifiers move to the survivor, and
        // `person_identifiers` is unique on `(system, value)` across the
        // table, so the duplicate's rows must go first: written with them
        // still in place, every merge of a duplicate holding an identifier
        // violated the key and answered `500`. Its pre-image, identifiers
        // and all, stays in the audit trail's DELETE row.
        sqlx::query!(
            "DELETE FROM person_identifiers WHERE person_id = $1",
            duplicate_id
        )
        .execute(&mut *txn)
        .await?;
        apply_update_rows(&mut txn, survivor, ctx).await?;

        // Soft-delete the duplicate (shared with `delete`), restamping all
        // three digests over its stored row on this transaction (PE-D1),
        // so its tombstone does not read as tampered. The same `ctx`
        // applies to both halves of the merge — one operator initiated
        // the whole action.
        apply_soft_delete_row(&mut txn, duplicate_id, ctx).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor (carrying the duplicate's pid via `merged_from`, so a
        // merge-repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
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

        // SEC-B10: write the merge audit rows **inside** the transaction so
        // they commit atomically with the merge — a crash after commit can no
        // longer leave a durable merge with no audit trail. An audit-write
        // failure rolls the whole merge back. The survivor's new-value is its
        // applied payload (`survivor`), the same data written by
        // `apply_update_rows`. SEC-B8: both rows carry the real acting
        // operator (`ctx`).
        if let Some(ref audit_log) = self.audit_log {
            if let Some(old_json) = old_survivor
                .as_ref()
                .and_then(|p| serde_json::to_value(p).ok())
                && let Ok(new_json) = serde_json::to_value(survivor)
            {
                audit_log
                    .log_update_on(&mut *txn, "Person", survivor.id, old_json, new_json, ctx)
                    .await?;
            }
            if let Some(dup_json) = old_duplicate
                .as_ref()
                .and_then(|p| serde_json::to_value(p).ok())
            {
                audit_log
                    .log_delete_on(&mut *txn, "Person", *duplicate_id, dup_json, ctx)
                    .await?;
            }
        }

        txn.commit().await?;

        let result = self
            .get_by_id(&survivor.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Person not found after merge".to_string()))?;

        // Post-commit in-memory stream (memory transport keeps this): a
        // `Merged` for the survivor and a `Deleted` for the duplicate.
        self.publish_event(crate::streaming::PersonEvent::Merged {
            source_id: *duplicate_id,
            target_id: result.id,
            timestamp: Utc::now(),
        });
        self.publish_event(crate::streaming::PersonEvent::Deleted {
            person_id: *duplicate_id,
            timestamp: Utc::now(),
        });

        Ok(result)
    }

    /// Soft-delete by stamping `deleted_at`/`deleted_by`; child rows are
    /// retained. Publishes a `Deleted` event and writes a DELETE audit row.
    async fn delete(&self, id: &Uuid, ctx: &AuditContext) -> Result<()> {
        // Get old values for audit
        let old_person = self.get_by_id(id).await?;

        // Soft delete: stamp `deleted_at`/`deleted_by` and restamp the
        // digests (PE-D1), in one transaction under either transport, so
        // no reader sees the new stamp with the old digests. Under the
        // outbox transport the `deleted` outbox row commits with them.
        let mut txn = self.db.begin().await?;
        apply_soft_delete_row(&mut txn, id, ctx).await?;
        if let Some(old) = old_person.as_ref() {
            self.enqueue_outbox(&mut txn, old, EventKind::Deleted)
                .await?;
        }
        txn.commit().await?;

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Deleted {
            person_id: *id,
            timestamp: Utc::now(),
        });

        // Log audit
        if let Some(old_person) = old_person
            && let Ok(old_json) = serde_json::to_value(&old_person)
        {
            self.log_audit("DELETE", *id, Some(old_json), None, ctx)
                .await;
        }

        Ok(())
    }

    /// SQL `LIKE` search over lowercased family name; resolves each
    /// matched person id to a full record. (Tantivy is the richer path.)
    async fn search(&self, query: &str) -> Result<Vec<Person>> {
        // SEC-G4: escape the caller's `LIKE` metacharacters (`%`, `_`, `\`)
        // before wrapping in the `%…%` contains-pattern, so a query of `%`
        // (matches every row) or `_`×N (forces an expensive scan) is treated
        // literally rather than as a wildcard. The value is already a bound
        // parameter (no SQL injection); this closes the wildcard-injection /
        // DoS vector. Postgres `LIKE` treats `\` as the escape char.
        let search_pattern = format!("%{}%", escape_like(&query.to_lowercase()));

        let person_ids = sqlx::query_scalar!(
            "SELECT DISTINCT person_id FROM person_names WHERE LOWER(family) LIKE $1",
            search_pattern,
        )
        .fetch_all(&self.db)
        .await?;

        let mut persons = Vec::new();
        for person_id in person_ids {
            if let Some(person) = self.get_by_id(&person_id).await? {
                persons.push(person);
            }
        }

        Ok(persons)
    }

    /// Page through active, non-deleted persons, hydrating each to a full
    /// record. (One follow-up `get_by_id` per row — fine for modest pages.)
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Person>> {
        // `ORDER BY id` is load-bearing, not cosmetic: without an explicit
        // ORDER BY, Postgres does not guarantee a stable row order across
        // repeated `LIMIT`/`OFFSET` queries, so a caller paginating through
        // every page (bulk export; the link-graph suggestion job's
        // person/worker enumeration, T-31) could silently skip or duplicate
        // rows between pages. Ordering by the primary key is cheap (already
        // indexed) and gives every page a deterministic, non-overlapping
        // slice.
        let ids = sqlx::query_scalar!(
            "SELECT id FROM persons
             WHERE deleted_at IS NULL AND active = true
             ORDER BY id ASC
             LIMIT $1 OFFSET $2",
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(&self.db)
        .await?;

        let mut persons = Vec::new();
        for id in ids {
            if let Some(person) = self.get_by_id(&id).await? {
                persons.push(person);
            }
        }

        Ok(persons)
    }
}

/// One `persons` row's stored digests and soft-delete stamp, as
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
    pub deleted_at: Option<DateTime<Utc>>,
}

/// One record **as stored**, soft-deleted or not, with its stored digests:
/// the input a check of a row's digests needs, including the deleted rows
/// `/api/records/verify` skips (the PE-D1 test reads them through this).
/// `None` when no row has `id`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if a query fails.
pub async fn stored_record(db: &PgPool, id: Uuid) -> Result<Option<(Person, StoredDigests)>> {
    let mut conn = db.acquire().await?;
    let Some(row) = fetch_person_row(&mut conn, id, true).await? else {
        return Ok(None);
    };
    let digests = StoredDigests {
        id: row.id,
        content_hash: row.content_hash.clone(),
        content_hash_sha3: row.content_hash_sha3.clone(),
        content_mac: row.content_mac.clone(),
        deleted_at: row.deleted_at,
    };
    let person = assemble(&mut conn, row).await?;
    Ok(Some((person, digests)))
}

/// The `limit` most recently updated `persons` rows' stored digests, for
/// `/api/records/verify`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn rows_for_verification(db: &PgPool, limit: i64) -> Result<Vec<StoredDigests>> {
    let rows = sqlx::query_as!(
        StoredDigests,
        "SELECT id, content_hash, content_hash_sha3, content_mac, deleted_at
         FROM persons
         ORDER BY updated_at DESC
         LIMIT $1",
        limit,
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Up to `limit` `persons` rows carrying a MAC, in id order — the
/// `integrity_resign` task's walk.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn rows_with_content_mac(db: &PgPool, limit: u64) -> Result<Vec<StoredDigests>> {
    let rows = sqlx::query_as!(
        StoredDigests,
        "SELECT id, content_hash, content_hash_sha3, content_mac, deleted_at
         FROM persons
         WHERE content_mac IS NOT NULL
         ORDER BY id ASC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Replace one person's MAC, the only column `integrity_resign` writes.
/// No matching row is an error, as it was under `SeaORM`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails or matches no
/// row.
pub async fn set_content_mac(db: &PgPool, id: Uuid, mac: Option<String>) -> Result<()> {
    let done = sqlx::query!("UPDATE persons SET content_mac = $2 WHERE id = $1", id, mac)
        .execute(db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }
    Ok(())
}

/// How many rows `persons` holds, deleted or not.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn count_persons(db: &PgPool) -> Result<u64> {
    let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM persons"#)
        .fetch_one(db)
        .await?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// The ids of the persons holding identifier `(system, value)`, deleted or
/// not, for the bulk import's stable-key lookup, on a pool or on the
/// import's own transaction.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn person_ids_with_identifier(
    conn: impl Into<axa::sqlx::Db<'_>>,
    system: &str,
    value: &str,
) -> Result<Vec<Uuid>> {
    let mut conn = conn.into().acquire().await?;
    let ids = sqlx::query_scalar!(
        "SELECT person_id FROM person_identifiers WHERE system = $1 AND value = $2",
        system,
        value,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(ids)
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path.
/// They require a migrated `PostgreSQL` via `DB_URL` and are skipped
/// by a bare `cargo test`; run with
/// `DB_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod tests {
    use super::{AuditContext, PersonRepository, PgPersonRepository, escape_like};
    use crate::db::outbox::Model as OutboxRow;
    use crate::models::{Gender, HumanName, Person};
    use crate::streaming::EventTransport;
    use sqlx::PgPool;

    /// SEC-G4: `LIKE` wildcards in a search query are neutralised so they
    /// match literally and cannot scan every row (`%`) or force pathological
    /// work (`_`). Pure, DB-free.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("smith"), "smith");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }

    async fn connect() -> PgPool {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for DB tests");
        PgPool::connect(&url).await.expect("connect to DB_URL")
    }

    fn a_person(family: &str) -> Person {
        Person::new(
            HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec!["Test".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Unknown,
        )
    }

    /// Create writes a `created` outbox row in the same transaction as the
    /// entity rows.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = PgPersonRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let person = repo
            .create(&a_person("Created"), &AuditContext::default())
            .await
            .unwrap();

        let rows = OutboxRow::rows_for(&db, person.id, Some("created"))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new person");
        assert_eq!(rows[0].entity, "person");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = PgPersonRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo
            .create(&a_person("Survivor"), &AuditContext::default())
            .await
            .unwrap();
        let duplicate = repo
            .create(&a_person("Duplicate"), &AuditContext::default())
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

    /// SEC-B10: the merge audit rows commit **with** the merge — after a
    /// successful merge there is an `UPDATE` audit row for the survivor and a
    /// `DELETE` audit row for the duplicate (written in-transaction, not
    /// best-effort post-commit). SEC-B8: both rows carry the real actor who
    /// initiated the merge, not the hard-coded `"system"` default.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn merge_writes_the_audit_rows_in_transaction() {
        use crate::db::audit::AuditLogRepository;

        let db = connect().await;
        let audit = std::sync::Arc::new(AuditLogRepository::new(db.clone()));
        let repo = PgPersonRepository::new(db.clone()).with_audit_log(audit.clone());

        let survivor = repo
            .create(&a_person("AuditSurvivor"), &AuditContext::default())
            .await
            .unwrap();
        let duplicate = repo
            .create(&a_person("AuditDuplicate"), &AuditContext::default())
            .await
            .unwrap();

        let actor_ctx = AuditContext {
            user_id: Some("operator-merge-42".to_string()),
            ip_address: None,
            user_agent: None,
        };
        repo.merge(&survivor, &duplicate.id, &actor_ctx)
            .await
            .unwrap();

        let survivor_update = audit
            .rows_for(None, survivor.id, Some("UPDATE"))
            .await
            .unwrap();
        assert!(
            !survivor_update.is_empty(),
            "an UPDATE audit row for the merged survivor must exist"
        );
        assert_eq!(
            survivor_update[0].user_id.as_deref(),
            Some("operator-merge-42"),
            "the merge's UPDATE audit row must carry the real actor, not \"system\""
        );

        let duplicate_delete = audit
            .rows_for(None, duplicate.id, Some("DELETE"))
            .await
            .unwrap();
        assert!(
            !duplicate_delete.is_empty(),
            "a DELETE audit row for the merged-away duplicate must exist"
        );
        assert_eq!(
            duplicate_delete[0].user_id.as_deref(),
            Some("operator-merge-42"),
            "the merge's DELETE audit row must carry the real actor, not \"system\""
        );
    }

    /// SEC-B8: `create`/`update` thread the caller's real `AuditContext`
    /// into the per-row `CREATE`/`UPDATE` audit rows, instead of the
    /// previously-hard-coded `"system"` actor built inside the repository.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn create_and_update_audit_rows_carry_the_real_actor() {
        use crate::db::audit::AuditLogRepository;

        let db = connect().await;
        let audit = std::sync::Arc::new(AuditLogRepository::new(db.clone()));
        let repo = PgPersonRepository::new(db.clone()).with_audit_log(audit.clone());

        let create_ctx = AuditContext {
            user_id: Some("operator-create-7".to_string()),
            ip_address: None,
            user_agent: None,
        };
        let mut person = repo
            .create(&a_person("RealActorCreate"), &create_ctx)
            .await
            .unwrap();

        let create_rows = audit
            .rows_for(None, person.id, Some("CREATE"))
            .await
            .unwrap();
        assert_eq!(create_rows.len(), 1, "one CREATE audit row");
        assert_eq!(
            create_rows[0].user_id.as_deref(),
            Some("operator-create-7"),
            "CREATE audit row must carry the real actor, not \"system\""
        );

        let update_ctx = AuditContext {
            user_id: Some("operator-update-9".to_string()),
            ip_address: None,
            user_agent: None,
        };
        person.name.family = "RealActorCreateRenamed".to_string();
        repo.update(&person, &update_ctx).await.unwrap();

        let update_rows = audit
            .rows_for(None, person.id, Some("UPDATE"))
            .await
            .unwrap();
        assert_eq!(update_rows.len(), 1, "one UPDATE audit row");
        assert_eq!(
            update_rows[0].user_id.as_deref(),
            Some("operator-update-9"),
            "UPDATE audit row must carry the real actor, not \"system\""
        );
    }
}
