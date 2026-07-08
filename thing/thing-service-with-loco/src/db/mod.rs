//! Database access — connection pool + repository.
//!
//! [`SeaOrmThingRepository`] round-trips a domain [`Thing`] against a fully
//! normalized relational schema: scalar fields live on the `things` row;
//! the `alternate_names`, `identifiers`, `images`, and `same_as`
//! collections live in dedicated child tables joined by `thing_id`. Writes
//! are transactional (parent + children); updates replace the child rows.

pub mod audit;
pub mod convert;
pub mod models;
pub mod outbox;

use convert::{offset_to_ts, ts_to_offset};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::db::outbox::OutboxInsert;
use crate::models::identifier::{IdentifierType, ThingIdentifier};
use crate::models::merge::MergeRecord;
use crate::models::thing::Thing;
use crate::streaming::envelope::{EventKind, EventTransport};
use models::{
    thing_alternate_names, thing_identifiers, thing_images, thing_merge_records, thing_same_as,
    things,
};

/// Open a connection pool from a [`DatabaseConfig`].
///
/// # Errors
///
/// Returns an error if the database connection cannot be established.
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);
    Database::connect(opt)
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

/// SeaORM-backed [`ThingRepository`] over a `PostgreSQL` connection pool.
pub struct SeaOrmThingRepository {
    /// The shared `SeaORM` connection pool.
    db: DatabaseConnection,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the handler's in-memory
    /// publish untouched; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl SeaOrmThingRepository {
    /// Wrap an existing connection pool in a repository, defaulting to the
    /// [`EventTransport::Memory`] transport (behaviour unchanged from
    /// before the outbox landed).
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
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
    /// `&DatabaseTransaction` so the row commits with the entity write
    /// (the outbox atomicity guarantee). A no-op under
    /// [`EventTransport::Memory`].
    ///
    /// The `ConnectionTrait` generic lives here on a concrete method (not
    /// on the object-safe [`ThingRepository`] trait), which is how a
    /// `dyn`-trait repository threads the outbox insert into its own
    /// transaction.
    async fn enqueue_outbox<C: ConnectionTrait>(
        &self,
        conn: &C,
        thing: &Thing,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(thing, kind)?.insert_on(conn).await?;
        }
        Ok(())
    }

    /// Load child collections and assemble a domain [`Thing`].
    async fn hydrate(&self, row: things::Model) -> Result<Thing> {
        let id = row.id;

        let alt_rows = thing_alternate_names::Entity::find()
            .filter(thing_alternate_names::Column::ThingId.eq(id))
            .order_by_asc(thing_alternate_names::Column::Position)
            .all(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let alternate_names = alt_rows.into_iter().map(|r| r.name).collect();

        let id_rows = thing_identifiers::Entity::find()
            .filter(thing_identifiers::Column::ThingId.eq(id))
            .order_by_asc(thing_identifiers::Column::Position)
            .all(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let identifiers = id_rows
            .into_iter()
            .map(|r| ThingIdentifier {
                property_id: parse_ident_type(&r.property_id, r.custom_label),
                value: r.value,
                name: r.name,
                url: r.url,
            })
            .collect();

        let img_rows = thing_images::Entity::find()
            .filter(thing_images::Column::ThingId.eq(id))
            .order_by_asc(thing_images::Column::Position)
            .all(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let images = img_rows.into_iter().map(|r| r.url).collect();

        let same_rows = thing_same_as::Entity::find()
            .filter(thing_same_as::Column::ThingId.eq(id))
            .order_by_asc(thing_same_as::Column::Position)
            .all(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let same_as = same_rows.into_iter().map(|r| r.url).collect();

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
            deleted_at: row.deleted_at.map(offset_to_ts),
            created_at: offset_to_ts(row.created_at),
            updated_at: offset_to_ts(row.updated_at),
        })
    }
}

#[async_trait::async_trait]
impl ThingRepository for SeaOrmThingRepository {
    /// Insert the scalar row plus all child-collection rows in one
    /// transaction, then re-hydrate so the caller gets the stored form
    /// (including any DB-side defaults). Rolls back on any failure.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] on any DB failure, or if the row
    /// cannot be read back after insert.
    async fn create(&self, thing: &Thing) -> Result<Thing> {
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
        to_active(thing)
            .insert(&txn)
            .await
            .map_err(|e| map_db(&e))?;
        insert_collections(&txn, thing).await?;
        // Durable event bus (Phase 2): under the outbox transport, write
        // the `event_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the handler's in-memory publish.
        self.enqueue_outbox(&txn, thing, EventKind::Created).await?;
        txn.commit().await.map_err(|e| map_db(&e))?;
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
        let row = things::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let Some(row) = row else { return Ok(None) };
        if row.is_deleted {
            return Ok(None);
        }
        Ok(Some(self.hydrate(row).await?))
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
        let exists = things::Entity::find_by_id(thing.id)
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        if exists.is_none() {
            return Err(crate::Error::NotFound);
        }
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
        to_active(thing)
            .update(&txn)
            .await
            .map_err(|e| map_db(&e))?;
        delete_collections(&txn, thing.id).await?;
        insert_collections(&txn, thing).await?;
        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&txn, thing, EventKind::Updated).await?;
        txn.commit().await.map_err(|e| map_db(&e))?;
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
        let row = things::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?
            .ok_or(crate::Error::NotFound)?;
        let now = OffsetDateTime::now_utc();
        let mut active: things::ActiveModel = row.clone().into();
        active.is_deleted = Set(true);
        active.deleted_at = Set(Some(now));
        active.updated_at = Set(now);
        // Unlike create/update, the soft-delete had no existing
        // transaction. Under the outbox transport we open one here so the
        // tombstone write and the `deleted` outbox row commit atomically;
        // the memory transport keeps the plain, tx-free update.
        if self.transport.is_outbox() {
            let thing = self.hydrate(row).await?;
            let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
            active.update(&txn).await.map_err(|e| map_db(&e))?;
            self.enqueue_outbox(&txn, &thing, EventKind::Deleted).await?;
            txn.commit().await.map_err(|e| map_db(&e))?;
        } else {
            active.update(&self.db).await.map_err(|e| map_db(&e))?;
        }
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
        let rows = things::Entity::find()
            .filter(things::Column::IsDeleted.eq(false))
            .order_by_desc(things::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.hydrate(row).await?);
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
        let active = thing_merge_records::ActiveModel {
            id: Set(rec.id),
            main_thing_id: Set(rec.main_thing_id),
            duplicate_thing_id: Set(rec.duplicate_thing_id),
            merge_reason: Set(rec.merge_reason.clone()),
            transferred_data: Set(rec.transferred_data.clone()),
            merged_at: Set(ts_to_offset(rec.merged_at)),
        };
        active.insert(&self.db).await.map_err(|e| map_db(&e))?;
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

        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;

        // Apply the survivor's parent row + wholesale child replacement.
        to_active(survivor)
            .update(&txn)
            .await
            .map_err(|e| map_db(&e))?;
        delete_collections(&txn, survivor.id).await?;
        insert_collections(&txn, survivor).await?;

        // Soft-delete the duplicate on the same transaction.
        apply_soft_delete_row(&txn, duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor and a `Deleted` outbox row for the duplicate — both
        // inside this transaction. A no-op under the memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&txn)
                .await?;
            if let Some(dup) = duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&txn)
                    .await?;
            }
        }

        txn.commit().await.map_err(|e| map_db(&e))?;

        self.get_by_id(&survivor.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }
}

