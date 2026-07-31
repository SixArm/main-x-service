//! `organizations` model — CRUD over the stored
//! `organization_matcher::Organization` payload.

use loco_rs::prelude::*;
use organization_matcher::Organization as MatchOrg;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

/// Re-export the generated entity types so callers use
/// `models::organizations::{Model, ActiveModel, Entity}` (and the CRUD
/// helpers below) from a single path.
pub use super::_entities::organizations::{self, ActiveModel, Entity, Model};

/// Default active-model lifecycle hooks (no custom create/update logic;
/// payload serialization and soft-delete are handled explicitly below).
impl ActiveModelBehavior for super::_entities::organizations::ActiveModel {}

/// Read-side helpers on a fetched `organizations` row.
impl Model {
    /// Deserialize the stored payload into a matcher `Organization`.
    ///
    /// # Errors
    ///
    /// When the stored JSON cannot be parsed.
    pub fn to_org(&self) -> ModelResult<MatchOrg> {
        serde_json::from_value(self.data.clone()).map_err(|e| ModelError::Any(e.into()))
    }

    /// Insert a new organization, returning the created row.
    ///
    /// Generic over [`ConnectionTrait`] so the caller can pass either the
    /// pooled `&DatabaseConnection` (memory transport) or its own
    /// `&DatabaseTransaction` (outbox transport — so the row and its
    /// `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create<C: ConnectionTrait>(db: &C, org: &MatchOrg) -> ModelResult<Self> {
        let data = serde_json::to_value(org).map_err(|e| ModelError::Any(e.into()))?;
        // Mint the public id here (not a DB default) so it is known
        // before the insert, returned to the caller, and bound into the
        // digests below.
        let pid = Uuid::new_v4();
        // All three digests from one call, so none can be stamped
        // without the others (see `record_integrity::digests`). Computed
        // inline rather than stamped afterwards, so one statement writes
        // the row and its integrity values together — there is no window
        // in which a row exists unhashed.
        let digests = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid,
                name: &org.name,
                data: &data,
                active: true,
                deleted_at_micros: None,
            },
        );
        let model = organizations::ActiveModel {
            pid: ActiveValue::set(pid),
            // Denormalise the name for fast list/search.
            name: ActiveValue::set(org.name.clone()),
            // A new record is live, so the digests bind `deleted_at` as
            // `None`.
            content_hash: ActiveValue::set(Some(digests.sha256.clone())),
            content_hash_sha3: ActiveValue::set(Some(digests.sha3.clone())),
            content_mac: ActiveValue::set(digests.mac.clone()),
            data: ActiveValue::set(data),
            active: ActiveValue::set(true),
            deleted_at: ActiveValue::set(None),
            // `id`, `created_at`, `updated_at` are DB/SeaORM-managed.
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Find an active organization by its public id.
    ///
    /// # Errors
    ///
    /// When not found or the query fails.
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        // Parse defensively: a malformed pid is "not found", surfaced as
        // a model error the controller maps to 404 (not a 500).
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let org = organizations::Entity::find()
            .filter(organizations::Column::Pid.eq(uuid))
            // Soft-deleted rows are invisible to lookups.
            .filter(organizations::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        // `None` row ⇒ EntityNotFound, which the controller maps to 404.
        org.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Fetch the active rows for a list of public ids, **preserving the
    /// order of `pids`**.
    ///
    /// The order matters: the caller is a search or blocking query whose
    /// hits are already ranked by relevance, and re-sorting them by row
    /// id would throw that ranking away. Ids that do not resolve (unknown
    /// or soft-deleted — a stale index entry) are simply absent, which is
    /// what keeps a drifted index from resurrecting a deleted record.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_pids(db: &DatabaseConnection, pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        if pids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = organizations::Entity::find()
            .filter(organizations::Column::Pid.is_in(pids.iter().copied()))
            .filter(organizations::Column::DeletedAt.is_null())
            .all(db)
            .await?;
        // Re-order to match `pids` (the SQL `IN` result order is
        // unspecified). Linear in `pids` × `rows`, both bounded by the
        // caller's result limit.
        let mut ordered = Vec::with_capacity(rows.len());
        for pid in pids {
            if let Some(row) = rows.iter().find(|r| r.pid == *pid) {
                ordered.push(row.clone());
            }
        }
        Ok(ordered)
    }

    /// List active organizations (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = organizations::Entity::find()
            .filter(organizations::Column::DeletedAt.is_null())
            .order_by_desc(organizations::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }
}

