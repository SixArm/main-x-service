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
//! layer); the row handed back stores `time::Date`, as it did under
//! `SeaORM`, bridged via [`crate::db::convert`].

use axa::sqlx::Db;
use chrono::NaiveDate;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::db::convert::offset_to_ts;
use crate::db::models::{EntityLinkRaw, entity_links};

/// The column values for a new (or re-asserted) outbound edge — the
/// validated, ready-to-persist shape the controller hands to [`upsert`].
/// Dates are domain-side `chrono::NaiveDate`.
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
/// Takes a [`Db`], so the caller can pass either the pool or its own open
/// transaction. `created_at` defaults to `now()` at the database (it is not
/// in the insert), so a revived edge keeps its original creation time.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the insert/upsert fails.
pub async fn upsert(db: impl Into<Db<'_>>, edge: &NewEdge) -> Result<entity_links::Model> {
    let mut conn = db.into().acquire().await?;
    let row = sqlx::query_as!(
        EntityLinkRaw,
        "INSERT INTO entity_links
             (id, from_pid, kind, to_ref, role, confidence, provenance, valid_from, valid_to,
              deleted_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)
         ON CONFLICT (from_pid, kind, to_ref, valid_from) DO UPDATE SET
             role = EXCLUDED.role,
             confidence = EXCLUDED.confidence,
             provenance = EXCLUDED.provenance,
             valid_to = EXCLUDED.valid_to,
             deleted_at = EXCLUDED.deleted_at
         RETURNING id, from_pid, kind, to_ref, role, confidence, provenance, valid_from,
                   valid_to, created_at, deleted_at",
        Uuid::new_v4(),
        edge.from_pid,
        edge.kind,
        edge.to_ref,
        edge.role,
        edge.confidence,
        edge.provenance,
        edge.valid_from,
        edge.valid_to,
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.into())
}

/// List a worker's **active** outbound edges (not soft-deleted), newest
/// first.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn list_active(db: &PgPool, from_pid: Uuid) -> Result<Vec<entity_links::Model>> {
    let rows = sqlx::query_as!(
        EntityLinkRaw,
        "SELECT id, from_pid, kind, to_ref, role, confidence, provenance, valid_from,
                valid_to, created_at, deleted_at
         FROM entity_links
         WHERE from_pid = $1 AND deleted_at IS NULL
         ORDER BY created_at DESC",
        from_pid,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Find one active edge by its id **scoped to a worker** (`from_pid`), so
/// a caller cannot withdraw an edge belonging to a different worker.
/// Returns `None` when unknown / already withdrawn / wrong worker.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn find_active(
    db: &PgPool,
    from_pid: Uuid,
    id: Uuid,
) -> Result<Option<entity_links::Model>> {
    let row = sqlx::query_as!(
        EntityLinkRaw,
        "SELECT id, from_pid, kind, to_ref, role, confidence, provenance, valid_from,
                valid_to, created_at, deleted_at
         FROM entity_links
         WHERE id = $1 AND from_pid = $2 AND deleted_at IS NULL
         LIMIT 1",
        id,
        from_pid,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(Into::into))
}

/// Every **active** outbound edge across all workers, oldest first — the
/// authoritative set the link-graph aggregator reconciles against
/// (`GET /api/workers/links`, design §8). `since` (on `created_at`)
/// bounds an incremental pull; `None` is a full replay.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn list_all_active(
    db: impl Into<Db<'_>>,
    since: Option<OffsetDateTime>,
) -> Result<Vec<entity_links::Model>> {
    let mut conn = db.into().acquire().await?;
    let rows = sqlx::query_as!(
        EntityLinkRaw,
        "SELECT id, from_pid, kind, to_ref, role, confidence, provenance, valid_from,
                valid_to, created_at, deleted_at
         FROM entity_links
         WHERE deleted_at IS NULL AND ($1::timestamptz IS NULL OR created_at >= $1)
         ORDER BY created_at ASC",
        since.map(offset_to_ts),
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Soft-delete (withdraw) an edge: stamp `deleted_at = now()`.
///
/// Only `deleted_at` is written, as `SeaORM`'s update of the loaded model
/// did, and an id that matches no row is an error.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the update fails or matches no
/// row.
pub async fn soft_delete(
    db: impl Into<Db<'_>>,
    row: entity_links::Model,
) -> Result<entity_links::Model> {
    let mut conn = db.into().acquire().await?;
    let deleted_at = offset_to_ts(OffsetDateTime::now_utc());
    let updated = sqlx::query_as!(
        EntityLinkRaw,
        "UPDATE entity_links SET deleted_at = $2
         WHERE id = $1
         RETURNING id, from_pid, kind, to_ref, role, confidence, provenance, valid_from,
                   valid_to, created_at, deleted_at",
        row.id,
        deleted_at,
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(super::not_updated)?;
    Ok(updated.into())
}