/// Soft-delete the thing `id` on `conn` by stamping `is_deleted`/
/// `deleted_at`/`updated_at` and leaving every other column (and the
/// child rows) in place. A partial `UPDATE` (all other columns stay
/// `NotSet`) so [`ThingRepository::merge`] can reuse it on its own
/// transaction without loading the row first.
async fn apply_soft_delete_row<C: ConnectionTrait>(conn: &C, id: &Uuid) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let active = things::ActiveModel {
        id: Set(*id),
        is_deleted: Set(true),
        deleted_at: Set(Some(now)),
        updated_at: Set(now),
        ..Default::default()
    };
    active.update(conn).await.map_err(|e| map_db(&e))?;
    Ok(())
}

/// Count non-deleted things (diagnostics / benches).
///
/// # Errors
///
/// Returns an error if the count query fails.
pub async fn count_active(db: &DatabaseConnection) -> Result<u64> {
    things::Entity::find()
        .filter(things::Column::IsDeleted.eq(false))
        .count(db)
        .await
        .map_err(|e| map_db(&e))
}

/// Build the `things` scalar active model from a domain [`Thing`].
fn to_active(thing: &Thing) -> things::ActiveModel {
    things::ActiveModel {
        id: Set(thing.id),
        name: Set(thing.name.clone()),
        description: Set(thing.description.clone()),
        disambiguating_description: Set(thing.disambiguating_description.clone()),
        additional_type: Set(thing.additional_type.clone()),
        url: Set(thing.url.clone()),
        main_entity_of_page: Set(thing.main_entity_of_page.clone()),
        owner: Set(thing.owner.clone()),
        subject_of: Set(thing.subject_of.clone()),
        potential_action: Set(thing.potential_action.clone()),
        is_deleted: Set(thing.is_deleted),
        deleted_at: Set(thing.deleted_at.map(ts_to_offset)),
        created_at: Set(ts_to_offset(thing.created_at)),
        updated_at: Set(ts_to_offset(thing.updated_at)),
    }
}

