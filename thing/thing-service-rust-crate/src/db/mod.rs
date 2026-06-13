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

use convert::{offset_to_ts, ts_to_offset};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::models::identifier::{IdentifierType, ThingIdentifier};
use crate::models::merge::MergeRecord;
use crate::models::thing::Thing;
use models::{
    thing_alternate_names, thing_identifiers, thing_images, thing_merge_records, thing_same_as,
    things,
};

/// Open a connection pool from a [`DatabaseConfig`].
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
}

/// SeaORM-backed [`ThingRepository`] over a PostgreSQL connection pool.
pub struct SeaOrmThingRepository {
    /// The shared SeaORM connection pool.
    db: DatabaseConnection,
}

impl SeaOrmThingRepository {
    /// Wrap an existing connection pool in a repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Load child collections and assemble a domain [`Thing`].
    async fn hydrate(&self, row: things::Model) -> Result<Thing> {
        let id = row.id;

        let alt_rows = thing_alternate_names::Entity::find()
            .filter(thing_alternate_names::Column::ThingId.eq(id))
            .order_by_asc(thing_alternate_names::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let alternate_names = alt_rows.into_iter().map(|r| r.name).collect();

        let id_rows = thing_identifiers::Entity::find()
            .filter(thing_identifiers::Column::ThingId.eq(id))
            .order_by_asc(thing_identifiers::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
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
            .map_err(map_db)?;
        let images = img_rows.into_iter().map(|r| r.url).collect();

        let same_rows = thing_same_as::Entity::find()
            .filter(thing_same_as::Column::ThingId.eq(id))
            .order_by_asc(thing_same_as::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
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
    async fn create(&self, thing: &Thing) -> Result<Thing> {
        let txn = self.db.begin().await.map_err(map_db)?;
        to_active(thing).insert(&txn).await.map_err(map_db)?;
        insert_collections(&txn, thing).await?;
        txn.commit().await.map_err(map_db)?;
        self.get_by_id(&thing.id)
            .await?
            .ok_or_else(|| crate::Error::Database("thing not found after insert".into()))
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Thing>> {
        let row = things::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(map_db)?;
        let Some(row) = row else { return Ok(None) };
        if row.is_deleted {
            return Ok(None);
        }
        Ok(Some(self.hydrate(row).await?))
    }

    async fn update(&self, thing: &Thing) -> Result<Thing> {
        let exists = things::Entity::find_by_id(thing.id)
            .one(&self.db)
            .await
            .map_err(map_db)?;
        if exists.is_none() {
            return Err(crate::Error::NotFound);
        }
        let txn = self.db.begin().await.map_err(map_db)?;
        to_active(thing).update(&txn).await.map_err(map_db)?;
        delete_collections(&txn, thing.id).await?;
        insert_collections(&txn, thing).await?;
        txn.commit().await.map_err(map_db)?;
        self.get_by_id(&thing.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        let row = things::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(map_db)?
            .ok_or(crate::Error::NotFound)?;
        let mut active: things::ActiveModel = row.into();
        active.is_deleted = Set(true);
        active.deleted_at = Set(Some(OffsetDateTime::now_utc()));
        active.updated_at = Set(OffsetDateTime::now_utc());
        active.update(&self.db).await.map_err(map_db)?;
        Ok(())
    }

    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Thing>> {
        let rows = things::Entity::find()
            .filter(things::Column::IsDeleted.eq(false))
            .order_by_desc(things::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.hydrate(row).await?);
        }
        Ok(out)
    }

    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord> {
        let active = thing_merge_records::ActiveModel {
            id: Set(rec.id),
            main_thing_id: Set(rec.main_thing_id),
            duplicate_thing_id: Set(rec.duplicate_thing_id),
            merge_reason: Set(rec.merge_reason.clone()),
            transferred_data: Set(rec.transferred_data.clone()),
            merged_at: Set(ts_to_offset(rec.merged_at)),
        };
        active.insert(&self.db).await.map_err(map_db)?;
        Ok(rec.clone())
    }
}

/// Count non-deleted things (diagnostics / benches).
pub async fn count_active(db: &DatabaseConnection) -> Result<u64> {
    things::Entity::find()
        .filter(things::Column::IsDeleted.eq(false))
        .count(db)
        .await
        .map_err(map_db)
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
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
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
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    for (i, url) in thing.images.iter().enumerate() {
        thing_images::ActiveModel {
            id: Set(Uuid::new_v4()),
            thing_id: Set(thing.id),
            url: Set(url.clone()),
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    for (i, url) in thing.same_as.iter().enumerate() {
        thing_same_as::ActiveModel {
            id: Set(Uuid::new_v4()),
            thing_id: Set(thing.id),
            url: Set(url.clone()),
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    Ok(())
}

/// Delete all child-collection rows for `thing_id` on connection `conn`.
async fn delete_collections<C: ConnectionTrait>(conn: &C, thing_id: Uuid) -> Result<()> {
    thing_alternate_names::Entity::delete_many()
        .filter(thing_alternate_names::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    thing_identifiers::Entity::delete_many()
        .filter(thing_identifiers::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    thing_images::Entity::delete_many()
        .filter(thing_images::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    thing_same_as::Entity::delete_many()
        .filter(thing_same_as::Column::ThingId.eq(thing_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
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

/// Map a SeaORM `DbErr` into the crate error type.
fn map_db(e: sea_orm::DbErr) -> crate::Error {
    crate::Error::Database(e.to_string())
}

pub use audit::AuditLogRepository;
