//! Database access — connection pool + repository.
//!
//! [`SeaOrmPlaceRepository`] round-trips a domain [`Place`] against a fully
//! normalized relational schema: scalar fields plus the flattened 0..1
//! address/geo objects live on the `places` row; the keywords, identifiers,
//! amenity_features, and opening_hours collections live in dedicated child
//! tables joined by `place_id`. Writes are transactional; updates replace
//! the child rows.

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
use crate::models::address::PostalAddress;
use crate::models::amenity::AmenityFeature;
use crate::models::geo::GeoCoordinates;
use crate::models::identifier::{IdentifierType, PlaceIdentifier};
use crate::models::merge::MergeRecord;
use crate::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};
use crate::models::place::Place;
use crate::models::place_type::PlaceType;
use models::{
    place_amenity_features, place_identifiers, place_keywords, place_merge_records,
    place_opening_hours, places,
};

/// Open a SeaORM connection pool from a [`DatabaseConfig`].
///
/// Applies the configured `max`/`min` pool bounds so the service caps its
/// concurrent database connections (PostgreSQL has a finite backend slot
/// budget) while keeping a warm minimum to avoid cold-connect latency on
/// the first request after idle.
///
/// # Errors
/// Returns [`crate::Error::Pool`] if SeaORM cannot establish the pool
/// (bad URL, unreachable server, auth failure, …).
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    // SeaORM's connect options carry the pool sizing knobs.
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);
    Database::connect(opt)
        .await
        // Normalise the SeaORM error into the crate's pool-error variant.
        .map_err(|e| crate::Error::Pool(e.to_string()))
}

/// Storage-agnostic CRUD interface for [`Place`] records.
///
/// Abstracting persistence behind a trait lets `AppState` carry an
/// `Arc<dyn PlaceRepository>` so handlers depend on the contract, not the
/// SeaORM implementation — easing testing and any future storage swap.
/// `Send + Sync` is required because the repository is shared across Axum's
/// worker tasks.
#[async_trait::async_trait]
pub trait PlaceRepository: Send + Sync {
    /// Insert a new place; returns the stored form (re-read after insert).
    ///
    /// # Errors
    /// Propagates any database error from the insert or the read-back.
    async fn create(&self, place: &Place) -> Result<Place>;
    /// Fetch a place by id, returning `None` if absent or soft-deleted.
    ///
    /// Soft-deleted rows are treated as absent so callers never see tombstones.
    ///
    /// # Errors
    /// Propagates any database error from the lookup or child-row hydration.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Place>>;
    /// Replace an existing place (scalar row + child collections).
    ///
    /// # Errors
    /// Returns [`crate::Error::NotFound`] if no row matches, or a database
    /// error from the update transaction.
    async fn update(&self, place: &Place) -> Result<Place>;
    /// Soft-delete a place (sets `is_deleted`/`deleted_at`); the row is kept
    /// for audit/compliance rather than physically removed.
    ///
    /// # Errors
    /// Returns [`crate::Error::NotFound`] if no row matches, or a database error.
    async fn soft_delete(&self, id: &Uuid) -> Result<()>;
    /// List non-deleted places, newest first, with limit/offset paging.
    ///
    /// # Errors
    /// Propagates any database error from the query or hydration.
    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Place>>;
    /// Record a merge of `duplicate` into `main` in the merge-history table.
    ///
    /// # Errors
    /// Propagates any database error from the insert.
    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord>;
}

/// SeaORM-backed [`PlaceRepository`] over a PostgreSQL connection pool.
pub struct SeaOrmPlaceRepository {
    /// The shared SeaORM connection pool.
    db: DatabaseConnection,
}

