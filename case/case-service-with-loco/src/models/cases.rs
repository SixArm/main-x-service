//! `cases` model — CRUD over the stored `case_matcher::Case` payload.

use case_matcher::Case as MatchCase;
use chrono::SubsecRound as _;
use loco_rs::prelude::*;

use crate::compliance::record_integrity;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::cases::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::cases::ActiveModel {}

impl Model {
    /// Deserialize the stored payload into a matcher `Case`.
    ///
    /// # Errors
    ///
    /// When the stored JSON cannot be parsed.
    pub fn to_case(&self) -> ModelResult<MatchCase> {
        serde_json::from_value(self.data.clone()).map_err(|e| ModelError::Any(e.into()))
    }

    /// Insert a new case, returning the created row.
    ///
    /// Generic over [`ConnectionTrait`] so the caller can pass either the
    /// pooled `&DatabaseConnection` (memory transport) or its own
    /// `&DatabaseTransaction` (outbox transport — so the row and its
    /// `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create<C: ConnectionTrait>(db: &C, case: &MatchCase) -> ModelResult<Self> {
        let data = serde_json::to_value(case).map_err(|e| ModelError::Any(e.into()))?;
        let pid = Uuid::new_v4();
        // Both digests from one call, so neither can be stamped without
        // the other (see `record_integrity::digests`).
        let digests = record_integrity::digests(&record_integrity::RecordInput {
            pid,
            title: &case.title,
            data: &data,
            active: true,
            deleted_at_micros: None,
        });
        let model = cases::ActiveModel {
            pid: ActiveValue::set(pid),
            title: ActiveValue::set(case.title.clone()),
            // A new record is live, so the digest binds `deleted_at` as
            // `None`.
            content_hash: ActiveValue::set(Some(digests.sha256.clone())),
            content_hash_blake3: ActiveValue::set(Some(digests.blake3.clone())),
            content_hash_sha3: ActiveValue::set(Some(digests.sha3.clone())),
            data: ActiveValue::set(data),
            active: ActiveValue::set(true),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Find an active case by its public id.
    ///
    /// # Errors
    ///
    /// When not found or the query fails.
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let row = cases::Entity::find()
            .filter(cases::Column::Pid.eq(uuid))
            .filter(cases::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        row.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// List active cases (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = cases::Entity::find()
            .filter(cases::Column::DeletedAt.is_null())
            .order_by_desc(cases::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Case-insensitive substring search on the denormalised `title`, over
    /// active rows (Postgres `ILIKE '%q%'`). This is the pragmatic title
    /// search; full-text / fuzzy search over the JSONB payload via Tantivy
    /// is deferred (spec §13 T-6). `q` is matched literally — `%` and `_`
    /// in the query are escaped so they are not treated as wildcards.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(db: &DatabaseConnection, q: &str, limit: u64) -> ModelResult<Vec<Self>> {
        let pattern = format!("%{}%", escape_like(q));
        let rows = cases::Entity::find()
            .filter(cases::Column::DeletedAt.is_null())
            .filter(Expr::col(cases::Column::Title).ilike(pattern))
            .order_by_desc(cases::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// The most recently updated `limit` cases, **including soft-deleted
    /// and erased rows**, for integrity verification.
    ///
    /// Deliberately not filtered to live rows: a retired or erased case
    /// still carries a digest, and an attacker editing a row nobody reads
    /// any more is exactly the case worth catching.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent_for_integrity<C: ConnectionTrait>(
        db: &C,
        limit: u64,
    ) -> ModelResult<Vec<Model>> {
        Ok(cases::Entity::find()
            .order_by_desc(cases::Column::UpdatedAt)
            .limit(limit)
            .all(db)
            .await?)
    }

    /// Recompute every recent record's content hash and report mismatches.
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
    /// Replace the payload of an existing case.
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
        case: &MatchCase,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(case).map_err(|e| ModelError::Any(e.into()))?;
        // The lifecycle fields are untouched by an update, so read them
        // back off the active model to hash the row as it will land.
        let pid = *self.pid.as_ref();
        let active = *self.active.as_ref();
        let deleted_at_micros = self.deleted_at.as_ref().map(|d| d.timestamp_micros());
        let d = record_integrity::digests(&record_integrity::RecordInput {
            pid,
            title: &case.title,
            data: &data,
            active,
            deleted_at_micros,
        });
        self.content_hash = ActiveValue::set(Some(d.sha256));
        self.content_hash_blake3 = ActiveValue::set(Some(d.blake3));
        self.content_hash_sha3 = ActiveValue::set(Some(d.sha3));
        self.title = ActiveValue::set(case.title.clone());
        self.data = ActiveValue::set(data);
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
        // Truncated to microseconds so the value hashed here is the value
        // Postgres returns (see `compliance::record_integrity`).
        let deleted_at: chrono::DateTime<chrono::FixedOffset> =
            chrono::Utc::now().trunc_subsecs(6).into();
        // A soft delete changes the lifecycle state the digest binds, so
        // the row rehashes — otherwise every deleted case would read as
        // tampered.
        let d = record_integrity::digests(&record_integrity::RecordInput {
            pid: *self.pid.as_ref(),
            title: self.title.as_ref(),
            data: self.data.as_ref(),
            active: false,
            deleted_at_micros: Some(deleted_at.timestamp_micros()),
        });
        self.content_hash = ActiveValue::set(Some(d.sha256));
        self.content_hash_blake3 = ActiveValue::set(Some(d.blake3));
        self.content_hash_sha3 = ActiveValue::set(Some(d.sha3));
        self.active = ActiveValue::set(false);
        self.deleted_at = ActiveValue::set(Some(deleted_at));
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::escape_like;

    /// Pins the `LIKE` escaping: literal text is unchanged, `%` and `_`
    /// are escaped so they match literally, and the backslash is escaped
    /// first so a user backslash cannot re-enable a wildcard.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("housing"), "housing");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }
}
