//! `organizations` model — CRUD over the stored
//! `organization_matcher::Organization` payload.

use loco_rs::prelude::*;
use organization_matcher::Organization as MatchOrg;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::Expr;
use sea_orm::{QueryOrder, QuerySelect};
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
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create(db: &DatabaseConnection, org: &MatchOrg) -> ModelResult<Self> {
        let data = serde_json::to_value(org).map_err(|e| ModelError::Any(e.into()))?;
        let model = organizations::ActiveModel {
            // Mint the public id here (not a DB default) so it is known
            // before the insert and returned to the caller.
            pid: ActiveValue::set(Uuid::new_v4()),
            // Denormalise the name for fast list/search.
            name: ActiveValue::set(org.name.clone()),
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

    /// Case-insensitive substring search on the denormalised `name`,
    /// over active rows. (Postgres `ILIKE '%q%'`.)
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(db: &DatabaseConnection, q: &str, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = organizations::Entity::find()
            .filter(organizations::Column::DeletedAt.is_null())
            .filter(Expr::col(organizations::Column::Name).ilike(format!("%{q}%")))
            .order_by_desc(organizations::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
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
    /// # Errors
    ///
    /// When serialization or the update fails.
    pub async fn update_data(
        mut self,
        db: &DatabaseConnection,
        org: &MatchOrg,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(org).map_err(|e| ModelError::Any(e.into()))?;
        self.name = ActiveValue::set(org.name.clone());
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
        // `chrono` here is `SeaORM`'s timestamp type for this column (the
        // `with-jiff` migration is tracked separately); this is a
        // soft-delete stamp, not domain time.
        self.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}
