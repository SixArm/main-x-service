//! `entity_links` model — the cross-service **write side**
//! (`agents/share/cross-service-linking.md` §4.1).
//!
//! [`Model::upsert`] is the idempotent create/update on the
//! `(from_pid, kind, to_ref, valid_from)` unique key, [`Model::list_active`]
//! lists a case's live outbound edges, [`Model::soft_delete`] withdraws
//! one, and [`Model::withdraw_all`] withdraws every edge of one case (the
//! GDPR erasure sweep). Validation of the edge (that `to_ref` parses and
//! the `kind`/endpoint pair is permitted) lives in `controllers/links.rs`
//! and is DB-free; this layer is pure persistence.
//!
//! One persisted row is one **outbound** edge this service originates
//! from a case (v1: the `subject_of` case → person edge). Separate from
//! the within-entity `relationships` on the `Case` payload — these are
//! never a matcher signal (the partition rule, §7).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One persisted outbound cross-service edge.
///
/// `Eq` is intentionally not derived: `confidence` is an `f64` (not `Eq`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTime<FixedOffset>,
    /// Row last-update timestamp: the column default, never bumped by a
    /// statement here, exactly as under `SeaORM`.
    pub updated_at: DateTime<FixedOffset>,
    /// The edge id (also the `linked`/`unlinked` event's `edge_id`).
    pub id: Uuid,
    /// This service's originating case record (`pid`).
    pub from_pid: Uuid,
    /// The edge kind token (§9 registry), e.g. `subject_of`.
    pub kind: String,
    /// The far record's `EntityRef` URN, e.g. `person:<uuid>`.
    pub to_ref: String,
    /// Optional role label (unused by `subject_of`).
    pub role: Option<String>,
    /// Optional confidence: `1.0` operator-asserted, `<1` suggested.
    pub confidence: Option<f64>,
    /// How the edge arose: `operator` | `import` | `matcher_suggested`.
    pub provenance: String,
    /// Affiliation start (nullable — `subject_of` is temporal).
    pub valid_from: Option<NaiveDate>,
    /// Affiliation end ("former subject of" once past).
    pub valid_to: Option<NaiveDate>,
    /// Soft-delete timestamp (a withdrawn edge); `None` while live.
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The column values for a new (or re-asserted) outbound edge — the
/// validated, ready-to-persist shape the controller hands to
/// [`Model::upsert`]. Kept separate from the row type so the controller
/// does not construct row internals directly.
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
    pub valid_from: Option<NaiveDate>,
    /// Optional validity end.
    pub valid_to: Option<NaiveDate>,
}

impl Model {
    /// Idempotently create or update an outbound edge on the
    /// `(from_pid, kind, to_ref, valid_from)` unique key: a re-assertion of
    /// the same edge updates its mutable columns (role / confidence /
    /// provenance / `valid_to`) and **revives** a previously soft-deleted
    /// edge (clears `deleted_at`) rather than inserting a duplicate, so the
    /// edge id is stable across re-asserts.
    ///
    /// Takes a [`Db`], so the caller can pass either the pool (memory
    /// transport) or its own open transaction (outbox transport — the row
    /// and its `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When the insert/upsert fails.
    pub async fn upsert(db: impl Into<Db<'_>>, edge: &NewEdge) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO entity_links
                   (id, from_pid, kind, to_ref, role, confidence, provenance,
                    valid_from, valid_to, deleted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)
               ON CONFLICT (from_pid, kind, to_ref, valid_from) DO UPDATE SET
                   role = EXCLUDED.role,
                   confidence = EXCLUDED.confidence,
                   provenance = EXCLUDED.provenance,
                   valid_to = EXCLUDED.valid_to,
                   deleted_at = EXCLUDED.deleted_at
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, from_pid, kind, to_ref, role, confidence, provenance,
                         valid_from, valid_to, deleted_at AS "deleted_at: _""#,
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
        Ok(row)
    }

    /// List a case's **active** outbound edges (not soft-deleted),
    /// newest first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_active(db: &PgPool, from_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, from_pid, kind, to_ref, role, confidence, provenance,
                      valid_from, valid_to, deleted_at AS "deleted_at: _"
               FROM entity_links
               WHERE from_pid = $1 AND deleted_at IS NULL
               ORDER BY created_at DESC"#,
            from_pid,
        )
        .fetch_all(db)
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
    pub async fn find_active(db: &PgPool, from_pid: Uuid, id: Uuid) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, from_pid, kind, to_ref, role, confidence, provenance,
                      valid_from, valid_to, deleted_at AS "deleted_at: _"
               FROM entity_links
               WHERE id = $1 AND from_pid = $2 AND deleted_at IS NULL
               LIMIT 1"#,
            id,
            from_pid,
        )
        .fetch_optional(db)
        .await?;
        row.ok_or(ModelError::EntityNotFound)
    }

    /// Every **active** outbound edge across all cases, oldest first — the
    /// authoritative set the link-graph aggregator reconciles against
    /// (`GET /api/cases/links`, design §8). `since` (on `created_at`)
    /// bounds an incremental pull; `None` is a full replay.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_all_active(
        db: impl Into<Db<'_>>,
        since: Option<DateTime<FixedOffset>>,
    ) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        // One static statement: an absent bound is a NULL `$1`, which
        // matches every row.
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, from_pid, kind, to_ref, role, confidence, provenance,
                      valid_from, valid_to, deleted_at AS "deleted_at: _"
               FROM entity_links
               WHERE deleted_at IS NULL
                 AND ($1::timestamptz IS NULL OR created_at >= $1)
               ORDER BY created_at ASC"#,
            since,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Soft-delete (withdraw) this edge: stamp `deleted_at`.
    ///
    /// Takes a [`Db`], so it can run on the outbox transaction alongside
    /// the `unlinked` event insert.
    ///
    /// # Errors
    ///
    /// When the update fails, including when the row is gone
    /// (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn soft_delete(self, db: impl Into<Db<'_>>) -> ModelResult<Self> {
        let stamp: DateTime<FixedOffset> = chrono::Utc::now().into();
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE entity_links SET deleted_at = $2
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, from_pid, kind, to_ref, role, confidence, provenance,
                         valid_from, valid_to, deleted_at AS "deleted_at: _""#,
            self.id,
            stamp,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Soft-delete every live outbound edge of one case, returning how
    /// many were withdrawn — the GDPR erasure sweep
    /// ([`crate::compliance::erasure`]).
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn withdraw_all(db: impl Into<Db<'_>>, from_pid: Uuid) -> ModelResult<u64> {
        let mut conn = db.into().acquire().await?;
        let done = sqlx::query!(
            "UPDATE entity_links SET deleted_at = NOW()
             WHERE from_pid = $1 AND deleted_at IS NULL",
            from_pid,
        )
        .execute(&mut *conn)
        .await?;
        Ok(done.rows_affected())
    }
}
