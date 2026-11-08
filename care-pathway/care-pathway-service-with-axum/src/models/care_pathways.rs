//! `care_pathways` model — CRUD over the stored
//! `care_pathway_matcher::CarePathway` payload.
//!
//! The full payload is stored verbatim in the `data` JSONB column; `pid`
//! and `name` are denormalised for lookup and listing. Every statement
//! that touches `care_pathways` is here, including the record-integrity,
//! erasure, bulk stable-key and MAC re-signing statements the compliance
//! modules, the bulk pipeline and the tasks drive.

use axa::cursor_pagination::escape_like;
use axa::sqlx::Db;
use care_pathway_matcher::CarePathway as MatchPathway;
use chrono::{DateTime, FixedOffset, SubsecRound as _};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult, to_i64, updated};
use crate::compliance::record_integrity;

/// A persisted care-pathway row. Mirrors the `care_pathways` table; the
/// canonical payload lives in `data` while `pid`/`name` are denormalised
/// for lookup.
///
/// Timestamps are `DateTime<FixedOffset>` (always `+00:00` from
/// Postgres), the type the `SeaORM` entity used, so anything that
/// serializes a row keeps its exact wire form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTime<FixedOffset>,
    /// Row last-update timestamp: the column default, never bumped by a
    /// statement here, exactly as under `SeaORM`.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal auto-increment primary key.
    pub id: i64,
    /// Public, externally exposed identifier (UUID v4).
    pub pid: Uuid,
    /// Denormalised pathway name, for lookup and listing.
    pub name: String,
    /// The full `care_pathway_matcher::CarePathway` payload (JSONB).
    pub data: serde_json::Value,
    /// Whether the row is active (cleared on soft-delete).
    pub active: bool,
    /// Soft-delete timestamp; `None` while active.
    pub deleted_at: Option<DateTime<FixedOffset>>,
    /// SHA-256 over the record's content and lifecycle state, for
    /// out-of-band tamper detection (`crate::compliance::record_integrity`).
    /// `None` on rows written before the column existed; never
    /// back-filled.
    pub content_hash: Option<String>,
    /// SHA-3 digest over the same pre-image as `content_hash`.
    pub content_hash_sha3: Option<String>,
    /// HMAC over the same pre-image, as `"<key id>:<hex>"`. Unlike the
    /// digests, not recomputable from the database alone.
    pub content_mac: Option<String>,
}

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
    /// Takes a [`Db`], so the caller can pass either the pool (memory
    /// transport) or its own open transaction (outbox transport — so the
    /// row and its `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create(db: impl Into<Db<'_>>, pathway: &MatchPathway) -> ModelResult<Self> {
        let data = serde_json::to_value(pathway).map_err(|e| ModelError::Any(e.into()))?;
        let pid = Uuid::new_v4();
        // Both digests from one call, so neither can be stamped without
        // the other (see `record_integrity::digests`).
        let digests = record_integrity::digests(&record_integrity::RecordInput {
            pid,
            name: &pathway.name,
            data: &data,
            active: true,
            deleted_at_micros: None,
        });
        let mut conn = db.into().acquire().await?;
        // `id`, `created_at` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO care_pathways
                   (pid, name, data, active, deleted_at,
                    content_hash, content_hash_sha3, content_mac)
               VALUES ($1, $2, $3, true, NULL, $4, $5, $6)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            pid,
            pathway.name,
            data,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Find an active care pathway by its public id.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when no active row has that id;
    /// [`ModelError::Any`] when `pid` is not a UUID; a database error
    /// when the query fails.
    pub async fn find_by_pid(db: &PgPool, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE pid = $1 AND deleted_at IS NULL
               LIMIT 1"#,
            uuid,
        )
        .fetch_optional(db)
        .await?;
        row.ok_or(ModelError::EntityNotFound)
    }

    /// Fetch the active rows for a list of public ids, **preserving the
    /// order of `pids`**.
    ///
    /// The order matters: the caller is a search or blocking query whose
    /// hits are already ranked by relevance, and re-sorting them by row
    /// id would throw that ranking away. Ids that do not resolve (unknown
    /// or soft-deleted — a stale index entry) are simply absent, which is
    /// what keeps a drifted index from resurrecting a deleted record.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_pids(db: &PgPool, pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        if pids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE pid = ANY($1) AND deleted_at IS NULL"#,
            pids,
        )
        .fetch_all(db)
        .await?;
        // Re-order to match `pids` (the SQL result order is unspecified).
        // Linear in `pids` × `rows`, both bounded by the caller's result
        // limit.
        let mut ordered = Vec::with_capacity(rows.len());
        for pid in pids {
            if let Some(row) = rows.iter().find(|r| r.pid == *pid) {
                ordered.push(row.clone());
            }
        }
        Ok(ordered)
    }

    /// The first active care pathway whose `data->'identifiers'` array
    /// contains `needle` (a one-element JSON array of
    /// `{"scheme": …, "value": …}`), via the JSONB containment operator —
    /// the bulk import's identifier stable key.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_identifier_needle(
        db: &PgPool,
        needle: &serde_json::Value,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE deleted_at IS NULL AND data -> 'identifiers' @> $1::jsonb
               LIMIT 1"#,
            needle,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// The first active care pathway owning a `(provider_id,
    /// pathway_code)` pair, via JSONB scalar equality — the bulk import's
    /// provider-scoped stable key.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_provider_and_code(
        db: &PgPool,
        provider_id: &str,
        pathway_code: &str,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE deleted_at IS NULL
                 AND data ->> 'provider_id' = $1
                 AND data ->> 'pathway_code' = $2
               LIMIT 1"#,
            provider_id,
            pathway_code,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// List active care pathways (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        Self::list_paged(db, limit, 0).await
    }

    /// One page of active care pathways (most-recent first).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_paged(db: &PgPool, limit: u64, offset: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE deleted_at IS NULL
               ORDER BY id DESC
               LIMIT $1 OFFSET $2"#,
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// One page of active care pathways **oldest first** — the search
    /// reindex's replay order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_active_oldest_first(
        db: &PgPool,
        limit: u64,
        offset: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE deleted_at IS NULL
               ORDER BY id ASC
               LIMIT $1 OFFSET $2"#,
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// How many rows the table holds, soft-deleted ones included.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_all(db: &PgPool) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM care_pathways"#)
            .fetch_one(db)
            .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// How many rows carry `pid`, soft-deleted ones included.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_by_pid(db: &PgPool, pid: Uuid) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM care_pathways WHERE pid = $1"#,
            pid
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// How many active (not soft-deleted) rows the table holds.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_active(db: &PgPool) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM care_pathways WHERE deleted_at IS NULL"#
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Case-insensitive substring search on the denormalised `name`, over
    /// active rows (Postgres `ILIKE '%q%'`). `q` is matched literally —
    /// `%` and `_` in the query are escaped so they are not treated as
    /// wildcards.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(db: &PgPool, q: &str, limit: u64) -> ModelResult<Vec<Self>> {
        Self::search_paged(db, q, limit, 0).await
    }

    /// One page of name-search hits, `limit` rows from `offset`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search_paged(
        db: &PgPool,
        q: &str,
        limit: u64,
        offset: u64,
    ) -> ModelResult<Vec<Self>> {
        let pattern = format!("%{}%", escape_like(q));
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE deleted_at IS NULL AND name ILIKE $1
               ORDER BY id DESC
               LIMIT $2 OFFSET $3"#,
            pattern,
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// The newest `limit` rows **including soft-deleted and erased ones**,
    /// for row-level integrity verification.
    ///
    /// Unlike [`Model::list`], this deliberately does not filter on
    /// `deleted_at`: a retired row is exactly where an out-of-band edit is
    /// least likely to be noticed, so it is the row most worth checking.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent_for_integrity(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               ORDER BY id DESC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Verify the newest `limit` records by recomputing each content hash.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn verify_records(
        db: &PgPool,
        limit: u64,
    ) -> ModelResult<record_integrity::RecordIntegrityReport> {
        Ok(record_integrity::verify(
            &Self::recent_for_integrity(db, limit).await?,
        ))
    }

    /// The rows carrying a MAC, oldest first, capped at `limit` — the
    /// `integrity_resign` task's walk.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn with_mac_oldest_first(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM care_pathways
               WHERE content_mac IS NOT NULL
               ORDER BY id ASC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Replace one row's MAC, and nothing else: the `integrity_resign`
    /// task's write, after the existing MAC verified.
    ///
    /// # Errors
    ///
    /// When the update fails, including when the row is gone
    /// (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn set_content_mac(db: &PgPool, id: i64, mac: Option<&str>) -> ModelResult<()> {
        let done = sqlx::query!(
            "UPDATE care_pathways SET content_mac = $2 WHERE id = $1",
            id,
            mac
        )
        .execute(db)
        .await?;
        updated(done.rows_affected())
    }

    /// Replace the payload of this care pathway, returning the updated
    /// row.
    ///
    /// Takes a [`Db`], so the update can run on a transaction alongside
    /// the outbox insert (see [`Model::create`]).
    ///
    /// # Errors
    ///
    /// When serialization or the update fails, including when the row is
    /// gone (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn update_data(
        self,
        db: impl Into<Db<'_>>,
        pathway: &MatchPathway,
    ) -> ModelResult<Self> {
        let data = serde_json::to_value(pathway).map_err(|e| ModelError::Any(e.into()))?;
        // The lifecycle fields are untouched by an update, so read them
        // off the loaded row to hash the row as it will land.
        let d = record_integrity::digests(&record_integrity::RecordInput {
            pid: self.pid,
            name: &pathway.name,
            data: &data,
            active: self.active,
            deleted_at_micros: self.deleted_at.map(|d| d.timestamp_micros()),
        });
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE care_pathways
               SET name = $2, data = $3,
                   content_hash = $4, content_hash_sha3 = $5, content_mac = $6
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            self.id,
            pathway.name,
            data,
            d.sha256,
            d.sha3,
            d.mac,
        )
        .fetch_optional(&mut *conn)
        .await?;
        row.ok_or(ModelError::Sqlx(sqlx::Error::RowNotFound))
    }

    /// Soft-delete: mark inactive and stamp `deleted_at`.
    ///
    /// Takes a [`Db`], so the soft-delete can run on a transaction
    /// alongside the outbox insert (see [`Model::create`]).
    ///
    /// # Errors
    ///
    /// When the update fails, including when the row is gone
    /// (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn soft_delete(self, db: impl Into<Db<'_>>) -> ModelResult<Self> {
        // Truncated to microseconds so the value hashed here is the value
        // Postgres returns (see `compliance::record_integrity`).
        let deleted_at: DateTime<FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
        let d = record_integrity::digests(&record_integrity::RecordInput {
            pid: self.pid,
            name: &self.name,
            data: &self.data,
            active: false,
            deleted_at_micros: Some(deleted_at.timestamp_micros()),
        });
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE care_pathways
               SET active = false, deleted_at = $2,
                   content_hash = $3, content_hash_sha3 = $4, content_mac = $5
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            self.id,
            deleted_at,
            d.sha256,
            d.sha3,
            d.mac,
        )
        .fetch_optional(&mut *conn)
        .await?;
        row.ok_or(ModelError::Sqlx(sqlx::Error::RowNotFound))
    }

    /// GDPR Art. 17: replace this row's name and payload with the
    /// tombstone, retire it, and stamp the digests over the tombstoned
    /// record ([`crate::compliance::erasure`] computes them).
    ///
    /// # Errors
    ///
    /// When the update fails, including when the row is gone
    /// (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn erase_payload(
        self,
        db: impl Into<Db<'_>>,
        tombstone: &Tombstone<'_>,
    ) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        let done = sqlx::query!(
            "UPDATE care_pathways
             SET name = $2, data = $3, active = false, deleted_at = $4,
                 content_hash = $5, content_hash_sha3 = $6, content_mac = $7
             WHERE id = $1",
            self.id,
            tombstone.name,
            tombstone.data,
            tombstone.deleted_at,
            tombstone.digests.sha256,
            tombstone.digests.sha3,
            tombstone.digests.mac,
        )
        .execute(&mut *conn)
        .await?;
        updated(done.rows_affected())
    }
}

/// What an erased row is left holding: the tombstone name and payload,
/// the soft-delete stamp, and the digests over exactly those values.
#[derive(Debug, Clone, Copy)]
pub struct Tombstone<'a> {
    /// The tombstone name.
    pub name: &'a str,
    /// The tombstone payload.
    pub data: &'a serde_json::Value,
    /// The soft-delete stamp, truncated to microseconds.
    pub deleted_at: DateTime<FixedOffset>,
    /// The digests over the tombstoned record.
    pub digests: &'a record_integrity::Digests,
}

/// Tests for the model's pure helpers (DB-free).
#[cfg(test)]
mod tests {
    use axa::cursor_pagination::escape_like;

    /// The `escape_like` this module binds into its `ILIKE` patterns
    /// neutralises `%`, `_`, and `\` so a user query matches
    /// literally and cannot inject `ILIKE` wildcards.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("stroke"), "stroke");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }
}