/// Write-side helpers on a mutable `organizations` active model.
impl ActiveModel {
    /// Replace the payload of an existing organization.
    ///
    /// Generic over [`ConnectionTrait`] so the update can run on a
    /// transaction alongside the outbox insert (see [`Model::create`]).
    ///
    /// # Errors
    ///
    /// When serialization or the update fails.
    pub async fn update_data<C: ConnectionTrait>(
        mut self,
        db: &C,
        org: &MatchOrg,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(org).map_err(|e| ModelError::Any(e.into()))?;
        // Re-digest over the *new* content. Leaving the old digests would
        // report every legitimate edit as tampering, which is as damaging
        // as missing a real one.
        let d = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid: self.pid.as_ref().to_owned(),
                name: &org.name,
                data: &data,
                active: *self.active.as_ref(),
                deleted_at_micros: deleted_at_micros(self.deleted_at.as_ref().as_ref()),
            },
        );
        self.name = ActiveValue::set(org.name.clone());
        self.data = ActiveValue::set(data);
        self.content_hash = ActiveValue::set(Some(d.sha256));
        self.content_hash_sha3 = ActiveValue::set(Some(d.sha3));
        self.content_mac = ActiveValue::set(d.mac);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Soft-delete: mark inactive and stamp `deleted_at`.
    ///
    /// Generic over [`ConnectionTrait`] so the soft-delete can run on a
    /// transaction alongside the outbox insert (see [`Model::create`]).
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn soft_delete<C: ConnectionTrait>(mut self, db: &C) -> ModelResult<Model> {
        self.active = ActiveValue::set(false);
        // `chrono` is the family-standard timestamp type (`SeaORM`'s type
        // for this column); this is a soft-delete stamp, not domain time.
        let stamp: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        self.deleted_at = ActiveValue::set(Some(stamp));
        // `active` and `deleted_at` are both in the pre-image, so a
        // soft delete changes the digest. Not re-digesting here would
        // make every soft-deleted row look tampered with.
        let d = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid: self.pid.as_ref().to_owned(),
                name: self.name.as_ref(),
                data: self.data.as_ref(),
                active: false,
                deleted_at_micros: deleted_at_micros(Some(&stamp)),
            },
        );
        self.content_hash = ActiveValue::set(Some(d.sha256));
        self.content_hash_sha3 = ActiveValue::set(Some(d.sha3));
        self.content_mac = ActiveValue::set(d.mac);
        self.update(db).await.map_err(ModelError::from)
    }
}

/// A soft-delete stamp as epoch microseconds, the form the digest
/// pre-image binds.
///
/// Microseconds because that is what Postgres stores; binding a finer
/// precision than the column keeps would make a row's digest fail to
/// reproduce after a round-trip.
fn deleted_at_micros(stamp: Option<&chrono::DateTime<chrono::FixedOffset>>) -> Option<i64> {
    stamp.map(chrono::DateTime::timestamp_micros)
}

// The `ILIKE '%q%'` name search (and its `escape_like` wildcard guard,
// SEC-G4) lived here until search moved to Tantivy (spec §13). Both were
// removed rather than left dormant: this crate now issues no `LIKE`
// query at all, so an escaper with no caller would only invite a future
// caller to assume it was still wired in. The sibling care-pathway /
// case services keep theirs, because they still search with `ILIKE`.
//
// This module's remaining behaviour is all database access, exercised by
// the Postgres-gated suite (`tests/requests/organizations.rs`); there is
// no pure logic left here to unit-test.
