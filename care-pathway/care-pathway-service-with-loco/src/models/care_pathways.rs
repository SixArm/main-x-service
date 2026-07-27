//! `care_pathways` model — CRUD over the stored
//! `care_pathway_matcher::CarePathway` payload.

use care_pathway_matcher::CarePathway as MatchPathway;
use chrono::SubsecRound as _;
use loco_rs::prelude::*;

use crate::compliance::record_integrity;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

/// Re-export the generated `care_pathways` entity (the module plus
/// `ActiveModel`, `Entity`, and `Model`) so callers use
/// `models::care_pathways::…` rather than reaching into `_entities`.
pub use super::_entities::care_pathways::{self, ActiveModel, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks needed; CRUD
/// side effects (audit, events) live in the controller, not the model.
impl ActiveModelBehavior for super::_entities::care_pathways::ActiveModel {}

impl Model {
    /// Deserialize the stored payload into a matcher `CarePathway`.
    ///
    /// # Errors
    ///
    /// When the stored JSON cannot be parsed.
    pub fn to_pathway(&self) -> ModelResult<MatchPathway> {
        serde_json::from_value(self.data.clone()).map_err(|e| ModelError::Any(e.into()))
    }

    /// Insert a new care pathway, returning the created row.
    ///
    /// Generic over [`ConnectionTrait`] so the caller can pass either the
    /// pooled `&DatabaseConnection` or a `&DatabaseTransaction` (the
    /// `outbox` transport writes this insert on the handler's tx alongside
    /// the `event_outbox` row).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create<C: ConnectionTrait>(db: &C, pathway: &MatchPathway) -> ModelResult<Self> {
        let data = serde_json::to_value(pathway).map_err(|e| ModelError::Any(e.into()))?;
        let pid = Uuid::new_v4();
        // Both digests from one call, so neither can be stamped without
        // the other (see `record_integrity::digests`).
        let digests = record_integrity::digests(&record_integrity::RecordInput {
            pid,
            name: &pathway.name,
            data: &data,
            active: true,
            deleted_at_micros: None,
        });
        let model = care_pathways::ActiveModel {
            pid: ActiveValue::set(pid),
            name: ActiveValue::set(pathway.name.clone()),
            content_hash: ActiveValue::set(Some(digests.0.clone())),
            content_hash_blake3: ActiveValue::set(Some(digests.1.clone())),
            data: ActiveValue::set(data),
            active: ActiveValue::set(true),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Find an active care pathway by its public id.
    ///
    /// # Errors
    ///
    /// When not found or the query fails.
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let row = care_pathways::Entity::find()
            .filter(care_pathways::Column::Pid.eq(uuid))
            .filter(care_pathways::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        row.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// List active care pathways (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = care_pathways::Entity::find()
            .filter(care_pathways::Column::DeletedAt.is_null())
            .order_by_desc(care_pathways::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Case-insensitive substring search on the denormalised `name`, over
    /// active rows (Postgres `ILIKE '%q%'`). This is the pragmatic name
    /// search; full-text / fuzzy search over the JSONB payload via Tantivy
    /// is deferred (spec §13 T-6). `q` is matched literally — `%` and `_`
    /// in the query are escaped so they are not treated as wildcards.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(db: &DatabaseConnection, q: &str, limit: u64) -> ModelResult<Vec<Self>> {
        let pattern = format!("%{}%", escape_like(q));
        let rows = care_pathways::Entity::find()
            .filter(care_pathways::Column::DeletedAt.is_null())
            .filter(Expr::col(care_pathways::Column::Name).ilike(pattern))
            .order_by_desc(care_pathways::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }
}

impl Model {
    /// The newest `limit` rows **including soft-deleted and erased ones**,
    /// for row-level integrity verification.
    ///
    /// Unlike [`Model::list`], this deliberately does not filter on
    /// `deleted_at`: a retired row is exactly where an out-of-band edit is
    /// least likely to be noticed, so it is the row most worth checking.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent_for_integrity(
        db: &DatabaseConnection,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = care_pathways::Entity::find()
            .order_by_desc(care_pathways::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Verify the newest `limit` records by recomputing each content hash.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn verify_records(
        db: &DatabaseConnection,
        limit: u64,
    ) -> ModelResult<record_integrity::RecordIntegrityReport> {
        Ok(record_integrity::verify(
            &Self::recent_for_integrity(db, limit).await?,
        ))
    }
}

/// Escape `LIKE`/`ILIKE` wildcards so a user query matches literally.
fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl ActiveModel {
    /// Replace the payload of an existing care pathway.
    ///
    /// Generic over [`ConnectionTrait`] so the update can run on a
    /// caller-supplied transaction (the `outbox` transport path).
    ///
    /// # Errors
    ///
    /// When serialization or the update fails.
    pub async fn update_data<C: ConnectionTrait>(
        mut self,
        db: &C,
        pathway: &MatchPathway,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(pathway).map_err(|e| ModelError::Any(e.into()))?;
        // The lifecycle fields are untouched by an update, so read them
        // back off the active model to hash the row as it will land.
        let pid = *self.pid.as_ref();
        let active = *self.active.as_ref();
        let deleted_at_micros = self.deleted_at.as_ref().map(|d| d.timestamp_micros());
        let (sha, b3) = record_integrity::digests(&record_integrity::RecordInput {
            pid,
            name: &pathway.name,
            data: &data,
            active,
            deleted_at_micros,
        });
        self.content_hash = ActiveValue::set(Some(sha));
        self.content_hash_blake3 = ActiveValue::set(Some(b3));
        self.name = ActiveValue::set(pathway.name.clone());
        self.data = ActiveValue::set(data);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Soft-delete: mark inactive and stamp `deleted_at`.
    ///
    /// Generic over [`ConnectionTrait`] so the soft-delete can run on a
    /// caller-supplied transaction (the `outbox` transport path).
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn soft_delete<C: ConnectionTrait>(mut self, db: &C) -> ModelResult<Model> {
        // Truncated to microseconds so the value hashed here is the value
        // Postgres returns (see `compliance::record_integrity`).
        let deleted_at: chrono::DateTime<chrono::FixedOffset> =
            chrono::Utc::now().trunc_subsecs(6).into();
        let (sha, b3) = record_integrity::digests(&record_integrity::RecordInput {
            pid: *self.pid.as_ref(),
            name: self.name.as_ref(),
            data: self.data.as_ref(),
            active: false,
            deleted_at_micros: Some(deleted_at.timestamp_micros()),
        });
        self.content_hash = ActiveValue::set(Some(sha));
        self.content_hash_blake3 = ActiveValue::set(Some(b3));
        self.active = ActiveValue::set(false);
        self.deleted_at = ActiveValue::set(Some(deleted_at));
        self.update(db).await.map_err(ModelError::from)
    }
}

/// Tests for the model's pure helpers (DB-free).
#[cfg(test)]
mod tests {
    use super::escape_like;

    /// `escape_like` neutralises `%`, `_`, and `\` so a user query matches
    /// literally and cannot inject `ILIKE` wildcards.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("stroke"), "stroke");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }
}