/// Insert all child-collection rows for `thing` on connection `conn`.
async fn insert_collections<C: ConnectionTrait>(conn: &C, thing: &Thing) -> Result<()> {
    for (i, name) in thing.alternate_names.iter().enumerate() {
        thing_alternate_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            thing_id: Set(thing.id),
            name: Set(name.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await
        .map_err(|e| map_db(&e))?;
    }
    for (i, ident) in thing.identifiers.iter().enumerate() {
        let (property_id, custom_label) = ident_type_parts(&ident.property_id);
        thing_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            thing_id: Set(thing.id),
            property_id: Set(property_id),
            custom_label: Set(custom_label),
            value: Set(ident.value.clone()),
            name: Set(ident.name.clone()),
            url: Set(ident.url.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await
        .map_err(|e| map_db(&e))?;
    }
    for (i, url) in thing.images.iter().enumerate() {
        thing_images::ActiveModel {
            id: Set(Uuid::new_v4()),
            thing_id: Set(thing.id),
            url: Set(url.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await
        .map_err(|e| map_db(&e))?;
    }
    for (i, url) in thing.same_as.iter().enumerate() {
        thing_same_as::ActiveModel {
            id: Set(Uuid::new_v4()),
            thing_id: Set(thing.id),
            url: Set(url.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await
        .map_err(|e| map_db(&e))?;
    }
    Ok(())
}

/// Delete all child-collection rows for `thing_id` on connection `conn`.
async fn delete_collections<C: ConnectionTrait>(conn: &C, thing_id: Uuid) -> Result<()> {
    thing_alternate_names::Entity::delete_many()
        .filter(thing_alternate_names::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    thing_identifiers::Entity::delete_many()
        .filter(thing_identifiers::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    thing_images::Entity::delete_many()
        .filter(thing_images::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    thing_same_as::Entity::delete_many()
        .filter(thing_same_as::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    Ok(())
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

/// Map a `SeaORM` `DbErr` into the crate error type.
fn map_db(e: &sea_orm::DbErr) -> crate::Error {
    crate::Error::Database(e.to_string())
}

pub use audit::AuditLogRepository;

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod outbox_tests {
    use super::{SeaOrmThingRepository, ThingRepository};
    use crate::db::models::event_outbox;
    use crate::models::thing::Thing;
    use crate::streaming::EventTransport;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Create, under the outbox transport, writes one `created` outbox row
    /// in the same transaction as the entity rows.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = SeaOrmThingRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let thing = repo.create(&Thing::new("Widget")).await.unwrap();

        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(thing.id))
            .filter(event_outbox::Column::Kind.eq("created"))
            .all(&db)
            .await
            .unwrap();
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
        let repo = SeaOrmThingRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&Thing::new("Survivor")).await.unwrap();
        let duplicate = repo.create(&Thing::new("Duplicate")).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(survivor.id))
            .filter(event_outbox::Column::Kind.eq("merged"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(duplicate.id))
            .filter(event_outbox::Column::Kind.eq("deleted"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(deleted_rows.len(), 1, "one deleted outbox row for duplicate");
    }
}