impl SeaOrmPlaceRepository {
    /// Wrap an existing connection pool in a repository. Cheap: the pool
    /// (a `DatabaseConnection`) is itself a clonable handle.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Load child collections and assemble a domain [`Place`].
    ///
    /// Reverses the normalization done on write: the flattened address/geo
    /// columns are rebuilt into their nested objects, and each child table
    /// (keywords, identifiers, amenity features, opening hours) is queried
    /// separately and ordered by `position` so the original list order is
    /// preserved across the round-trip.
    ///
    /// # Errors
    /// Propagates any database error from the four child-table queries.
    async fn hydrate(&self, row: places::Model) -> Result<Place> {
        let id = row.id;

        // Keywords: ordered by stored position to keep list order stable.
        let kw_rows = place_keywords::Entity::find()
            .filter(place_keywords::Column::PlaceId.eq(id))
            .order_by_asc(place_keywords::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let keywords = kw_rows.into_iter().map(|r| r.keyword).collect();

        // Identifiers: the stored `(type tag, custom_label)` pair is decoded
        // back into the rich `IdentifierType` enum.
        let id_rows = place_identifiers::Entity::find()
            .filter(place_identifiers::Column::PlaceId.eq(id))
            .order_by_asc(place_identifiers::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let identifiers = id_rows
            .into_iter()
            .map(|r| PlaceIdentifier {
                identifier_type: parse_ident_type(&r.identifier_type, r.custom_label),
                value: r.value,
            })
            .collect();

        let am_rows = place_amenity_features::Entity::find()
            .filter(place_amenity_features::Column::PlaceId.eq(id))
            .order_by_asc(place_amenity_features::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let amenity_features = am_rows
            .into_iter()
            .map(|r| AmenityFeature {
                name: r.name,
                value: r.value,
            })
            .collect();

        let oh_rows = place_opening_hours::Entity::find()
            .filter(place_opening_hours::Column::PlaceId.eq(id))
            .order_by_asc(place_opening_hours::Column::Position)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let opening_hours = oh_rows
            .into_iter()
            .map(|r| OpeningHoursSpecification {
                day_of_week: parse_day_of_week(&r.day_of_week),
                opens: r.opens,
                closes: r.closes,
            })
            .collect();

        let address = build_address(&row);
        let geo = match (row.geo_latitude, row.geo_longitude) {
            (Some(latitude), Some(longitude)) => Some(GeoCoordinates {
                latitude,
                longitude,
                elevation: row.geo_elevation,
            }),
            _ => None,
        };
        let place_type = row
            .place_type
            .as_deref()
            .map(|t| parse_place_type(t, row.place_type_custom.clone()));

        Ok(Place {
            id: row.id,
            name: row.name,
            alternate_name: row.alternate_name,
            description: row.description,
            place_type,
            address,
            geo,
            telephone: row.telephone,
            fax_number: row.fax_number,
            url: row.url,
            global_location_number: row.global_location_number,
            branch_code: row.branch_code,
            contained_in_place: row.contained_in_place,
            keywords,
            identifiers,
            amenity_features,
            opening_hours,
            is_accessible_for_free: row.is_accessible_for_free,
            public_access: row.public_access,
            smoking_allowed: row.smoking_allowed,
            maximum_attendee_capacity: row.maximum_attendee_capacity.map(|v| v as u32),
            is_deleted: row.is_deleted,
            deleted_at: row.deleted_at.map(offset_to_ts),
            created_at: offset_to_ts(row.created_at),
            updated_at: offset_to_ts(row.updated_at),
        })
    }
}

#[async_trait::async_trait]
impl PlaceRepository for SeaOrmPlaceRepository {
    async fn create(&self, place: &Place) -> Result<Place> {
        let txn = self.db.begin().await.map_err(map_db)?;
        to_active(place).insert(&txn).await.map_err(map_db)?;
        insert_collections(&txn, place).await?;
        txn.commit().await.map_err(map_db)?;
        self.get_by_id(&place.id)
            .await?
            .ok_or_else(|| crate::Error::Database("place not found after insert".into()))
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Place>> {
        let row = places::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(map_db)?;
        let Some(row) = row else { return Ok(None) };
        if row.is_deleted {
            return Ok(None);
        }
        Ok(Some(self.hydrate(row).await?))
    }

    async fn update(&self, place: &Place) -> Result<Place> {
        let exists = places::Entity::find_by_id(place.id)
            .one(&self.db)
            .await
            .map_err(map_db)?;
        if exists.is_none() {
            return Err(crate::Error::NotFound);
        }
        let txn = self.db.begin().await.map_err(map_db)?;
        to_active(place).update(&txn).await.map_err(map_db)?;
        delete_collections(&txn, place.id).await?;
        insert_collections(&txn, place).await?;
        txn.commit().await.map_err(map_db)?;
        self.get_by_id(&place.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        let row = places::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(map_db)?
            .ok_or(crate::Error::NotFound)?;
        let mut active: places::ActiveModel = row.into();
        active.is_deleted = Set(true);
        active.deleted_at = Set(Some(OffsetDateTime::now_utc()));
        active.updated_at = Set(OffsetDateTime::now_utc());
        active.update(&self.db).await.map_err(map_db)?;
        Ok(())
    }

    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Place>> {
        let rows = places::Entity::find()
            .filter(places::Column::IsDeleted.eq(false))
            .order_by_desc(places::Column::CreatedAt)
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
        let active = place_merge_records::ActiveModel {
            id: Set(rec.id),
            main_place_id: Set(rec.main_place_id),
            duplicate_place_id: Set(rec.duplicate_place_id),
            merge_reason: Set(rec.merge_reason.clone()),
            transferred_data: Set(rec.transferred_data.clone()),
            merged_at: Set(ts_to_offset(rec.merged_at)),
        };
        active.insert(&self.db).await.map_err(map_db)?;
        Ok(rec.clone())
    }
}

/// Count non-deleted places (diagnostics / benches).
pub async fn count_active(db: &DatabaseConnection) -> Result<u64> {
    places::Entity::find()
        .filter(places::Column::IsDeleted.eq(false))
        .count(db)
        .await
        .map_err(map_db)
}

/// Build the `places` scalar+flattened active model from a domain [`Place`].
fn to_active(place: &Place) -> places::ActiveModel {
    let (place_type, place_type_custom) = match &place.place_type {
        Some(t) => {
            let (tag, custom) = place_type_parts(t);
            (Some(tag), custom)
        }
        None => (None, None),
    };
    places::ActiveModel {
        id: Set(place.id),
        name: Set(place.name.clone()),
        alternate_name: Set(place.alternate_name.clone()),
        description: Set(place.description.clone()),
        place_type: Set(place_type),
        place_type_custom: Set(place_type_custom),
        address_street_address: Set(place
            .address
            .as_ref()
            .and_then(|a| a.street_address.clone())),
        address_locality: Set(place
            .address
            .as_ref()
            .and_then(|a| a.address_locality.clone())),
        address_region: Set(place
            .address
            .as_ref()
            .and_then(|a| a.address_region.clone())),
        address_country: Set(place
            .address
            .as_ref()
            .and_then(|a| a.address_country.clone())),
        address_postal_code: Set(place.address.as_ref().and_then(|a| a.postal_code.clone())),
        geo_latitude: Set(place.geo.as_ref().map(|g| g.latitude)),
        geo_longitude: Set(place.geo.as_ref().map(|g| g.longitude)),
        geo_elevation: Set(place.geo.as_ref().and_then(|g| g.elevation)),
        telephone: Set(place.telephone.clone()),
        fax_number: Set(place.fax_number.clone()),
        url: Set(place.url.clone()),
        global_location_number: Set(place.global_location_number.clone()),
        branch_code: Set(place.branch_code.clone()),
        contained_in_place: Set(place.contained_in_place),
        is_accessible_for_free: Set(place.is_accessible_for_free),
        public_access: Set(place.public_access),
        smoking_allowed: Set(place.smoking_allowed),
        maximum_attendee_capacity: Set(place.maximum_attendee_capacity.map(|v| v as i32)),
        is_deleted: Set(place.is_deleted),
        deleted_at: Set(place.deleted_at.map(ts_to_offset)),
        created_at: Set(ts_to_offset(place.created_at)),
        updated_at: Set(ts_to_offset(place.updated_at)),
    }
}

/// Reconstruct a [`PostalAddress`] from the flattened row, or `None` when
/// every address column is empty.
fn build_address(row: &places::Model) -> Option<PostalAddress> {
    if row.address_street_address.is_none()
        && row.address_locality.is_none()
        && row.address_region.is_none()
        && row.address_country.is_none()
        && row.address_postal_code.is_none()
    {
        return None;
    }
    Some(PostalAddress {
        street_address: row.address_street_address.clone(),
        address_locality: row.address_locality.clone(),
        address_region: row.address_region.clone(),
        address_country: row.address_country.clone(),
        postal_code: row.address_postal_code.clone(),
    })
}

/// Insert all child-collection rows for `place` on connection `conn`.
async fn insert_collections<C: ConnectionTrait>(conn: &C, place: &Place) -> Result<()> {
    for (i, keyword) in place.keywords.iter().enumerate() {
        place_keywords::ActiveModel {
            id: Set(Uuid::new_v4()),
            place_id: Set(place.id),
            keyword: Set(keyword.clone()),
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    for (i, ident) in place.identifiers.iter().enumerate() {
        let (identifier_type, custom_label) = ident_type_parts(&ident.identifier_type);
        place_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            place_id: Set(place.id),
            identifier_type: Set(identifier_type),
            custom_label: Set(custom_label),
            value: Set(ident.value.clone()),
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    for (i, am) in place.amenity_features.iter().enumerate() {
        place_amenity_features::ActiveModel {
            id: Set(Uuid::new_v4()),
            place_id: Set(place.id),
            name: Set(am.name.clone()),
            value: Set(am.value.clone()),
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    for (i, oh) in place.opening_hours.iter().enumerate() {
        place_opening_hours::ActiveModel {
            id: Set(Uuid::new_v4()),
            place_id: Set(place.id),
            day_of_week: Set(day_of_week_str(&oh.day_of_week)),
            opens: Set(oh.opens.clone()),
            closes: Set(oh.closes.clone()),
            position: Set(i as i32),
        }
        .insert(conn)
        .await
        .map_err(map_db)?;
    }
    Ok(())
}

/// Delete all child-collection rows for `place_id` on connection `conn`.
async fn delete_collections<C: ConnectionTrait>(conn: &C, place_id: Uuid) -> Result<()> {
    place_keywords::Entity::delete_many()
        .filter(place_keywords::Column::PlaceId.eq(place_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    place_identifiers::Entity::delete_many()
        .filter(place_identifiers::Column::PlaceId.eq(place_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    place_amenity_features::Entity::delete_many()
        .filter(place_amenity_features::Column::PlaceId.eq(place_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    place_opening_hours::Entity::delete_many()
        .filter(place_opening_hours::Column::PlaceId.eq(place_id))
        .exec(conn)
        .await
        .map_err(map_db)?;
    Ok(())
}

/// Map a [`PlaceType`] to its `(tag, custom_label)` columns.
fn place_type_parts(t: &PlaceType) -> (String, Option<String>) {
    match t {
        PlaceType::LocalBusiness => ("local_business".into(), None),
        PlaceType::CivicStructure => ("civic_structure".into(), None),
        PlaceType::AdministrativeArea => ("administrative_area".into(), None),
        PlaceType::Landform => ("landform".into(), None),
        PlaceType::Park => ("park".into(), None),
        PlaceType::Airport => ("airport".into(), None),
        PlaceType::Hospital => ("hospital".into(), None),
        PlaceType::School => ("school".into(), None),
        PlaceType::Library => ("library".into(), None),
        PlaceType::Museum => ("museum".into(), None),
        PlaceType::Restaurant => ("restaurant".into(), None),
        PlaceType::Hotel => ("hotel".into(), None),
        PlaceType::Other(label) => ("other".into(), Some(label.clone())),
    }
}

/// Parse a stored `(tag, custom_label)` back into a [`PlaceType`].
fn parse_place_type(tag: &str, custom_label: Option<String>) -> PlaceType {
    match tag {
        "local_business" => PlaceType::LocalBusiness,
        "civic_structure" => PlaceType::CivicStructure,
        "administrative_area" => PlaceType::AdministrativeArea,
        "landform" => PlaceType::Landform,
        "park" => PlaceType::Park,
        "airport" => PlaceType::Airport,
        "hospital" => PlaceType::Hospital,
        "school" => PlaceType::School,
        "library" => PlaceType::Library,
        "museum" => PlaceType::Museum,
        "restaurant" => PlaceType::Restaurant,
        "hotel" => PlaceType::Hotel,
        _ => PlaceType::Other(custom_label.unwrap_or_else(|| tag.to_string())),
    }
}

/// Map an [`IdentifierType`] to its `(tag, custom_label)` columns.
fn ident_type_parts(t: &IdentifierType) -> (String, Option<String>) {
    match t {
        IdentifierType::GlobalLocationNumber => ("global_location_number".into(), None),
        IdentifierType::BranchCode => ("branch_code".into(), None),
        IdentifierType::Fips => ("fips".into(), None),
        IdentifierType::Gnis => ("gnis".into(), None),
        IdentifierType::OpenStreetMap => ("open_street_map".into(), None),
        IdentifierType::Custom(label) => ("custom".into(), Some(label.clone())),
    }
}

/// Parse a stored `(tag, custom_label)` back into an [`IdentifierType`].
fn parse_ident_type(tag: &str, custom_label: Option<String>) -> IdentifierType {
    match tag {
        "global_location_number" => IdentifierType::GlobalLocationNumber,
        "branch_code" => IdentifierType::BranchCode,
        "fips" => IdentifierType::Fips,
        "gnis" => IdentifierType::Gnis,
        "open_street_map" => IdentifierType::OpenStreetMap,
        _ => IdentifierType::Custom(custom_label.unwrap_or_else(|| tag.to_string())),
    }
}

/// Map a [`DayOfWeek`] to its lowercase tag.
fn day_of_week_str(d: &DayOfWeek) -> String {
    match d {
        DayOfWeek::Monday => "monday",
        DayOfWeek::Tuesday => "tuesday",
        DayOfWeek::Wednesday => "wednesday",
        DayOfWeek::Thursday => "thursday",
        DayOfWeek::Friday => "friday",
        DayOfWeek::Saturday => "saturday",
        DayOfWeek::Sunday => "sunday",
    }
    .to_string()
}

/// Parse a stored day tag back into a [`DayOfWeek`] (defaults to Monday).
fn parse_day_of_week(tag: &str) -> DayOfWeek {
    match tag {
        "tuesday" => DayOfWeek::Tuesday,
        "wednesday" => DayOfWeek::Wednesday,
        "thursday" => DayOfWeek::Thursday,
        "friday" => DayOfWeek::Friday,
        "saturday" => DayOfWeek::Saturday,
        "sunday" => DayOfWeek::Sunday,
        _ => DayOfWeek::Monday,
    }
}

/// Map a SeaORM `DbErr` into the crate error type.
fn map_db(e: sea_orm::DbErr) -> crate::Error {
    crate::Error::Database(e.to_string())
}

pub use audit::AuditLogRepository;
