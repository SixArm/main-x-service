//! `plans` model — CRUD over the stored `project_portfolio_management_matcher::Plan`
//! payload. All plans live in one recursive collection (one table); the
//! `kind` column is an optional descriptive label, and `parent_pid` is
//! the containment link (any plan may contain any other).

use loco_rs::prelude::*;
use project_portfolio_management_matcher::Plan as MatchPlan;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ConnectionTrait, PaginatorTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::plans::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::plans::ActiveModel {}

/// Parse a `parent_ref` payload string into the denormalised
/// `parent_pid` column value: `Some(uuid)` when it is a valid UUID,
/// `None` otherwise (or when absent — e.g. a root plan).
fn parent_pid_of(pl: &MatchPlan) -> Option<Uuid> {
    pl.parent_ref
        .as_deref()
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
}

/// The optional `kind` column value: the serde name of the payload's
/// (optional) `kind` label, or `None`.
fn kind_of(pl: &MatchPlan) -> Option<String> {
    pl.kind.map(|k| {
        // `PlanKind` serialises as its variant name (e.g. "Project").
        serde_json::to_value(k)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default()
    })
}

impl Model {
    /// Deserialize the stored payload into a matcher `Plan`.
    ///
    /// # Errors
    ///
    /// When the stored JSON cannot be parsed.
    pub fn to_plan(&self) -> ModelResult<MatchPlan> {
        serde_json::from_value(self.data.clone()).map_err(|e| ModelError::Any(e.into()))
    }

