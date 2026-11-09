//! Database access — the connection pool, the migrator, and the
//! repository. **Every SQL statement this service runs is in this
//! directory** (spec/tech-stack §5.2), written as a `sqlx::query!` /
//! `query_as!` literal so the compiler checks it against the schema.
//!
//! [`PgThingRepository`] round-trips a domain [`Thing`] against a fully
//! normalized relational schema: scalar fields live on the `things` row;
//! the `alternate_names`, `identifiers`, `images`, and `same_as`
//! collections live in dedicated child tables joined by `thing_id`. Writes
//! are transactional (parent + children); updates replace the child rows.

pub mod audit;
pub mod convert;
pub mod models;
pub mod outbox;
pub mod review_queue;

use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::db::outbox::OutboxInsert;
use crate::models::identifier::{IdentifierType, ThingIdentifier};
use crate::models::merge::MergeRecord;
use crate::models::thing::Thing;
use crate::streaming::envelope::{EventKind, EventTransport};
use models::ThingRow;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `thing-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a connection pool from a [`DatabaseConfig`].
///
/// # Errors
///
/// Returns an error if the database connection cannot be established.
pub async fn create_connection(config: &DatabaseConfig) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}

/// Storage-agnostic CRUD interface for [`Thing`] records.
#[async_trait::async_trait]
pub trait ThingRepository: Send + Sync {
    /// Insert a new thing; returns the stored form.
    async fn create(&self, thing: &Thing) -> Result<Thing>;
    /// Fetch a thing by id, returning `None` if absent or soft-deleted.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Thing>>;
    /// Replace an existing thing.
    async fn update(&self, thing: &Thing) -> Result<Thing>;
    /// Soft-delete a thing (sets `is_deleted`/`deleted_at`).
    async fn soft_delete(&self, id: &Uuid) -> Result<()>;
    /// List non-deleted things, newest first, with limit/offset paging.
    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Thing>>;
    /// Record a merge of `duplicate` into `main`.
    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord>;
    /// Apply the `survivor`'s row + child updates and soft-delete the
    /// `duplicate_id` record **in one transaction**, so the whole merge
    /// commits (or rolls back) atomically. Under the outbox transport this
    /// enqueues a `Merged` outbox row for the survivor (carrying the
    /// duplicate's pid via `merged_from`) and a `Deleted` outbox row for
    /// the duplicate on the same transaction. Returns the reloaded
    /// survivor. Merge-history recording, search sync, and the in-memory
    /// event stay in the handler.
    async fn merge(&self, survivor: &Thing, duplicate_id: &Uuid) -> Result<Thing>;
}

/// sqlx-backed [`ThingRepository`] over a `PostgreSQL` connection pool.
pub struct PgThingRepository {
    /// The shared connection pool.
    db: PgPool,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the handler's in-memory
    /// publish untouched; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl PgThingRepository {
    /// Wrap an existing connection pool in a repository, defaulting to the
    /// [`EventTransport::Memory`] transport (behaviour unchanged from
    /// before the outbox landed).
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            transport: EventTransport::Memory,
        }
    }

    /// Builder: select the event transport (see [`EventTransport`]).
    /// `AppState` wires this from `THING_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `thing` **on `conn`** — pass the open
    /// transaction so the row commits with the entity write (the outbox
    /// atomicity guarantee). A no-op under [`EventTransport::Memory`].
    async fn enqueue_outbox(
        &self,
        conn: &mut PgConnection,
        thing: &Thing,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(thing, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Read one `things` row by id, soft-deleted or not.
    async fn find_row(&self, id: Uuid) -> Result<Option<ThingRow>> {
        let mut conn = self.db.acquire().await?;
        Self::find_row_on(&mut conn, id).await
    }

    /// [`Self::find_row`] on `conn`, so a write can read inside its own
    /// transaction.
    async fn find_row_on(conn: &mut PgConnection, id: Uuid) -> Result<Option<ThingRow>> {
        let row = sqlx::query_as!(
            ThingRow,
            "SELECT id, name, description, disambiguating_description, additional_type,
                    url, main_entity_of_page, owner, subject_of, potential_action,
                    is_deleted, content_hash, content_hash_sha3, content_mac,
                    deleted_at, created_at, updated_at
             FROM things
             WHERE id = $1",
            id,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The assembled record for `id` as stored, read on `conn`. `None`
    /// when absent, and when soft-deleted unless `include_deleted`.
    /// `get_by_id`, which `GET /api/records/verify` reassembles through,
    /// and the digest stamp both read here, so a digest covers exactly
    /// the record verification recomputes.
    async fn read_thing(
        conn: &mut PgConnection,
        id: Uuid,
        include_deleted: bool,
    ) -> Result<Option<Thing>> {
        let Some(row) = Self::find_row_on(conn, id).await? else {
            return Ok(None);
        };
        if row.is_deleted && !include_deleted {
            return Ok(None);
        }
        Ok(Some(Self::hydrate(conn, row).await?))
    }

    /// Stamp `id`'s three digests over the record **as stored**, on the
    /// write's own connection, so they commit with the write (INT-V1).
    ///
    /// Digesting the domain value a caller passed in is what made every
    /// fresh record fail verification: `create` and `update` replace its
    /// timestamps with the server's after digesting it, and Postgres
    /// keeps microseconds where the clock gives nanoseconds, so the
    /// value read back was never the value hashed. Reading the record
    /// back inside the transaction makes the digest cover what
    /// verification recomputes, whatever the database normalised.
    ///
    /// `things` has no `updated_at` trigger, and this `UPDATE` writes only
    /// the three digest columns, none of which is in the pre-image, so
    /// the stamp cannot change what it covers. A soft-deleted record is
    /// stamped too: `is_deleted` is in the pre-image so that resurrecting
    /// a deleted record in SQL is caught.
    ///
    /// # Errors
    /// When the record cannot be read back or the stamp fails.
    async fn stamp_digests(conn: &mut PgConnection, id: Uuid) -> Result<()> {
        let stored = Self::read_thing(conn, id, true).await?.ok_or_else(|| {
            crate::Error::Database("thing not found while stamping digests".into())
        })?;
        let digests = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                id: stored.id,
                thing: &stored,
                is_deleted: stored.is_deleted,
            },
        );
        let done = sqlx::query!(
            "UPDATE things SET content_hash = $2, content_hash_sha3 = $3, content_mac = $4
             WHERE id = $1",
            id,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .execute(&mut *conn)
        .await?;
        if done.rows_affected() == 0 {
            return Err(crate::Error::Database(
                "thing not stamped: no such row".into(),
            ));
        }
        Ok(())
    }

    /// Load child collections and assemble a domain [`Thing`], on `conn`.
    async fn hydrate(conn: &mut PgConnection, row: ThingRow) -> Result<Thing> {
        let id = row.id;

        let alternate_names = sqlx::query_scalar!(
            "SELECT name FROM thing_alternate_names WHERE thing_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?;

        let identifiers = sqlx::query!(
            "SELECT property_id, custom_label, value, name, url
             FROM thing_identifiers WHERE thing_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| ThingIdentifier {
            property_id: parse_ident_type(&r.property_id, r.custom_label),
            value: r.value,
            name: r.name,
            url: r.url,
        })
        .collect();

        let images = sqlx::query_scalar!(
            "SELECT url FROM thing_images WHERE thing_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?;

        let same_as = sqlx::query_scalar!(
            "SELECT url FROM thing_same_as WHERE thing_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(Thing {
            id: row.id,
            name: row.name,
            alternate_names,
            description: row.description,
            disambiguating_description: row.disambiguating_description,
            additional_type: row.additional_type,
            url: row.url,
            identifiers,
            images,
            main_entity_of_page: row.main_entity_of_page,
            owner: row.owner,
            same_as,
            subject_of: row.subject_of,
            potential_action: row.potential_action,
            is_deleted: row.is_deleted,
            deleted_at: row.deleted_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait::async_trait]
impl ThingRepository for PgThingRepository {
    /// Insert the scalar row plus all child-collection rows in one
    /// transaction, then re-hydrate so the caller gets the stored form
    /// (including any DB-side defaults). Rolls back on any failure.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] on any DB failure, or if the row
    /// cannot be read back after insert.
    async fn create(&self, thing: &Thing) -> Result<Thing> {
        // `created_at`/`updated_at` are server-managed (see `Thing`'s
        // field docs): stamp both to "now" here, overriding whatever the
        // domain value carries, rather than trusting a client-supplied —
        // or `#[serde(default)]`-defaulted, i.e. Unix-epoch — timestamp.
        let now = Utc::now();
        let mut row = thing_row(thing);
        row.created_at = now;
        row.updated_at = now;

        let mut txn = self.db.begin().await?;
        insert_thing_row(&mut txn, &row).await?;
        insert_collections(&mut txn, thing).await?;
        // Digests over the record as stored, in this transaction (INT-V1).
        Self::stamp_digests(&mut txn, thing.id).await?;
        // Durable event bus (Phase 2): under the outbox transport, write
        // the `event_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the handler's in-memory publish.
        self.enqueue_outbox(&mut txn, thing, EventKind::Created)
            .await?;
        txn.commit().await?;
        self.get_by_id(&thing.id)
            .await?
            .ok_or_else(|| crate::Error::Database("thing not found after insert".into()))
    }

    /// Fetch by primary key. Soft-deleted rows are treated as absent
    /// (returns `None`), so callers never see logically-deleted things.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] on any DB failure.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Thing>> {
        let mut conn = self.db.acquire().await?;
        Self::read_thing(&mut conn, *id, false).await
    }

    /// Replace an existing thing. Child collections are deleted and
    /// re-inserted wholesale (simpler and safer than a per-row diff), all
    /// inside one transaction. Returns the re-hydrated stored form.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::NotFound`] if no row has this id, or
    /// [`crate::Error::Database`] on any DB failure.
    async fn update(&self, thing: &Thing) -> Result<Thing> {
        if self.find_row(thing.id).await?.is_none() {
            return Err(crate::Error::NotFound);
        }
        // `created_at` is preserved (not overwritten by whatever the
        // domain value carries — a stale read or a defaulted-empty one);
        // `updated_at` is always refreshed to "now". Both are
        // server-managed, matching `create`.
        let mut row = thing_row(thing);
        row.updated_at = Utc::now();

        let mut txn = self.db.begin().await?;
        update_thing_row(&mut txn, &row, None).await?;
        delete_collections(&mut txn, thing.id).await?;
        insert_collections(&mut txn, thing).await?;
        Self::stamp_digests(&mut txn, thing.id).await?;
        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&mut txn, thing, EventKind::Updated)
            .await?;
        txn.commit().await?;
        self.get_by_id(&thing.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    /// Soft-delete: flip `is_deleted` and stamp `deleted_at`/`updated_at`
    /// rather than removing the row, preserving the audit trail. The row
    /// stays in the table but is filtered out of reads and lists.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::NotFound`] if no row has this id, or
    /// [`crate::Error::Database`] on any DB failure.
    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        let row = self.find_row(*id).await?.ok_or(crate::Error::NotFound)?;
        // One transaction, so the tombstone, its re-stamped digests
        // (INT-V1) and, under the outbox transport, the `deleted` outbox
        // row commit together: a deleted row is never left carrying the
        // live record's digests.
        let mut txn = self.db.begin().await?;
        if self.transport.is_outbox() {
            let thing = Self::hydrate(&mut txn, row).await?;
            mark_deleted(&mut txn, *id).await?;
            Self::stamp_digests(&mut txn, *id).await?;
            self.enqueue_outbox(&mut txn, &thing, EventKind::Deleted)
                .await?;
        } else {
            mark_deleted(&mut txn, *id).await?;
            Self::stamp_digests(&mut txn, *id).await?;
        }
        txn.commit().await?;
        Ok(())
    }

    /// List non-deleted things newest-first, paged by `limit`/`offset`.
    /// Each row is hydrated with its child collections (a per-row query),
    /// so large pages are intentionally avoided by the caller.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] on any DB failure.
    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Thing>> {
        let rows = sqlx::query_as!(
            ThingRow,
            "SELECT id, name, description, disambiguating_description, additional_type,
                    url, main_entity_of_page, owner, subject_of, potential_action,
                    is_deleted, content_hash, content_hash_sha3, content_mac,
                    deleted_at, created_at, updated_at
             FROM things
             WHERE is_deleted = false
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2",
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(&self.db)
        .await?;
        let mut conn = self.db.acquire().await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(Self::hydrate(&mut conn, row).await?);
        }
        Ok(out)
    }

    /// Persist a merge-history row (a snapshot of the transferred data),
    /// giving every merge a durable audit record. Returns the input clone
    /// unchanged since the row is fully caller-supplied.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] on any DB failure.
    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord> {
        sqlx::query!(
            "INSERT INTO thing_merge_records
                 (id, main_thing_id, duplicate_thing_id, merge_reason, transferred_data, merged_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            rec.id,
            rec.main_thing_id,
            rec.duplicate_thing_id,
            rec.merge_reason,
            rec.transferred_data,
            rec.merged_at,
        )
        .execute(&self.db)
        .await?;
        Ok(rec.clone())
    }

    /// Apply the survivor's row + child updates and soft-delete the
    /// duplicate **in one transaction**, so the whole merge commits (or
    /// rolls back) atomically. Under the outbox transport it enqueues a
    /// `Merged` outbox row for the survivor (carrying the duplicate's pid
    /// via `merged_from`, so a merge-repointing consumer can move edges off
    /// the duplicate) and a `Deleted` outbox row for the duplicate — both
    /// inside this transaction. A no-op under the memory transport.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] on any DB failure, or
    /// [`crate::Error::NotFound`] if the survivor cannot be re-read after
    /// the merge.
    async fn merge(&self, survivor: &Thing, duplicate_id: &Uuid) -> Result<Thing> {
        // Load the duplicate before the transaction so its `Deleted`
        // envelope carries the record's name (needed under outbox).
        let duplicate = self.get_by_id(duplicate_id).await?;

        let mut txn = self.db.begin().await?;
        // SEC-B5: the merge was computed from unlocked reads, so lock both
        // rows before writing and check under the lock that both are still
        // active. Otherwise two concurrent merges of one duplicate both
        // apply, and "B into A" racing "A into B" deletes both. The loser
        // gets `Conflict` (409) and writes nothing.
        lock_merge_participants(&mut txn, survivor.id, *duplicate_id).await?;

        // Apply the survivor's parent row + wholesale child replacement.
        // Every column is written from the survivor, `created_at` and
        // `updated_at` included, exactly as the merge always has.
        let row = thing_row(survivor);
        update_thing_row(&mut txn, &row, Some(row.created_at)).await?;
        delete_collections(&mut txn, survivor.id).await?;
        insert_collections(&mut txn, survivor).await?;
        // The survivor's digests over the survivor as stored (INT-V1).
        Self::stamp_digests(&mut txn, survivor.id).await?;

        // Soft-delete the duplicate on the same transaction, and re-stamp
        // it so its digests cover the tombstone.
        mark_deleted(&mut txn, *duplicate_id).await?;
        Self::stamp_digests(&mut txn, *duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor and a `Deleted` outbox row for the duplicate — both
        // inside this transaction. A no-op under the memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&mut txn)
                .await?;
            if let Some(dup) = duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&mut txn)
                    .await?;
            }
        }

        txn.commit().await?;

        self.get_by_id(&survivor.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }
}

/// Count non-deleted things (diagnostics / benches).
///
/// # Errors
///
/// Returns an error if the count query fails.
pub async fn count_active(db: &PgPool) -> Result<u64> {
    let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM things WHERE is_deleted = false"#)
        .fetch_one(db)
        .await?;
    Ok(u64::try_from(n).unwrap_or_default())
}

/// The rows `GET /api/records/verify` recomputes, most recently updated
/// first, soft-deleted ones included (the handler skips those).
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn rows_for_verification(db: &PgPool, limit: u64) -> Result<Vec<ThingRow>> {
    let rows = sqlx::query_as!(
        ThingRow,
        "SELECT id, name, description, disambiguating_description, additional_type,
                url, main_entity_of_page, owner, subject_of, potential_action,
                is_deleted, content_hash, content_hash_sha3, content_mac,
                deleted_at, created_at, updated_at
         FROM things
         ORDER BY updated_at DESC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Build the full `things` row from a domain [`Thing`], without digests.
fn thing_row(thing: &Thing) -> ThingRow {
    // No digests here. They cover the record as stored, which only exists
    // once the row and its children are written, so every write path
    // stamps them afterwards in the same transaction (`stamp_digests`,
    // INT-V1).
    ThingRow {
        id: thing.id,
        content_hash: None,
        content_hash_sha3: None,
        content_mac: None,
        name: thing.name.clone(),
        description: thing.description.clone(),
        disambiguating_description: thing.disambiguating_description.clone(),
        additional_type: thing.additional_type.clone(),
        url: thing.url.clone(),
        main_entity_of_page: thing.main_entity_of_page.clone(),
        owner: thing.owner.clone(),
        subject_of: thing.subject_of.clone(),
        potential_action: thing.potential_action.clone(),
        is_deleted: thing.is_deleted,
        deleted_at: thing.deleted_at,
        created_at: thing.created_at,
        updated_at: thing.updated_at,
    }
}

/// `INSERT` one whole `things` row.
async fn insert_thing_row(conn: &mut PgConnection, row: &ThingRow) -> Result<()> {
    sqlx::query!(
        "INSERT INTO things (
             id, name, description, disambiguating_description, additional_type,
             url, main_entity_of_page, owner, subject_of, potential_action,
             is_deleted, content_hash, content_hash_sha3, content_mac,
             deleted_at, created_at, updated_at
         ) VALUES (
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
         )",
        row.id,
        row.name,
        row.description,
        row.disambiguating_description,
        row.additional_type,
        row.url,
        row.main_entity_of_page,
        row.owner,
        row.subject_of,
        row.potential_action,
        row.is_deleted,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.deleted_at,
        row.created_at,
        row.updated_at,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// `UPDATE` every column of one `things` row except `id`. `created_at` is
/// written only when `created_at` is `Some` (the merge, which writes the
/// survivor whole); `None` keeps the stored value (an ordinary update).
async fn update_thing_row(
    conn: &mut PgConnection,
    row: &ThingRow,
    created_at: Option<DateTime<Utc>>,
) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE things SET
             name = $2, description = $3, disambiguating_description = $4,
             additional_type = $5, url = $6, main_entity_of_page = $7, owner = $8,
             subject_of = $9, potential_action = $10, is_deleted = $11,
             content_hash = $12, content_hash_sha3 = $13, content_mac = $14,
             deleted_at = $15, updated_at = $16,
             created_at = COALESCE($17, created_at)
         WHERE id = $1",
        row.id,
        row.name,
        row.description,
        row.disambiguating_description,
        row.additional_type,
        row.url,
        row.main_entity_of_page,
        row.owner,
        row.subject_of,
        row.potential_action,
        row.is_deleted,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.deleted_at,
        row.updated_at,
        created_at,
    )
    .execute(conn)
    .await?;
    // A row that vanished is an error, as it was when the ORM reported
    // "no record updated".
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "thing not updated: no such row".into(),
        ));
    }
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
        sqlx::query_scalar!("SELECT id FROM things WHERE id = $1 FOR UPDATE", id)
            .fetch_optional(&mut *conn)
            .await?;
    }
    let active = sqlx::query_scalar!(
        r#"SELECT count(*) AS "active!" FROM things WHERE id = ANY($1) AND is_deleted = false"#,
        &ids[..],
    )
    .fetch_one(&mut *conn)
    .await?;
    if active == 2 {
        Ok(())
    } else {
        Err(crate::Error::Conflict(format!(
            "thing {survivor_id} or {duplicate_id} is no longer active (merged or deleted meanwhile)"
        )))
    }
}

/// Soft-delete one thing: set the tombstone and its timestamp, leaving
/// every other column (the stored digests included) and the child rows in
/// place. Takes the caller's connection so [`ThingRepository::merge`] can
/// run it inside its own transaction.
async fn mark_deleted(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    let now = Utc::now();
    let done = sqlx::query!(
        "UPDATE things SET is_deleted = true, deleted_at = $2, updated_at = $2 WHERE id = $1",
        id,
        now,
    )
    .execute(conn)
    .await?;
    // As under the ORM, an update that touches no row is an error.
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "thing not deleted: no such row".into(),
        ));
    }
    Ok(())
}

