//! `cases` model — CRUD over the stored `case_matcher::Case` payload.

use case_matcher::Case as MatchCase;
use loco_rs::prelude::*;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::Expr;
use sea_orm::{QueryOrder, QuerySelect};
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
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create(db: &DatabaseConnection, case: &MatchCase) -> ModelResult<Self> {
        let data = serde_json::to_value(case).map_err(|e| ModelError::Any(e.into()))?;
        let model = cases::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            title: ActiveValue::set(case.title.clone()),
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
    /// # Errors
    ///
    /// When serialization or the update fails.
    pub async fn update_data(
        mut self,
        db: &DatabaseConnection,
        case: &MatchCase,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(case).map_err(|e| ModelError::Any(e.into()))?;
        self.title = ActiveValue::set(case.title.clone());
        self.data = ActiveValue::set(data);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Soft-delete: mark inactive and stamp `deleted_at`.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn soft_delete(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.active = ActiveValue::set(false);
        self.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::escape_like;

    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("housing"), "housing");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }
}
