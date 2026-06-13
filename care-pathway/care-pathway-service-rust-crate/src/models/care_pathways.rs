//! `care_pathways` model — CRUD over the stored
//! `care_pathway_matcher::CarePathway` payload.

use care_pathway_matcher::CarePathway as MatchPathway;
use loco_rs::prelude::*;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::Expr;
use sea_orm::{QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::care_pathways::{self, ActiveModel, Entity, Model};

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
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create(db: &DatabaseConnection, pathway: &MatchPathway) -> ModelResult<Self> {
        let data = serde_json::to_value(pathway).map_err(|e| ModelError::Any(e.into()))?;
        let model = care_pathways::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            name: ActiveValue::set(pathway.name.clone()),
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

/// Escape `LIKE`/`ILIKE` wildcards so a user query matches literally.
fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl ActiveModel {
    /// Replace the payload of an existing care pathway.
    ///
    /// # Errors
    ///
    /// When serialization or the update fails.
    pub async fn update_data(
        mut self,
        db: &DatabaseConnection,
        pathway: &MatchPathway,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(pathway).map_err(|e| ModelError::Any(e.into()))?;
        self.name = ActiveValue::set(pathway.name.clone());
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
        assert_eq!(escape_like("stroke"), "stroke");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }
}
