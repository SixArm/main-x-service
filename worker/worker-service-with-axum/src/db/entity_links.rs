//! `entity_links` persistence — the cross-service linking **write side**
//! (`agents/share/cross-service-linking.md` §4.1) for the worker service.
//!
//! Pure persistence over [`crate::db::models::entity_links`]: [`upsert`]
//! is the idempotent create/update on the
//! `(from_pid, kind, to_ref, valid_from)` unique key (it also **revives**
//! a soft-deleted row so the edge id stays stable across re-asserts),
//! [`list_active`] lists a worker's live outbound edges, [`find_active`]
//! fetches one scoped to a worker, [`list_all_active`] is the
//! aggregator's reconciliation pull across all workers, and
//! [`soft_delete`] withdraws one. Edge **validation** (that `to_ref`
//! parses and the `kind`/endpoint pair is permitted) is DB-free and lives
//! in `api::rest::links`; this layer never touches the far service.
//!
//! Domain dates are `chrono::NaiveDate` (matching the rest of the domain
//! layer); the SeaORM row stores `time::Date`, bridged via
//! [`crate::db::convert`].

use chrono::NaiveDate;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::db::convert::date_to_time;
use crate::db::models::entity_links;

/// The column values for a new (or re-asserted) outbound edge — the
/// validated, ready-to-persist shape the controller hands to [`upsert`].
/// Kept separate from the `SeaORM` `ActiveModel` so the controller does not
/// construct entity internals directly. Dates are domain-side
/// `chrono::NaiveDate`; [`upsert`] bridges them to `time::Date`.
#[derive(Debug, Clone)]
pub struct NewEdge {
    /// The originating worker's `id`.
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
    pub valid_from: Option<NaiveDate>,
    /// Optional validity end.
    pub valid_to: Option<NaiveDate>,
}

/// Idempotently create or update an outbound edge on the
/// `(from_pid, kind, to_ref, valid_from)` unique key: a re-assertion of
/// the same edge updates its mutable columns (role / confidence /
/// provenance / `valid_to`) and **revives** a previously soft-deleted
/// edge (clears `deleted_at`) rather than inserting a duplicate, so the
/// edge id is stable across re-asserts.
///
/// Generic over [`ConnectionTrait`] so the caller can pass either the
/// pooled `&DatabaseConnection` or its own `&DatabaseTransaction`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the insert/upsert fails.
pub async fn upsert<C: ConnectionTrait>(db: &C, edge: &NewEdge) -> Result<entity_links::Model> {
    let am = entity_links::ActiveModel {
        id: ActiveValue::set(Uuid::new_v4()),
        from_pid: ActiveValue::set(edge.from_pid),
        kind: ActiveValue::set(edge.kind.clone()),
        to_ref: ActiveValue::set(edge.to_ref.clone()),
        role: ActiveValue::set(edge.role.clone()),
        confidence: ActiveValue::set(edge.confidence),
        provenance: ActiveValue::set(edge.provenance.clone()),
        valid_from: ActiveValue::set(edge.valid_from.map(date_to_time)),
        valid_to: ActiveValue::set(edge.valid_to.map(date_to_time)),
        deleted_at: ActiveValue::set(None),
        // `created_at` defaults to now() at the DB (NOT set here), so a
        // revived edge keeps its original creation time.
        created_at: ActiveValue::not_set(),
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

/// List a worker's **active** outbound edges (not soft-deleted), newest
/// first.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn list_active(
    db: &DatabaseConnection,
    from_pid: Uuid,
) -> Result<Vec<entity_links::Model>> {
    let rows = entity_links::Entity::find()
        .filter(entity_links::Column::FromPid.eq(from_pid))
        .filter(entity_links::Column::DeletedAt.is_null())
        .order_by_desc(entity_links::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows)
}

/// Find one active edge by its id **scoped to a worker** (`from_pid`), so
/// a caller cannot withdraw an edge belonging to a different worker.
/// Returns `None` when unknown / already withdrawn / wrong worker.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn find_active(
    db: &DatabaseConnection,
    from_pid: Uuid,
    id: Uuid,
) -> Result<Option<entity_links::Model>> {
    let row = entity_links::Entity::find()
        .filter(entity_links::Column::Id.eq(id))
        .filter(entity_links::Column::FromPid.eq(from_pid))
        .filter(entity_links::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    Ok(row)
}

/// Every **active** outbound edge across all workers, oldest first — the
/// authoritative set the link-graph aggregator reconciles against
/// (`GET /api/workers/links`, design §8). `since` (on `created_at`)
/// bounds an incremental pull; `None` is a full replay.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn list_all_active<C: ConnectionTrait>(
    db: &C,
    since: Option<OffsetDateTime>,
) -> Result<Vec<entity_links::Model>> {
    let mut query = entity_links::Entity::find().filter(entity_links::Column::DeletedAt.is_null());
    if let Some(since) = since {
        query = query.filter(entity_links::Column::CreatedAt.gte(since));
    }
    let rows = query
        .order_by_asc(entity_links::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows)
}

/// Soft-delete (withdraw) an edge: stamp `deleted_at = now()`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the update fails.
pub async fn soft_delete<C: ConnectionTrait>(
    db: &C,
    row: entity_links::Model,
) -> Result<entity_links::Model> {
    let mut am: entity_links::ActiveModel = row.into();
    am.deleted_at = ActiveValue::set(Some(OffsetDateTime::now_utc()));
    let updated = am.update(db).await?;
    Ok(updated)
}