/// Insert all child-collection rows for `thing` on connection `conn`.
/// Each row's `position` is its index, so `hydrate` reads the lists back
/// in the order they were written.
async fn insert_collections(conn: &mut PgConnection, thing: &Thing) -> Result<()> {
    for (i, name) in thing.alternate_names.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO thing_alternate_names (id, thing_id, name, position)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            thing.id,
            name,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, ident) in thing.identifiers.iter().enumerate() {
        let (property_id, custom_label) = ident_type_parts(&ident.property_id);
        sqlx::query!(
            "INSERT INTO thing_identifiers
                 (id, thing_id, property_id, custom_label, value, name, url, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            Uuid::new_v4(),
            thing.id,
            property_id,
            custom_label,
            ident.value,
            ident.name,
            ident.url,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, url) in thing.images.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO thing_images (id, thing_id, url, position) VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            thing.id,
            url,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, url) in thing.same_as.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO thing_same_as (id, thing_id, url, position) VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            thing.id,
            url,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Delete all child-collection rows for `thing_id` on connection `conn`.
async fn delete_collections(conn: &mut PgConnection, thing_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM thing_alternate_names WHERE thing_id = $1",
        thing_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM thing_identifiers WHERE thing_id = $1",
        thing_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM thing_images WHERE thing_id = $1", thing_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM thing_same_as WHERE thing_id = $1", thing_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// A list index as the `INTEGER` `position` column.
fn position(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

/// A `LIMIT` / `OFFSET` bound: Postgres takes `BIGINT`, the callers count
/// in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Map an [`IdentifierType`] to its `(property_id, custom_label)` columns.
fn ident_type_parts(t: &IdentifierType) -> (String, Option<String>) {
    match t {
        IdentifierType::Doi => ("doi".into(), None),
        IdentifierType::Isbn => ("isbn".into(), None),
        IdentifierType::Issn => ("issn".into(), None),
        IdentifierType::Gtin => ("gtin".into(), None),
        IdentifierType::Sku => ("sku".into(), None),
        IdentifierType::Mpn => ("mpn".into(), None),
        IdentifierType::SerialNumber => ("serial_number".into(), None),
        IdentifierType::Uri => ("uri".into(), None),
        IdentifierType::Uuid => ("uuid".into(), None),
        IdentifierType::Custom(label) => ("custom".into(), Some(label.clone())),
    }
}

/// Parse a stored `(property_id, custom_label)` back into an [`IdentifierType`].
fn parse_ident_type(property_id: &str, custom_label: Option<String>) -> IdentifierType {
    match property_id {
        "doi" => IdentifierType::Doi,
        "isbn" => IdentifierType::Isbn,
        "issn" => IdentifierType::Issn,
        "gtin" => IdentifierType::Gtin,
        "sku" => IdentifierType::Sku,
        "mpn" => IdentifierType::Mpn,
        "serial_number" => IdentifierType::SerialNumber,
        "uri" => IdentifierType::Uri,
        "uuid" => IdentifierType::Uuid,
        _ => IdentifierType::Custom(custom_label.unwrap_or_else(|| property_id.to_string())),
    }
}

pub use audit::AuditLogRepository;

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod outbox_tests {
    use super::{PgThingRepository, ThingRepository};
    use crate::db::outbox;
    use crate::models::thing::Thing;
    use crate::streaming::EventTransport;

    async fn connect() -> sqlx::PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sqlx::PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Create, under the outbox transport, writes one `created` outbox row
    /// in the same transaction as the entity rows.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = PgThingRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let thing = repo.create(&Thing::new("Widget")).await.unwrap();

        let rows = outbox::rows_for(&db, thing.id, "created").await.unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new thing");
        assert_eq!(rows[0].entity, "thing");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = PgThingRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&Thing::new("Survivor")).await.unwrap();
        let duplicate = repo.create(&Thing::new("Duplicate")).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = outbox::rows_for(&db, survivor.id, "merged").await.unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = outbox::rows_for(&db, duplicate.id, "deleted")
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
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn a_merge_waits_for_a_concurrent_delete_and_then_refuses() {
        let db = connect().await;
        let repo = PgThingRepository::new(db.clone());
        let survivor = repo.create(&Thing::new("Race Survivor")).await.unwrap();
        let duplicate = repo.create(&Thing::new("Race Duplicate")).await.unwrap();

        let mut other = db.begin().await.unwrap();
        super::mark_deleted(&mut other, duplicate.id).await.unwrap();

        let mut renamed = survivor.clone();
        renamed.name = "Race Survivor, merged".into();
        let merger = PgThingRepository::new(db.clone());
        let dup_id = duplicate.id;
        let merge = tokio::spawn(async move { merger.merge(&renamed, &dup_id).await });

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
        assert_eq!(stored.name, "Race Survivor", "the survivor was not written");
    }
}