    /// Insert a new plan, returning the created row.
    ///
    /// Generic over [`ConnectionTrait`] so the caller can pass either the
    /// pooled `&DatabaseConnection` (memory transport) or its own
    /// `&DatabaseTransaction` (outbox transport — so the row and its
    /// `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create<C: ConnectionTrait>(db: &C, pl: &MatchPlan) -> ModelResult<Self> {
        let data = serde_json::to_value(pl).map_err(|e| ModelError::Any(e.into()))?;
        // Mint the pid before the insert so it is known to the digests.
        let pid = Uuid::new_v4();
        let kind = kind_of(pl);
        let parent_pid = parent_pid_of(pl);
        // All three digests from one call, computed inline rather than
        // stamped afterwards: one statement writes the row and its
        // integrity values, so no row ever exists unhashed.
        let digests = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid,
                name: &pl.name,
                kind: kind.as_deref(),
                parent_pid,
                stage: None,
                data: &data,
                active: true,
                deleted_at_micros: None,
            },
        );
        let model = plans::ActiveModel {
            pid: ActiveValue::set(pid),
            kind: ActiveValue::set(kind),
            name: ActiveValue::set(pl.name.clone()),
            content_hash: ActiveValue::set(Some(digests.sha256.clone())),
            content_hash_sha3: ActiveValue::set(Some(digests.sha3.clone())),
            content_mac: ActiveValue::set(digests.mac.clone()),
            data: ActiveValue::set(data),
            parent_pid: ActiveValue::set(parent_pid),
            active: ActiveValue::set(true),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Find an active plan by its public id.
    ///
    /// # Errors
    ///
    /// When not found or the query fails.
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let row = plans::Entity::find()
            .filter(plans::Column::Pid.eq(uuid))
            .filter(plans::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        row.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// List active plans (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        Self::list_paged(db, None, limit, 0).await
    }

    /// One page of active plans (most-recent first), optionally scoped to
    /// a parent's direct children.
    ///
    /// The parent filter and the page window live in one query so that
    /// [`count_for`](Self::count_for) can mirror it exactly: a total that
    /// counted a different predicate from the page would be worse than no
    /// total at all.
    ///
    /// # Errors
    ///
    /// When `parent` is not a UUID, or the query fails.
    pub async fn list_paged(
        db: &DatabaseConnection,
        parent: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> ModelResult<Vec<Self>> {
        let mut query = plans::Entity::find().filter(plans::Column::DeletedAt.is_null());
        if let Some(parent) = parent {
            let parent_uuid = Uuid::parse_str(parent).map_err(|e| ModelError::Any(e.into()))?;
            query = query.filter(plans::Column::ParentPid.eq(parent_uuid));
        }
        let rows = query
            .order_by_desc(plans::Column::Id)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// How many active plans match the same scope
    /// [`list_paged`](Self::list_paged) would page over, ignoring the
    /// window — the `X-Total-Count` a paged response reports.
    ///
    /// # Errors
    ///
    /// When `parent` is not a UUID, or the query fails.
    pub async fn count_for(db: &DatabaseConnection, parent: Option<&str>) -> ModelResult<u64> {
        let mut query = plans::Entity::find().filter(plans::Column::DeletedAt.is_null());
        if let Some(parent) = parent {
            let parent_uuid = Uuid::parse_str(parent).map_err(|e| ModelError::Any(e.into()))?;
            query = query.filter(plans::Column::ParentPid.eq(parent_uuid));
        }
        Ok(query.count(db).await?)
    }

    /// List active child plans whose denormalised `parent_pid` equals
    /// `parent` — the containment roll-up.
    ///
    /// # Errors
    ///
    /// When `parent` is not a UUID, or the query fails.
    pub async fn list_children(
        db: &DatabaseConnection,
        parent: &str,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let parent_uuid = Uuid::parse_str(parent).map_err(|e| ModelError::Any(e.into()))?;
        let rows = plans::Entity::find()
            .filter(plans::Column::ParentPid.eq(parent_uuid))
            .filter(plans::Column::DeletedAt.is_null())
            .order_by_desc(plans::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Walk `parent_pid` from `start` toward the root, returning the chain
    /// of ancestor pids (nearest first). Bounded by `MAX_DEPTH` so a
    /// pre-existing cycle in the data can never loop forever.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn ancestor_pids(db: &DatabaseConnection, start: Uuid) -> ModelResult<Vec<Uuid>> {
        /// Hard cap on how far the containment chain is walked.
        const MAX_DEPTH: usize = 64;
        let mut chain = Vec::new();
        let mut cursor = Some(start);
        while let Some(pid) = cursor {
            if chain.len() >= MAX_DEPTH {
                break;
            }
            let row = plans::Entity::find()
                .filter(plans::Column::Pid.eq(pid))
                .filter(plans::Column::DeletedAt.is_null())
                .one(db)
                .await?;
            match row.and_then(|r| r.parent_pid) {
                Some(parent) => {
                    chain.push(parent);
                    cursor = Some(parent);
                }
                None => cursor = None,
            }
        }
        Ok(chain)
    }

    /// Case-insensitive substring search on the denormalised `name`, over
    /// active rows (Postgres `ILIKE '%q%'`, wildcards escaped). Full-text
    /// / fuzzy search over the JSONB payload via Tantivy is deferred
    /// (spec §13).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(db: &DatabaseConnection, q: &str, limit: u64) -> ModelResult<Vec<Self>> {
        Self::search_paged(db, q, limit, 0).await
    }

    /// One page of name-search hits.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search_paged(
        db: &DatabaseConnection,
        q: &str,
        limit: u64,
        offset: u64,
    ) -> ModelResult<Vec<Self>> {
        let pattern = format!("%{}%", escape_like(q));
        let rows = plans::Entity::find()
            .filter(plans::Column::DeletedAt.is_null())
            .filter(Expr::col(plans::Column::Name).ilike(pattern))
            .order_by_desc(plans::Column::Id)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// How many active plans match `q`, ignoring any window.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search_count(db: &DatabaseConnection, q: &str) -> ModelResult<u64> {
        let pattern = format!("%{}%", escape_like(q));
        let total = plans::Entity::find()
            .filter(plans::Column::DeletedAt.is_null())
            .filter(Expr::col(plans::Column::Name).ilike(pattern))
            .count(db)
            .await?;
        Ok(total)
    }
}

/// Escape `LIKE`/`ILIKE` wildcards so a user query matches literally.
fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl ActiveModel {
    /// Replace the payload of an existing plan (and the denormalised
    /// `name`, `kind`, and `parent_pid`).
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
        pl: &MatchPlan,
    ) -> ModelResult<Model> {
        let data = serde_json::to_value(pl).map_err(|e| ModelError::Any(e.into()))?;
        let kind = kind_of(pl);
        let parent_pid = parent_pid_of(pl);
        // Re-digest over the *new* content: leaving the old digests would
        // report every legitimate edit as tampering.
        let d = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid: self.pid.as_ref().to_owned(),
                name: &pl.name,
                kind: kind.as_deref(),
                parent_pid,
                stage: self.stage.as_ref().as_deref(),
                data: &data,
                active: *self.active.as_ref(),
                deleted_at_micros: self
                    .deleted_at
                    .as_ref()
                    .as_ref()
                    .map(chrono::DateTime::timestamp_micros),
            },
        );
        self.name = ActiveValue::set(pl.name.clone());
        self.kind = ActiveValue::set(kind);
        self.data = ActiveValue::set(data);
        self.parent_pid = ActiveValue::set(parent_pid);
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
        let stamp: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        self.deleted_at = ActiveValue::set(Some(stamp));
        // `active` and `deleted_at` are both in the pre-image, so a soft
        // delete changes the digest; not re-digesting would make every
        // soft-deleted row look tampered with.
        let d = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid: self.pid.as_ref().to_owned(),
                name: self.name.as_ref(),
                kind: self.kind.as_ref().as_deref(),
                parent_pid: *self.parent_pid.as_ref(),
                stage: self.stage.as_ref().as_deref(),
                data: self.data.as_ref(),
                active: false,
                deleted_at_micros: Some(stamp.timestamp_micros()),
            },
        );
        self.content_hash = ActiveValue::set(Some(d.sha256));
        self.content_hash_sha3 = ActiveValue::set(Some(d.sha3));
        self.content_mac = ActiveValue::set(d.mac);
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_like, kind_of, parent_pid_of};
    use project_portfolio_management_matcher::{Plan, PlanKind};

    /// Pins the `LIKE` escaping: literal text is unchanged, `%` and `_`
    /// are escaped, and the backslash is escaped first.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("apollo"), "apollo");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }

    /// Pins `parent_pid_of`: a valid UUID `parent_ref` projects to
    /// `Some`, a non-UUID or absent ref to `None`.
    #[test]
    fn parent_pid_projection() {
        let mut pl = Plan::new("X");
        assert_eq!(parent_pid_of(&pl), None);
        pl.parent_ref = Some("not-a-uuid".into());
        assert_eq!(parent_pid_of(&pl), None);
        let u = uuid::Uuid::new_v4();
        pl.parent_ref = Some(u.to_string());
        assert_eq!(parent_pid_of(&pl), Some(u));
    }

    /// Pins `kind_of`: `None` projects to `None`, a set kind to its serde
    /// variant name.
    #[test]
    fn kind_projection() {
        let mut pl = Plan::new("X");
        assert_eq!(kind_of(&pl), None);
        pl.kind = Some(PlanKind::Project);
        assert_eq!(kind_of(&pl), Some("Project".to_string()));
    }
}
