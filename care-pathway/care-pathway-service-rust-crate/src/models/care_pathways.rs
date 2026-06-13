//! `care_pathways` model — CRUD over the stored
//! `care_pathway_matcher::CarePathway` payload.

use care_pathway_matcher::CarePathway as MatchPathway;
use loco_rs::prelude::*;
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
