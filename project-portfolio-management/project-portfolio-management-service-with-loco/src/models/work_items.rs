//! `work_items` model — CRUD over the stored `project_portfolio_management_matcher::WorkItem`
//! payload, scoped by `kind` so each of the four collections
//! (portfolios / projects / products / programs) sees only its own rows.

use loco_rs::prelude::*;
use project_portfolio_management_matcher::WorkItem as MatchWorkItem;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::work_items::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::work_items::ActiveModel {}

/// Parse a `portfolio_ref` payload string into the denormalised
/// `portfolio_pid` column value: `Some(uuid)` when it is a valid UUID,
/// `None` otherwise (or when absent — e.g. the Portfolio kind).
fn portfolio_pid_of(wi: &MatchWorkItem) -> Option<Uuid> {
    wi.portfolio_ref
        .as_deref()
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
}

impl Model {
    /// Deserialize the stored payload into a matcher `WorkItem`.
    ///
    /// # Errors
    ///
    /// When the stored JSON cannot be parsed.
    pub fn to_work_item(&self) -> ModelResult<MatchWorkItem> {
        serde_json::from_value(self.data.clone()).map_err(|e| ModelError::Any(e.into()))
    }

    /// Insert a new work item into the `kind` collection, returning the
    /// created row. `kind` is the collection discriminator (the caller
    /// has already checked it matches `wi.kind`).
    ///
    /// Generic over [`ConnectionTrait`] so the caller can pass either the
    /// pooled `&DatabaseConnection` (memory transport) or its own
    /// `&DatabaseTransaction` (outbox transport — so the row and its
    /// `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create<C: ConnectionTrait>(
        db: &C,
        kind: &str,
        wi: &MatchWorkItem,
    ) -> ModelResult<Self> {
        let data = serde_json::to_value(wi).map_err(|e| ModelError::Any(e.into()))?;
        let model = work_items::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            kind: ActiveValue::set(kind.to_string()),
            name: ActiveValue::set(wi.name.clone()),
            data: ActiveValue::set(data),
            portfolio_pid: ActiveValue::set(portfolio_pid_of(wi)),
            active: ActiveValue::set(true),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Find an active work item by its public id, scoped to `kind` so a
    /// project pid is never resolved under the products collection.
    ///
    /// # Errors
    ///
    /// When not found or the query fails.
    pub async fn find_by_pid(db: &DatabaseConnection, kind: &str, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let row = work_items::Entity::find()
            .filter(work_items::Column::Kind.eq(kind))
            .filter(work_items::Column::Pid.eq(uuid))
            .filter(work_items::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        row.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// List active work items of `kind` (most-recent first), capped at
    /// `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &DatabaseConnection, kind: &str, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = work_items::Entity::find()
            .filter(work_items::Column::Kind.eq(kind))
            .filter(work_items::Column::DeletedAt.is_null())
            .order_by_desc(work_items::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// List active child work items of `kind` whose denormalised
    /// `portfolio_pid` equals `parent` — the portfolio roll-up.
    ///
    /// # Errors
    ///
    /// When `parent` is not a UUID, or the query fails.
    pub async fn list_children(
        db: &DatabaseConnection,
        kind: &str,
        parent: &str,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let parent_uuid = Uuid::parse_str(parent).map_err(|e| ModelError::Any(e.into()))?;
        let rows = work_items::Entity::find()
            .filter(work_items::Column::Kind.eq(kind))
            .filter(work_items::Column::PortfolioPid.eq(parent_uuid))
            .filter(work_items::Column::DeletedAt.is_null())
            .order_by_desc(work_items::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Case-insensitive substring search on the denormalised `name`, over
    /// active rows of `kind` (Postgres `ILIKE '%q%'`, wildcards escaped).
    /// Full-text / fuzzy search over the JSONB payload via Tantivy is
    /// deferred (spec §13).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(
        db: &DatabaseConnection,
        kind: &str,
        q: &str,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let pattern = format!("%{}%", escape_like(q));
        let rows = work_items::Entity::find()
            .filter(work_items::Column::Kind.eq(kind))
            .filter(work_items::Column::DeletedAt.is_null())
            .filter(Expr::col(work_items::Column::Name).ilike(pattern))
            .order_by_desc(work_items::Column::Id)
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
    /// Replace the payload of an existing work item (and the denormalised
    /// `name` / `portfolio_pid`). The `kind` column is never changed —
    /// a record never moves between collections.
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
        wi: &MatchWorkItem,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(wi).map_err(|e| ModelError::Any(e.into()))?;
        self.name = ActiveValue::set(wi.name.clone());
        self.data = ActiveValue::set(data);
        self.portfolio_pid = ActiveValue::set(portfolio_pid_of(wi));
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
        self.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_like, portfolio_pid_of};
    use project_portfolio_management_matcher::{WorkItem, WorkItemKind};

    /// Pins the `LIKE` escaping: literal text is unchanged, `%` and `_`
    /// are escaped, and the backslash is escaped first.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("apollo"), "apollo");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }

    /// Pins `portfolio_pid_of`: a valid UUID `portfolio_ref` projects to
    /// `Some`, a non-UUID or absent ref to `None`.
    #[test]
    fn portfolio_pid_projection() {
        let mut wi = WorkItem::new(WorkItemKind::Project, "X");
        assert_eq!(portfolio_pid_of(&wi), None);
        wi.portfolio_ref = Some("not-a-uuid".into());
        assert_eq!(portfolio_pid_of(&wi), None);
        let u = uuid::Uuid::new_v4();
        wi.portfolio_ref = Some(u.to_string());
        assert_eq!(portfolio_pid_of(&wi), Some(u));
    }
}
