//! `entity_links` model — the cross-service **write side**
//! (`agents/share/cross-service-linking.md` §4.1).
//!
//! [`Model::upsert`] is the idempotent create/update on the
//! `(from_pid, kind, to_ref, valid_from)` unique key, [`Model::list_active`]
//! lists a case's live outbound edges, and [`ActiveModel::soft_delete`]
//! withdraws one. Validation of the edge (that `to_ref` parses and the
//! `kind`/endpoint pair is permitted) lives in `controllers/links.rs` and
//! is DB-free; this layer is pure persistence.

use loco_rs::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ConnectionTrait, QueryOrder, prelude::Date, prelude::DateTimeWithTimeZone};
use uuid::Uuid;

pub use super::_entities::entity_links::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::entity_links::ActiveModel {}

/// The column values for a new (or re-asserted) outbound edge — the
/// validated, ready-to-persist shape the controller hands to
/// [`Model::upsert`]. Kept separate from the `SeaORM` `ActiveModel` so the
/// controller does not construct entity internals directly.
#[derive(Debug, Clone)]
pub struct NewEdge {
    /// The originating case's `pid`.
    pub from_pid: Uuid,
    /// The edge kind token (validated against the §9 registry).
    pub kind: String,
    /// The far record's `EntityRef` URN (validated to parse).
    pub to_ref: String,
    /// Optional role label.
    pub role: Option<String>,
    /// Optional confidence in `[0.0, 1.0]`.
    pub confidence: Option<f64>,
    /// Provenance (`operator` by default).
    pub provenance: String,
    /// Optional validity start.
    pub valid_from: Option<Date>,
    /// Optional validity end.
    pub valid_to: Option<Date>,
}

impl Model {
    /// Idempotently create or update an outbound edge on the
    /// `(from_pid, kind, to_ref, valid_from)` unique key: a re-assertion of
    /// the same edge updates its mutable columns (role / confidence /
    /// provenance / `valid_to`) and **revives** a previously soft-deleted
    /// edge (clears `deleted_at`) rather than inserting a duplicate, so the
    /// edge id is stable across re-asserts.
    ///
    /// Generic over [`ConnectionTrait`] so the caller can pass either the
    /// pooled `&DatabaseConnection` (memory transport) or its own
    /// `&DatabaseTransaction` (outbox transport — the row and its
    /// `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When the insert/upsert fails.
    pub async fn upsert<C: ConnectionTrait>(db: &C, edge: &NewEdge) -> ModelResult<Self> {
        let am = entity_links::ActiveModel {
            id: ActiveValue::set(Uuid::new_v4()),
            from_pid: ActiveValue::set(edge.from_pid),
            kind: ActiveValue::set(edge.kind.clone()),
            to_ref: ActiveValue::set(edge.to_ref.clone()),
            role: ActiveValue::set(edge.role.clone()),
            confidence: ActiveValue::set(edge.confidence),
            provenance: ActiveValue::set(edge.provenance.clone()),
            valid_from: ActiveValue::set(edge.valid_from),
            valid_to: ActiveValue::set(edge.valid_to),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        };
        let row = entity_links::Entity::insert(am)
            .on_conflict(
                OnConflict::columns([
                    entity_links::Column::FromPid,
                    entity_links::Column::Kind,
                    entity_links::Column::ToRef,
                    entity_links::Column::ValidFrom,
                ])
                .update_columns([
                    entity_links::Column::Role,
                    entity_links::Column::Confidence,
                    entity_links::Column::Provenance,
                    entity_links::Column::ValidTo,
                    entity_links::Column::DeletedAt,
                ])
                .to_owned(),
            )
            .exec_with_returning(db)
            .await?;
        Ok(row)
    }

    /// List a case's **active** outbound edges (not soft-deleted),
    /// newest first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_active(db: &DatabaseConnection, from_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = entity_links::Entity::find()
            .filter(entity_links::Column::FromPid.eq(from_pid))
            .filter(entity_links::Column::DeletedAt.is_null())
            .order_by_desc(entity_links::Column::CreatedAt)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Find one active edge by its id **scoped to a case** (`from_pid`), so
    /// a caller cannot delete an edge belonging to a different case.
    ///
    /// # Errors
    ///
    /// When not found (unknown / already withdrawn / wrong case) or the
    /// query fails.
    pub async fn find_active(
        db: &DatabaseConnection,
        from_pid: Uuid,
        id: Uuid,
    ) -> ModelResult<Self> {
        let row = entity_links::Entity::find()
            .filter(entity_links::Column::Id.eq(id))
            .filter(entity_links::Column::FromPid.eq(from_pid))
            .filter(entity_links::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        row.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Every **active** outbound edge across all cases, oldest first — the
    /// authoritative set the link-graph aggregator reconciles against
    /// (`GET /api/cases/links`, design §8). `since` (on `created_at`)
    /// bounds an incremental pull; `None` is a full replay.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_all_active<C: ConnectionTrait>(
        db: &C,
        since: Option<DateTimeWithTimeZone>,
    ) -> ModelResult<Vec<Self>> {
        let mut query =
            entity_links::Entity::find().filter(entity_links::Column::DeletedAt.is_null());
        if let Some(since) = since {
            query = query.filter(entity_links::Column::CreatedAt.gte(since));
        }
        let rows = query
            .order_by_asc(entity_links::Column::CreatedAt)
            .all(db)
            .await?;
        Ok(rows)
    }
}

impl ActiveModel {
    /// Soft-delete (withdraw) an edge: stamp `deleted_at`.
    ///
    /// Generic over [`ConnectionTrait`] so it can run on the outbox
    /// transaction alongside the `unlinked` event insert.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn soft_delete<C: ConnectionTrait>(mut self, db: &C) -> ModelResult<Model> {
        self.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}
