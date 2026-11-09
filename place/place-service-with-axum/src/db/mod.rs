//! Database access — the connection pool, the migrator, and the
//! repository. **Every SQL statement this service runs is in this
//! directory** (spec/tech-stack §5.2), written as a `sqlx::query!` /
//! `query_as!` literal so the compiler checks it against the schema.
//!
//! [`PgPlaceRepository`] round-trips a domain [`Place`] against a fully
//! normalized relational schema: scalar fields plus the flattened 0..1
//! address/geo objects live on the `places` row; the keywords, identifiers,
//! `amenity_features`, and `opening_hours` collections live in dedicated child
//! tables joined by `place_id`. Writes are transactional; updates replace
//! the child rows.

pub mod audit;
pub mod convert;
pub mod models;
pub mod outbox;
pub mod review_queue;

use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::db::outbox::OutboxInsert;
use crate::models::address::PostalAddress;
use crate::models::amenity::AmenityFeature;
use crate::models::geo::GeoCoordinates;
use crate::models::identifier::{IdentifierType, PlaceIdentifier};
use crate::models::merge::MergeRecord;
use crate::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};
use crate::models::place::Place;
use crate::models::place_type::PlaceType;
use crate::streaming::envelope::{EventKind, EventTransport};
use models::PlaceRow;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `place-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a connection pool from a [`DatabaseConfig`].
///
/// Applies the configured `max`/`min` pool bounds so the service caps its
/// concurrent database connections (`PostgreSQL` has a finite backend slot
/// budget) while keeping a warm minimum to avoid cold-connect latency on
/// the first request after idle.
///
/// # Errors
/// Returns [`crate::Error::Pool`] if the pool cannot be established (bad
/// URL, unreachable server, auth failure, …).
pub async fn create_connection(config: &DatabaseConfig) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}

/// Storage-agnostic CRUD interface for [`Place`] records.
///
/// Abstracting persistence behind a trait lets `AppState` carry an
/// `Arc<dyn PlaceRepository>` so handlers depend on the contract, not the
/// sqlx implementation — easing testing and any future storage swap.
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
    /// One page of the active places within `radius_m` metres of
    /// `(lat, lon)`, nearest first and then by `id`: `limit` rows after
    /// skipping `offset`.
    ///
    /// Distance is the Haversine great-circle distance on the same
    /// 6 371 km sphere as [`GeoCoordinates::distance_to`], computed in SQL
    /// so the filter, the order and the page are all the database's and
    /// no candidate cap can leave a nearer place out. `bbox` (from
    /// [`crate::matching::geo::search_box`], which wraps across the
    /// antimeridian) is a coarse pre-filter the `idx_places_geo` index
    /// answers; it must contain the whole circle.
    ///
    /// [`GeoCoordinates::distance_to`]: crate::models::geo::GeoCoordinates::distance_to
    ///
    /// # Errors
    /// Propagates any database error from the query or hydration.
    async fn page_within_radius(
        &self,
        lat: f64,
        lon: f64,
        radius_m: f64,
        bbox: crate::geojson::BoundingBox,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Place>>;
    /// One page of the active places below `root` in the
    /// `contained_in_place` hierarchy, at any depth, in `id` order:
    /// `limit` rows after skipping `offset`. `root` itself is not
    /// included. The walk goes through active places only, so the
    /// children of a soft-deleted place are not reached through it. It
    /// stops on a cycle already in the data rather than looping.
    ///
    /// # Errors
    /// Propagates any database error from the query or hydration.
    async fn page_descendants(&self, root: Uuid, offset: u64, limit: u64) -> Result<Vec<Place>>;
    /// One page of the active places whose coordinates fall inside the
    /// box, in `id` order: `limit` rows after skipping `offset`. The edges
    /// are inclusive. When `lon_west > lon_east` the box crosses the
    /// antimeridian (RFC 7946 §5.2), so it covers longitudes at or east of
    /// `lon_west` and at or west of `lon_east`. A place with no
    /// coordinates is never inside a box.
    ///
    /// # Errors
    /// Propagates any database error from the query.
    async fn page_in_bbox(
        &self,
        bbox: crate::geojson::BoundingBox,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Place>>;
    /// Record a merge of `duplicate` into `main` in the merge-history table.
    ///
    /// # Errors
    /// Propagates any database error from the insert.
    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord>;
    /// Merge the `duplicate_id` record into `survivor`: soft-delete the
    /// duplicate **in one transaction**, so the merge commits (or rolls
    /// back) atomically. Under the outbox transport this enqueues a
    /// `Merged` outbox row for the survivor (carrying the duplicate's pid
    /// via `merged_from`) and a `Deleted` outbox row for the duplicate on
    /// the same transaction. Returns the reloaded survivor.
    ///
    /// # Errors
    /// Returns [`crate::Error::NotFound`] if the duplicate or survivor is
    /// absent, or a database error from the merge transaction.
    async fn merge(&self, survivor: &Place, duplicate_id: &Uuid) -> Result<Place>;
}

/// sqlx-backed [`PlaceRepository`] over a `PostgreSQL` connection pool.
pub struct PgPlaceRepository {
    /// The shared connection pool.
    db: PgPool,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the legacy post-commit
    /// in-memory publish only; [`EventTransport::Outbox`] additionally
    /// writes one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl PgPlaceRepository {
    /// Wrap an existing connection pool in a repository. Cheap: the pool
    /// is itself a clonable handle. Defaults to [`EventTransport::Memory`]
    /// (behaviour unchanged from before the outbox landed).
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            transport: EventTransport::Memory,
        }
    }

    /// Builder: select the event transport (see [`EventTransport`]).
    /// `AppState` wires this from `PLACE_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `place` **on `conn`** — pass the open
    /// transaction so the row commits with the entity write (the outbox
    /// atomicity guarantee). A no-op under [`EventTransport::Memory`].
    ///
    /// # Errors
    /// Propagates the outbox insert error (envelope serialize / DB insert).
    async fn enqueue_outbox(
        &self,
        conn: &mut PgConnection,
        place: &Place,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_place(place, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Report whether `place_id` is `parent_id` or one of its ancestors,
    /// i.e. whether setting `place_id`'s `contained_in_place` to
    /// `parent_id` would close a containment cycle (T-16, the multi-hop
    /// half of OQ-2's "validation rejects on insert", alongside
    /// `validate_place`'s pure direct-self-reference check).
    ///
    /// One recursive query walks the chain (T-2), where this used to
    /// cost a round trip per level. `UNION` rather than `UNION ALL`
    /// drops a place already reached, so a pre-existing cycle that does
    /// not involve `place_id` (a row corrupted before this check
    /// existed) ends the walk instead of looping. Soft-deleted places
    /// still count as ancestors, as they did.
    ///
    /// Run it on the write's own transaction, after
    /// [`lock_containment`]: checked before the transaction, two
    /// concurrent edits ("A inside B", "B inside A") could each see no
    /// cycle and both commit one.
    ///
    /// # Errors
    /// Propagates any database error from the lookup.
    async fn ancestor_chain_contains(
        conn: &mut PgConnection,
        place_id: Uuid,
        parent_id: Uuid,
    ) -> Result<bool> {
        let found = sqlx::query_scalar!(
            r#"WITH RECURSIVE chain(id) AS (
                   SELECT $2::uuid
                   UNION
                   SELECT p.contained_in_place
                   FROM places p JOIN chain c ON p.id = c.id
                   WHERE p.contained_in_place IS NOT NULL
               )
               SELECT EXISTS (SELECT 1 FROM chain WHERE id = $1) AS "found!""#,
            place_id,
            parent_id,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(found)
    }

    /// Read one `places` row by id, soft-deleted or not.
    ///
    /// # Errors
    /// Propagates any database error from the lookup.
    async fn find_row(&self, id: Uuid) -> Result<Option<PlaceRow>> {
        let mut conn = self.db.acquire().await?;
        Self::read_row(&mut conn, id).await
    }

    /// Read one `places` row by id on `conn`, soft-deleted or not, so a
    /// write can read its own uncommitted row back.
    ///
    /// # Errors
    /// Propagates any database error from the lookup.
    async fn read_row(conn: &mut PgConnection, id: Uuid) -> Result<Option<PlaceRow>> {
        let row = sqlx::query_as!(
            PlaceRow,
            "SELECT id, name, alternate_name, description, place_type, place_type_custom,
                    address_street_address, address_locality, address_region,
                    address_country, address_postal_code,
                    geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees,
                    geo_elevation_as_decimal_metres,
                    telephone, fax_number, url, global_location_number, branch_code,
                    contained_in_place, is_accessible_for_free, public_access,
                    smoking_allowed, maximum_attendee_capacity, is_deleted,
                    content_hash, content_hash_sha3, content_mac,
                    deleted_at, created_at, updated_at
             FROM places
             WHERE id = $1",
            id,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
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
    async fn hydrate(&self, row: PlaceRow) -> Result<Place> {
        let mut conn = self.db.acquire().await?;
        Self::hydrate_on(&mut conn, row).await
    }

    /// [`Self::hydrate`] on `conn`. `get_by_id`, which verification reads
    /// through, and the digest stamp both assemble through here, so the
    /// record a digest covers is exactly the one verification recomputes.
    ///
    /// # Errors
    /// Propagates any database error from the four child-table queries.
    async fn hydrate_on(conn: &mut PgConnection, row: PlaceRow) -> Result<Place> {
        let id = row.id;

        // Keywords: ordered by stored position to keep list order stable.
        let keywords = sqlx::query_scalar!(
            "SELECT keyword FROM place_keywords WHERE place_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?;

        // Identifiers: the stored `(type tag, custom_label)` pair is decoded
        // back into the rich `IdentifierType` enum.
        let identifiers = sqlx::query!(
            "SELECT identifier_type, custom_label, value
             FROM place_identifiers WHERE place_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| PlaceIdentifier {
            identifier_type: parse_ident_type(&r.identifier_type, r.custom_label),
            value: r.value,
        })
        .collect();

        let amenity_features = sqlx::query_as!(
            AmenityFeature,
            "SELECT name, value
             FROM place_amenity_features WHERE place_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?;

        let opening_hours = sqlx::query!(
            "SELECT day_of_week, opens, closes
             FROM place_opening_hours WHERE place_id = $1 ORDER BY position",
            id,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| OpeningHoursSpecification {
            day_of_week: parse_day_of_week(&r.day_of_week),
            opens: r.opens,
            closes: r.closes,
        })
        .collect();

        let address = build_address(&row);
        let geo = match (
            row.geo_latitude_as_decimal_degrees,
            row.geo_longitude_as_decimal_degrees,
        ) {
            (Some(latitude_as_decimal_degrees), Some(longitude_as_decimal_degrees)) => {
                Some(GeoCoordinates {
                    latitude_as_decimal_degrees,
                    longitude_as_decimal_degrees,
                    elevation_as_decimal_metres: row.geo_elevation_as_decimal_metres,
                })
            }
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
            maximum_attendee_capacity: row
                .maximum_attendee_capacity
                .map(|v| u32::try_from(v).unwrap_or_default()),
            is_deleted: row.is_deleted,
            deleted_at: row.deleted_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Hydrate each row in turn, keeping the query's order.
    ///
    /// # Errors
    /// Propagates any database error from hydration.
    async fn hydrate_all(&self, rows: Vec<PlaceRow>) -> Result<Vec<Place>> {
        let mut conn = self.db.acquire().await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(Self::hydrate_on(&mut conn, row).await?);
        }
        Ok(out)
    }

    /// Stamp `id`'s three digests over the record **as stored**, on the
    /// write's own connection, so they commit with the write.
    ///
    /// Digesting the domain value a caller passed in is what made every
    /// fresh record fail verification (tasks.md INT-V1, event's EVT-V1):
    /// the stored row takes the server's `created_at`/`updated_at`, which
    /// the caller's value did not carry, and Postgres keeps microseconds
    /// and stores NUMERIC coordinates in its own form, so the value read
    /// back was never the value hashed. Reading the record back inside
    /// the transaction makes the digest cover what verification will
    /// recompute, whatever the database normalised.
    ///
    /// `places` has no `updated_at` trigger, and this `UPDATE` names only
    /// the three digest columns, so stamping changes nothing it digests.
    ///
    /// # Errors
    /// When the record cannot be read back or the stamp fails.
    async fn stamp_digests(conn: &mut PgConnection, id: Uuid) -> Result<()> {
        let row = Self::read_row(conn, id).await?.ok_or_else(|| {
            crate::Error::Database("place not found while stamping digests".into())
        })?;
        let stored = Self::hydrate_on(conn, row).await?;
        let digests = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                id: stored.id,
                place: &stored,
                is_deleted: stored.is_deleted,
            },
        );
        sqlx::query!(
            "UPDATE places SET content_hash = $2, content_hash_sha3 = $3, content_mac = $4
             WHERE id = $1",
            id,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PlaceRepository for PgPlaceRepository {
    async fn create(&self, place: &Place) -> Result<Place> {
        // T-16 multi-hop half: `validate_place` already rejects the
        // direct (0-hop) self-reference; this catches a cycle formed
        // through one or more existing ancestors.
        // `created_at`/`updated_at` are server-managed (see `Place`'s
        // field docs): stamp both to "now" here, overriding whatever the
        // domain value carries, rather than trusting a client-supplied —
        // or `#[serde(default)]`-defaulted, i.e. Unix-epoch — timestamp.
        let now = Utc::now();
        let mut row = place_row(place);
        row.created_at = now;
        row.updated_at = now;

        let mut txn = self.db.begin().await?;
        if let Some(parent_id) = place.contained_in_place {
            reject_containment_cycle(&mut txn, place.id, parent_id).await?;
        }
        insert_place_row(&mut txn, &row).await?;
        insert_collections(&mut txn, place).await?;
        // Digests over the record as stored, in this transaction (INT-V1).
        Self::stamp_digests(&mut txn, place.id).await?;
        // Durable event bus (Phase 2): under the outbox transport, write the
        // `event_outbox` row **inside this same transaction**, before commit,
        // so the entity rows and the event commit atomically (or roll back
        // together). A no-op under the memory transport.
        self.enqueue_outbox(&mut txn, place, EventKind::Created)
            .await?;
        txn.commit().await?;
        self.get_by_id(&place.id)
            .await?
            .ok_or_else(|| crate::Error::Database("place not found after insert".into()))
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Place>> {
        let Some(row) = self.find_row(*id).await? else {
            return Ok(None);
        };
        if row.is_deleted {
            return Ok(None);
        }
        Ok(Some(self.hydrate(row).await?))
    }

    async fn update(&self, place: &Place) -> Result<Place> {
        if self.find_row(place.id).await?.is_none() {
            return Err(crate::Error::NotFound);
        }
        // T-16: reject an update that would close a containment cycle
        // through one or more existing ancestors (the case a plain
        // self-reference check in `validate_place` cannot see — this is
        // exactly the "A contains B, then update A to be contained in B"
        // shape).
        // `created_at` is preserved (the UPDATE below does not name it,
        // so whatever the domain value carries — a stale read or a
        // defaulted-empty one — is ignored); `updated_at` is always
        // refreshed to "now". Both are server-managed, matching `create`.
        let mut row = place_row(place);
        row.updated_at = Utc::now();

        let mut txn = self.db.begin().await?;
        if let Some(parent_id) = place.contained_in_place {
            reject_containment_cycle(&mut txn, place.id, parent_id).await?;
        }
        update_place_row(&mut txn, &row).await?;
        delete_collections(&mut txn, place.id).await?;
        insert_collections(&mut txn, place).await?;
        // Digests over the record as stored, in this transaction (INT-V1).
        Self::stamp_digests(&mut txn, place.id).await?;
        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&mut txn, place, EventKind::Updated)
            .await?;
        txn.commit().await?;
        self.get_by_id(&place.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        let row = self.find_row(*id).await?.ok_or(crate::Error::NotFound)?;
        // Under the outbox transport, the tombstone update and the `deleted`
        // outbox row commit atomically in one transaction; the memory
        // transport keeps the plain, tx-free single-row update.
        if self.transport.is_outbox() {
            let place = self.hydrate(row).await?;
            let mut txn = self.db.begin().await?;
            mark_deleted(&mut txn, *id).await?;
            self.enqueue_outbox(&mut txn, &place, EventKind::Deleted)
                .await?;
            txn.commit().await?;
        } else {
            let mut conn = self.db.acquire().await?;
            mark_deleted(&mut conn, *id).await?;
        }
        Ok(())
    }

    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Place>> {
        let rows = sqlx::query_as!(
            PlaceRow,
            "SELECT id, name, alternate_name, description, place_type, place_type_custom,
                    address_street_address, address_locality, address_region,
                    address_country, address_postal_code,
                    geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees,
                    geo_elevation_as_decimal_metres,
                    telephone, fax_number, url, global_location_number, branch_code,
                    contained_in_place, is_accessible_for_free, public_access,
                    smoking_allowed, maximum_attendee_capacity, is_deleted,
                    content_hash, content_hash_sha3, content_mac,
                    deleted_at, created_at, updated_at
             FROM places
             WHERE is_deleted = false
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2",
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(&self.db)
        .await?;
        self.hydrate_all(rows).await
    }

    async fn page_within_radius(
        &self,
        lat: f64,
        lon: f64,
        radius_m: f64,
        bbox: crate::geojson::BoundingBox,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Place>> {
        // The inner query narrows to the box on the index and computes the
        // distance once; the outer one filters, orders and pages on it.
        // `LEAST(1, …)` keeps `asin` inside its domain when rounding pushes
        // the half-chord a hair past 1 for antipodal points.
        let rows = sqlx::query_as!(
            PlaceRow,
            r#"SELECT id, name, alternate_name, description, place_type, place_type_custom,
                    address_street_address, address_locality, address_region,
                    address_country, address_postal_code,
                    geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees,
                    geo_elevation_as_decimal_metres,
                    telephone, fax_number, url, global_location_number, branch_code,
                    contained_in_place, is_accessible_for_free, public_access,
                    smoking_allowed, maximum_attendee_capacity, is_deleted,
                    content_hash, content_hash_sha3, content_mac,
                    deleted_at, created_at, updated_at
             FROM (
                 SELECT p.*,
                        2 * 6371000.0 * asin(LEAST(1.0, sqrt(
                            power(sin(radians(p.geo_latitude_as_decimal_degrees::float8 - $1::float8) / 2), 2)
                            + cos(radians($1::float8))
                              * cos(radians(p.geo_latitude_as_decimal_degrees::float8))
                              * power(sin(radians(p.geo_longitude_as_decimal_degrees::float8 - $2::float8) / 2), 2)
                        ))) AS distance_m
                 FROM places p
                 WHERE p.is_deleted = false
                   AND p.geo_latitude_as_decimal_degrees >= $4
                   AND p.geo_latitude_as_decimal_degrees <= $5
                   AND CASE WHEN $6::numeric <= $7::numeric
                            THEN p.geo_longitude_as_decimal_degrees >= $6
                                 AND p.geo_longitude_as_decimal_degrees <= $7
                            ELSE p.geo_longitude_as_decimal_degrees >= $6
                                 OR p.geo_longitude_as_decimal_degrees <= $7
                       END
             ) candidates
             WHERE distance_m <= $3::float8
             ORDER BY distance_m, id
             OFFSET $8
             LIMIT $9"#,
            lat,
            lon,
            radius_m,
            bbox_decimal(bbox.south),
            bbox_decimal(bbox.north),
            bbox_decimal(bbox.west),
            bbox_decimal(bbox.east),
            to_i64(offset),
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        self.hydrate_all(rows).await
    }

    async fn page_descendants(&self, root: Uuid, offset: u64, limit: u64) -> Result<Vec<Place>> {
        // `UNION` (not `UNION ALL`) drops a place already reached, so a
        // cycle in the data ends the walk; each step uses the index on
        // `contained_in_place`.
        let rows = sqlx::query_as!(
            PlaceRow,
            r#"WITH RECURSIVE tree(id) AS (
                   SELECT $1::uuid
                   UNION
                   SELECT p.id
                   FROM places p JOIN tree t ON p.contained_in_place = t.id
                   WHERE p.is_deleted = false
               )
               SELECT p.id, p.name, p.alternate_name, p.description, p.place_type,
                      p.place_type_custom,
                      p.address_street_address, p.address_locality, p.address_region,
                      p.address_country, p.address_postal_code,
                      p.geo_latitude_as_decimal_degrees, p.geo_longitude_as_decimal_degrees,
                      p.geo_elevation_as_decimal_metres,
                      p.telephone, p.fax_number, p.url, p.global_location_number,
                      p.branch_code, p.contained_in_place, p.is_accessible_for_free,
                      p.public_access, p.smoking_allowed, p.maximum_attendee_capacity,
                      p.is_deleted, p.content_hash, p.content_hash_sha3, p.content_mac,
                      p.deleted_at, p.created_at, p.updated_at
               FROM places p JOIN tree t ON p.id = t.id
               WHERE p.id <> $1
               ORDER BY p.id
               OFFSET $2
               LIMIT $3"#,
            root,
            to_i64(offset),
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        self.hydrate_all(rows).await
    }

    async fn page_in_bbox(
        &self,
        bbox: crate::geojson::BoundingBox,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Place>> {
        // Ordered by the primary key, so a page is the same rows on every
        // fetch and the signed offset in a cursor lands where it left off.
        let rows = sqlx::query_as!(
            PlaceRow,
            "SELECT id, name, alternate_name, description, place_type, place_type_custom,
                    address_street_address, address_locality, address_region,
                    address_country, address_postal_code,
                    geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees,
                    geo_elevation_as_decimal_metres,
                    telephone, fax_number, url, global_location_number, branch_code,
                    contained_in_place, is_accessible_for_free, public_access,
                    smoking_allowed, maximum_attendee_capacity, is_deleted,
                    content_hash, content_hash_sha3, content_mac,
                    deleted_at, created_at, updated_at
             FROM places
             WHERE is_deleted = false
               AND geo_latitude_as_decimal_degrees >= $1
               AND geo_latitude_as_decimal_degrees <= $2
               AND CASE WHEN $3::numeric <= $4::numeric
                        THEN geo_longitude_as_decimal_degrees >= $3
                             AND geo_longitude_as_decimal_degrees <= $4
                        ELSE geo_longitude_as_decimal_degrees >= $3
                             OR geo_longitude_as_decimal_degrees <= $4
                   END
             ORDER BY id
             OFFSET $5
             LIMIT $6",
            bbox_decimal(bbox.south),
            bbox_decimal(bbox.north),
            bbox_decimal(bbox.west),
            bbox_decimal(bbox.east),
            to_i64(offset),
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        self.hydrate_all(rows).await
    }

    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord> {
        sqlx::query!(
            "INSERT INTO place_merge_records
                 (id, main_place_id, duplicate_place_id, merge_reason, transferred_data, merged_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            rec.id,
            rec.main_place_id,
            rec.duplicate_place_id,
            rec.merge_reason,
            rec.transferred_data,
            rec.merged_at,
        )
        .execute(&self.db)
        .await?;
        Ok(rec.clone())
    }

    async fn merge(&self, survivor: &Place, duplicate_id: &Uuid) -> Result<Place> {
        // Fetch the duplicate row up front: it drives the soft-delete and,
        // under the outbox transport, the `Deleted` envelope's denormalised
        // name. Absent ⇒ NotFound (nothing to merge).
        let dup_row = self
            .find_row(*duplicate_id)
            .await?
            .ok_or(crate::Error::NotFound)?;
        // Under the outbox transport, hydrate the duplicate before the
        // transaction opens so the `Deleted` envelope carries its name.
        let dup_place = if self.transport.is_outbox() {
            Some(self.hydrate(dup_row).await?)
        } else {
            None
        };

        // The whole merge — the duplicate soft-delete plus both outbox rows
        // — commits (or rolls back) under one transaction.
        let mut txn = self.db.begin().await?;
        // SEC-B5: the merge was computed from unlocked reads, so lock both
        // rows before writing and check under the lock that both are still
        // active. Otherwise two concurrent merges of one duplicate both
        // apply, and "B into A" racing "A into B" deletes both. The loser
        // gets `Conflict` (409) and writes nothing.
        lock_merge_participants(&mut txn, survivor.id, *duplicate_id).await?;
        mark_deleted(&mut txn, *duplicate_id).await?;
        // Durable event bus (Phase 2): a `Merged` outbox row for the survivor
        // (carrying the duplicate's pid via `merged_from`, so a merge-
        // repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
        if let Some(ref dup) = dup_place {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&mut txn)
                .await?;
            OutboxInsert::for_place(dup, EventKind::Deleted)?
                .insert_on(&mut txn)
                .await?;
        }
        txn.commit().await?;

        self.get_by_id(&survivor.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }
}

/// Count non-deleted places (diagnostics / benches).
///
/// # Errors
/// Propagates any database error from the count query.
pub async fn count_active(db: &PgPool) -> Result<u64> {
    let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM places WHERE is_deleted = false"#)
        .fetch_one(db)
        .await?;
    Ok(u64::try_from(n).unwrap_or_default())
}

/// The rows `GET /api/records/verify` recomputes, most recently updated
/// first, soft-deleted ones included (the handler skips those).
///
/// # Errors
/// Propagates any database error from the query.
pub async fn rows_for_verification(db: &PgPool, limit: u64) -> Result<Vec<PlaceRow>> {
    let rows = sqlx::query_as!(
        PlaceRow,
        "SELECT id, name, alternate_name, description, place_type, place_type_custom,
                address_street_address, address_locality, address_region,
                address_country, address_postal_code,
                geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees,
                geo_elevation_as_decimal_metres,
                telephone, fax_number, url, global_location_number, branch_code,
                contained_in_place, is_accessible_for_free, public_access,
                smoking_allowed, maximum_attendee_capacity, is_deleted,
                content_hash, content_hash_sha3, content_mac,
                deleted_at, created_at, updated_at
         FROM places
         ORDER BY updated_at DESC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Build the full `places` row from a domain [`Place`], digests left
/// empty: they cover the record as stored, which only exists once the row
/// and its children are written, so the write path stamps them afterwards
/// in the same transaction (`stamp_digests`, INT-V1).
fn place_row(place: &Place) -> PlaceRow {
    let (place_type, place_type_custom) = match &place.place_type {
        Some(t) => {
            let (tag, custom) = place_type_parts(t);
            (Some(tag), custom)
        }
        None => (None, None),
    };
    let address = place.address.as_ref();
    let geo = place.geo.as_ref();
    PlaceRow {
        id: place.id,
        content_hash: None,
        content_hash_sha3: None,
        content_mac: None,
        name: place.name.clone(),
        alternate_name: place.alternate_name.clone(),
        description: place.description.clone(),
        place_type,
        place_type_custom,
        address_street_address: address.and_then(|a| a.street_address.clone()),
        address_locality: address.and_then(|a| a.address_locality.clone()),
        address_region: address.and_then(|a| a.address_region.clone()),
        address_country: address.and_then(|a| a.address_country.clone()),
        address_postal_code: address.and_then(|a| a.postal_code.clone()),
        geo_latitude_as_decimal_degrees: geo.map(|g| g.latitude_as_decimal_degrees.clone()),
        geo_longitude_as_decimal_degrees: geo.map(|g| g.longitude_as_decimal_degrees.clone()),
        geo_elevation_as_decimal_metres: geo.and_then(|g| g.elevation_as_decimal_metres.clone()),
        telephone: place.telephone.clone(),
        fax_number: place.fax_number.clone(),
        url: place.url.clone(),
        global_location_number: place.global_location_number.clone(),
        branch_code: place.branch_code.clone(),
        contained_in_place: place.contained_in_place,
        is_accessible_for_free: place.is_accessible_for_free,
        public_access: place.public_access,
        smoking_allowed: place.smoking_allowed,
        maximum_attendee_capacity: place
            .maximum_attendee_capacity
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        is_deleted: place.is_deleted,
        deleted_at: place.deleted_at,
        created_at: place.created_at,
        updated_at: place.updated_at,
    }
}

/// `INSERT` one whole `places` row.
/// Serialise every write that sets a `contained_in_place`, for the rest
/// of `conn`'s transaction, so the cycle check and the write it guards
/// see no concurrent hierarchy edit. One key for the whole table: a
/// cycle can join any two places, so no narrower key is safe, and only
/// writes that name a parent take it.
///
/// # Errors
/// Propagates any database error from taking the lock.
async fn lock_containment(conn: &mut PgConnection) -> Result<()> {
    sqlx::query_scalar!(
        r#"SELECT pg_advisory_xact_lock(hashtext('place:contained_in_place')) AS "locked!: ()""#
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(())
}

/// Take [`lock_containment`], then refuse with `409` when making
/// `parent_id` the parent of `place_id` would close a cycle (T-16).
///
/// # Errors
/// [`crate::Error::Conflict`] on a cycle; any database error otherwise.
async fn reject_containment_cycle(
    conn: &mut PgConnection,
    place_id: Uuid,
    parent_id: Uuid,
) -> Result<()> {
    lock_containment(conn).await?;
    if PgPlaceRepository::ancestor_chain_contains(conn, place_id, parent_id).await? {
        return Err(crate::Error::Conflict(
            "contained_in_place would create a containment cycle".into(),
        ));
    }
    Ok(())
}

async fn insert_place_row(conn: &mut PgConnection, row: &PlaceRow) -> Result<()> {
    sqlx::query!(
        "INSERT INTO places (
             id, name, alternate_name, description, place_type, place_type_custom,
             address_street_address, address_locality, address_region,
             address_country, address_postal_code,
             geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees,
             geo_elevation_as_decimal_metres,
             telephone, fax_number, url, global_location_number, branch_code,
             contained_in_place, is_accessible_for_free, public_access,
             smoking_allowed, maximum_attendee_capacity, is_deleted,
             content_hash, content_hash_sha3, content_mac,
             deleted_at, created_at, updated_at
         ) VALUES (
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
             $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31
         )",
        row.id,
        row.name,
        row.alternate_name,
        row.description,
        row.place_type,
        row.place_type_custom,
        row.address_street_address,
        row.address_locality,
        row.address_region,
        row.address_country,
        row.address_postal_code,
        row.geo_latitude_as_decimal_degrees,
        row.geo_longitude_as_decimal_degrees,
        row.geo_elevation_as_decimal_metres,
        row.telephone,
        row.fax_number,
        row.url,
        row.global_location_number,
        row.branch_code,
        row.contained_in_place,
        row.is_accessible_for_free,
        row.public_access,
        row.smoking_allowed,
        row.maximum_attendee_capacity,
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

/// `UPDATE` every column of one `places` row except `id` and
/// `created_at`, which never change after insert.
async fn update_place_row(conn: &mut PgConnection, row: &PlaceRow) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE places SET
             name = $2, alternate_name = $3, description = $4,
             place_type = $5, place_type_custom = $6,
             address_street_address = $7, address_locality = $8, address_region = $9,
             address_country = $10, address_postal_code = $11,
             geo_latitude_as_decimal_degrees = $12, geo_longitude_as_decimal_degrees = $13,
             geo_elevation_as_decimal_metres = $14,
             telephone = $15, fax_number = $16, url = $17,
             global_location_number = $18, branch_code = $19,
             contained_in_place = $20, is_accessible_for_free = $21, public_access = $22,
             smoking_allowed = $23, maximum_attendee_capacity = $24, is_deleted = $25,
             content_hash = $26, content_hash_sha3 = $27, content_mac = $28,
             deleted_at = $29, updated_at = $30
         WHERE id = $1",
        row.id,
        row.name,
        row.alternate_name,
        row.description,
        row.place_type,
        row.place_type_custom,
        row.address_street_address,
        row.address_locality,
        row.address_region,
        row.address_country,
        row.address_postal_code,
        row.geo_latitude_as_decimal_degrees,
        row.geo_longitude_as_decimal_degrees,
        row.geo_elevation_as_decimal_metres,
        row.telephone,
        row.fax_number,
        row.url,
        row.global_location_number,
        row.branch_code,
        row.contained_in_place,
        row.is_accessible_for_free,
        row.public_access,
        row.smoking_allowed,
        row.maximum_attendee_capacity,
        row.is_deleted,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.deleted_at,
        row.updated_at,
    )
    .execute(conn)
    .await?;
    // The caller checked the row exists; one that vanished in between is
    // an error, as it was when the ORM reported "no record updated".
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "place not updated: no such row".into(),
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
        sqlx::query_scalar!("SELECT id FROM places WHERE id = $1 FOR UPDATE", id)
            .fetch_optional(&mut *conn)
            .await?;
    }
    let active = sqlx::query_scalar!(
        r#"SELECT count(*) AS "active!" FROM places WHERE id = ANY($1) AND is_deleted = false"#,
        &ids[..],
    )
    .fetch_one(&mut *conn)
    .await?;
    if active == 2 {
        Ok(())
    } else {
        Err(crate::Error::Conflict(format!(
            "place {survivor_id} or {duplicate_id} is no longer active (merged or deleted meanwhile)"
        )))
    }
}

/// Soft-delete one place: set the tombstone and its timestamp. Nothing
/// else on the row changes, the stored digests included.
async fn mark_deleted(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE places SET is_deleted = true, deleted_at = $2, updated_at = $2 WHERE id = $1",
        id,
        now,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Reconstruct a [`PostalAddress`] from the flattened row, or `None` when
/// every address column is empty.
fn build_address(row: &PlaceRow) -> Option<PostalAddress> {
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
/// Each row's `position` is its index, so `hydrate` reads the lists back
/// in the order they were written.
async fn insert_collections(conn: &mut PgConnection, place: &Place) -> Result<()> {
    for (i, keyword) in place.keywords.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO place_keywords (id, place_id, keyword, position)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            place.id,
            keyword,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, ident) in place.identifiers.iter().enumerate() {
        let (identifier_type, custom_label) = ident_type_parts(&ident.identifier_type);
        sqlx::query!(
            "INSERT INTO place_identifiers
                 (id, place_id, identifier_type, custom_label, value, position)
             VALUES ($1, $2, $3, $4, $5, $6)",
            Uuid::new_v4(),
            place.id,
            identifier_type,
            custom_label,
            ident.value,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, am) in place.amenity_features.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO place_amenity_features (id, place_id, name, value, position)
             VALUES ($1, $2, $3, $4, $5)",
            Uuid::new_v4(),
            place.id,
            am.name,
            am.value,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    for (i, oh) in place.opening_hours.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO place_opening_hours (id, place_id, day_of_week, opens, closes, position)
             VALUES ($1, $2, $3, $4, $5, $6)",
            Uuid::new_v4(),
            place.id,
            day_of_week_str(&oh.day_of_week),
            oh.opens,
            oh.closes,
            position(i),
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Delete all child-collection rows for `place_id` on connection `conn`.
async fn delete_collections(conn: &mut PgConnection, place_id: Uuid) -> Result<()> {
    sqlx::query!("DELETE FROM place_keywords WHERE place_id = $1", place_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!(
        "DELETE FROM place_identifiers WHERE place_id = $1",
        place_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM place_amenity_features WHERE place_id = $1",
        place_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM place_opening_hours WHERE place_id = $1",
        place_id
    )
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

/// Convert a bounding-box bound (already-clamped, finite) into the
/// decimal the `NUMERIC` geo columns compare against.
///
/// Unlike the domain [`GeoCoordinates`] constructor, this never panics on
/// a pathological input: it is a query bound, not stored data, so a
/// value that somehow fails to parse degrades to `0` (an inert, always-
/// finite fallback) rather than taking down the request.
fn bbox_decimal(value: f64) -> bigdecimal::BigDecimal {
    format!("{value}").parse().unwrap_or_default()
}

pub use audit::AuditLogRepository;

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE under
/// a bare `cargo test --lib`.
#[cfg(test)]
mod tests {
    use super::{PgPlaceRepository, PlaceRepository};
    use crate::db::outbox;
    use crate::models::place::Place;
    use crate::streaming::envelope::EventTransport;

    async fn connect() -> sqlx::PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sqlx::PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Create writes, in one transaction, a `created` outbox row.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let place = repo.create(&Place::new("Central Park")).await.unwrap();

        let created_rows = outbox::rows_for(&db, place.id, "created").await.unwrap();
        assert_eq!(
            created_rows.len(),
            1,
            "one created outbox row for the place"
        );
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&Place::new("Survivor")).await.unwrap();
        let duplicate = repo.create(&Place::new("Duplicate")).await.unwrap();

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

    /// T-16 multi-hop cycle: A contains B (`B.contained_in_place = A`),
    /// then updating A to be contained in B must be rejected — the
    /// direct self-reference case is covered by `validate_place`'s pure
    /// unit tests; this is the repository-level ancestor-chain walk.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn update_rejects_a_cycle_through_an_existing_ancestor() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone());

        let mut a = Place::new("A");
        a = repo.create(&a).await.unwrap();
        let mut b = Place::new("B");
        b.contained_in_place = Some(a.id);
        b = repo.create(&b).await.unwrap();

        // A → B would close the cycle A → B → A.
        a.contained_in_place = Some(b.id);
        let err = repo.update(&a).await.unwrap_err();
        assert!(
            matches!(err, crate::Error::Conflict(_)),
            "expected Conflict, got {err:?}"
        );

        // The rejected update must not have persisted: A is still a root.
        let reloaded = repo.get_by_id(&a.id).await.unwrap().unwrap();
        assert_eq!(reloaded.contained_in_place, None);
    }

    /// T-2: the recursive walk finds a cycle four levels up, where the
    /// old loop took a round trip per level.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn update_rejects_a_cycle_four_levels_up() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone());

        let mut root = repo.create(&Place::new("Level 0")).await.unwrap();
        let mut parent = root.id;
        for level in 1..=4 {
            let mut child = Place::new(&format!("Level {level}"));
            child.contained_in_place = Some(parent);
            parent = repo.create(&child).await.unwrap().id;
        }

        root.contained_in_place = Some(parent);
        let err = repo.update(&root).await.unwrap_err();
        assert!(
            matches!(err, crate::Error::Conflict(_)),
            "expected Conflict, got {err:?}"
        );
    }

    /// A cycle already in the table that does not involve the place being
    /// written ends the walk: the write is allowed, not looped on.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn a_pre_existing_unrelated_cycle_does_not_stop_a_write() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone());

        let p = repo.create(&Place::new("Cycle P")).await.unwrap();
        let q = repo.create(&Place::new("Cycle Q")).await.unwrap();
        // Corrupt rows, as from before the check existed: P and Q contain
        // each other.
        sqlx::query("UPDATE places SET contained_in_place = $1 WHERE id = $2")
            .bind(q.id)
            .bind(p.id)
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("UPDATE places SET contained_in_place = $1 WHERE id = $2")
            .bind(p.id)
            .bind(q.id)
            .execute(&db)
            .await
            .unwrap();

        let mut r = Place::new("Inside the cycle");
        r.contained_in_place = Some(p.id);
        let created = repo.create(&r).await.unwrap();
        assert_eq!(created.contained_in_place, Some(p.id));
    }

    /// The cycle check and the write share one transaction under the
    /// containment lock. Here another writer holds that lock and has put
    /// B inside A without committing. An edit putting A inside B must wait
    /// for it, then see B inside A and refuse. Checked outside the
    /// transaction, as it was, the edit saw B as a root and both writes
    /// committed a cycle.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn a_concurrent_hierarchy_edit_cannot_close_a_cycle() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone());
        let mut a = repo.create(&Place::new("Race A")).await.unwrap();
        let b = repo.create(&Place::new("Race B")).await.unwrap();

        let mut other = db.begin().await.unwrap();
        super::lock_containment(&mut other).await.unwrap();
        sqlx::query("UPDATE places SET contained_in_place = $1 WHERE id = $2")
            .bind(a.id)
            .bind(b.id)
            .execute(&mut *other)
            .await
            .unwrap();

        a.contained_in_place = Some(b.id);
        let editor = PgPlaceRepository::new(db.clone());
        let edit = tokio::spawn(async move { editor.update(&a).await });

        // Wait, up to ten seconds, until Postgres reports the edit queued
        // on the containment lock.
        let mut blocked = false;
        for _ in 0..200 {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_locks
                 WHERE locktype = 'advisory' AND NOT granted AND objsubid = 1
                   AND objid = (hashtext('place:contained_in_place')::bigint & 4294967295)::oid",
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

        let result = edit.await.unwrap();
        assert!(blocked, "the edit never waited on the containment lock");
        assert!(
            matches!(result, Err(crate::Error::Conflict(_))),
            "expected Conflict, got {result:?}"
        );
    }

    /// SEC-B5: a merge locks both rows and rechecks them under the lock.
    /// Another transaction is soft-deleting the survivor and holds its
    /// row; the merge waits for it, then finds the survivor gone and
    /// answers `Conflict` without deleting the duplicate. Without the lock
    /// "B into A" racing "A into B" deleted both.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn a_merge_waits_for_a_concurrent_delete_and_then_refuses() {
        let db = connect().await;
        let repo = PgPlaceRepository::new(db.clone());
        let survivor = repo.create(&Place::new("Race Survivor")).await.unwrap();
        let duplicate = repo.create(&Place::new("Race Duplicate")).await.unwrap();

        let mut other = db.begin().await.unwrap();
        super::mark_deleted(&mut other, survivor.id).await.unwrap();

        let merger = PgPlaceRepository::new(db.clone());
        let main = survivor.clone();
        let dup_id = duplicate.id;
        let merge = tokio::spawn(async move { merger.merge(&main, &dup_id).await });

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
        assert!(blocked, "the merge never waited on the survivor's row lock");
        assert!(
            matches!(result, Err(crate::Error::Conflict(_))),
            "expected Conflict, got {result:?}"
        );
        assert!(
            repo.get_by_id(&duplicate.id).await.unwrap().is_some(),
            "the duplicate was not deleted"
        );
    }
}
