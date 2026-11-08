//! `cases` model — CRUD over the stored `case_matcher::Case` payload.
//!
//! The full payload is stored verbatim in the `data` JSONB column; `pid`
//! and `title` are denormalised for lookup and listing. Every statement
//! that touches `cases` is here, including the record-integrity, erasure,
//! merge-lock and MAC re-signing writes the compliance modules and tasks
//! drive.

use axa::cursor_pagination::escape_like;
use axa::sqlx::Db;
use case_matcher::Case as MatchCase;
use chrono::{DateTime, FixedOffset, SubsecRound as _};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{ModelError, ModelResult, to_i64, updated};
use crate::compliance::record_integrity;

/// A persisted case row. Mirrors the `cases` table; the canonical payload
/// lives in `data` while `pid`/`title` are denormalised for lookup.
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
    /// Denormalised case title, for lookup and listing.
    pub title: String,
    /// The full `case_matcher::Case` payload (JSONB).
    pub data: serde_json::Value,
    /// Whether the row is active (cleared on soft-delete).
    pub active: bool,
    /// Soft-delete timestamp; `None` while active.
    pub deleted_at: Option<DateTime<FixedOffset>>,
    /// SHA-256 over the record's content and lifecycle state, for
    /// out-of-band tamper detection (`crate::compliance::record_integrity`).
    /// `None` on rows written before the column existed; never
    /// back-filled, because a back-fill would certify whatever the current
    /// content happens to be — the claim the hash exists to test.
    pub content_hash: Option<String>,
    /// SHA-3 digest over the same pre-image as `content_hash`.
    pub content_hash_sha3: Option<String>,
    /// HMAC over the same pre-image, as `"<key id>:<hex>"`. Unlike the
    /// digests, not recomputable from the database alone.
    pub content_mac: Option<String>,
}

impl Model {
    /// Deserialize the stored payload into a matcher `Case`.
    ///
    /// # Errors
    ///
    /// When the stored JSON cannot be parsed.
    pub fn to_case(&self) -> ModelResult<MatchCase> {
        serde_json::from_value(self.data.clone()).map_err(|e| ModelError::Any(e.into()))
    }

    /// Insert a new case, returning the created row.
    ///
    /// Takes a [`Db`], so the caller can pass either the pool (memory
    /// transport) or its own open transaction (outbox transport — so the
    /// row and its `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create(db: impl Into<Db<'_>>, case: &MatchCase) -> ModelResult<Self> {
        let data = serde_json::to_value(case).map_err(|e| ModelError::Any(e.into()))?;
        let pid = Uuid::new_v4();
        // Both digests from one call, so neither can be stamped without
        // the other (see `record_integrity::digests`). A new record is
        // live, so the digest binds `deleted_at` as `None`.
        let digests = record_integrity::digests(&record_integrity::RecordInput {
            pid,
            title: &case.title,
            data: &data,
            active: true,
            deleted_at_micros: None,
        });
        let mut conn = db.into().acquire().await?;
        // `id`, `created_at` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO cases
                   (pid, title, data, active, deleted_at,
                    content_hash, content_hash_sha3, content_mac)
               VALUES ($1, $2, $3, true, NULL, $4, $5, $6)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, title, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            pid,
            case.title,
            data,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Find an active case by its public id.
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
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
               WHERE pid = $1 AND deleted_at IS NULL
               LIMIT 1"#,
            uuid,
        )
        .fetch_optional(db)
        .await?;
        row.ok_or(ModelError::EntityNotFound)
    }

    /// Find an active case by its **agency-scoped case number** — the
    /// `(agency_id, case_number)` pair stored inside the JSONB payload.
    ///
    /// Used by the bulk-import stable-key resolution
    /// (`agents/share/bulk-import-export.md` §10.1, BLK-5): a case
    /// number is only unique *within* an agency, so both fields are
    /// required and neither is queried alone. When more than one active
    /// row happens to share the pair (a pre-existing data-quality issue
    /// this method does not attempt to resolve), the most recently
    /// inserted one is returned, so an import is at least deterministic.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_agency_case_number(
        db: impl Into<Db<'_>>,
        agency_id: &str,
        case_number: &str,
    ) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
               WHERE deleted_at IS NULL
                 AND data->>'agency_id' = $1 AND data->>'case_number' = $2
               ORDER BY id DESC
               LIMIT 1"#,
            agency_id,
            case_number,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
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
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
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

    /// List active cases (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        Self::list_paged(db, limit, 0).await
    }

    /// One page of active cases (most-recent first).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_paged(db: &PgPool, limit: u64, offset: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
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

    /// One page of active cases **oldest first** — the search reindex's
    /// replay order.
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
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
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
        let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM cases"#)
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
        let n =
            sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM cases WHERE deleted_at IS NULL"#)
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
        let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM cases WHERE pid = $1"#, pid)
            .fetch_one(db)
            .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Case-insensitive substring search on the denormalised `title`, over
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

    /// One page of title-search hits.
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
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
               WHERE deleted_at IS NULL AND title ILIKE $1
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

    /// The most recently updated `limit` cases, **including soft-deleted
    /// and erased rows**, for integrity verification.
    ///
    /// Deliberately not filtered to live rows: a retired or erased case
    /// still carries a digest, and an attacker editing a row nobody reads
    /// any more is exactly the case worth catching.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent_for_integrity(db: impl Into<Db<'_>>, limit: u64) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
               ORDER BY updated_at DESC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Recompute every recent record's content hash and report mismatches.
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
                      id, pid, title, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM cases
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
        let done = sqlx::query!("UPDATE cases SET content_mac = $2 WHERE id = $1", id, mac)
            .execute(db)
            .await?;
        updated(done.rows_affected())
    }

    /// Lock one row `FOR UPDATE` until the caller's transaction ends — the
    /// SEC-B5 merge guard. A missing row locks nothing and is not an
    /// error; the caller re-checks liveness with [`Self::is_active`].
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn lock_for_update(conn: &mut PgConnection, pid: Uuid) -> ModelResult<()> {
        sqlx::query_scalar!(
            "SELECT id FROM cases WHERE pid = $1 LIMIT 1 FOR UPDATE",
            pid
        )
        .fetch_optional(conn)
        .await?;
        Ok(())
    }

    /// Whether an active (not soft-deleted) row carries `pid`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn is_active(conn: &mut PgConnection, pid: Uuid) -> ModelResult<bool> {
        let hit = sqlx::query_scalar!(
            "SELECT id FROM cases WHERE pid = $1 AND deleted_at IS NULL LIMIT 1",
            pid
        )
        .fetch_optional(conn)
        .await?;
        Ok(hit.is_some())
    }

    /// Replace the payload of this case, returning the updated row.
    ///
    /// Takes a [`Db`], so the update can run on a transaction alongside
    /// the outbox insert (see [`Model::create`]).
    ///
    /// # Errors
    ///
    /// When serialization or the update fails, including when the row is
    /// gone (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn update_data(self, db: impl Into<Db<'_>>, case: &MatchCase) -> ModelResult<Self> {
        let data = serde_json::to_value(case).map_err(|e| ModelError::Any(e.into()))?;
        // The lifecycle fields are untouched by an update, so read them
        // off the loaded row to hash the row as it will land.
        let d = record_integrity::digests(&record_integrity::RecordInput {
            pid: self.pid,
            title: &case.title,
            data: &data,
            active: self.active,
            deleted_at_micros: self.deleted_at.map(|d| d.timestamp_micros()),
        });
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE cases
               SET title = $2, data = $3,
                   content_hash = $4, content_hash_sha3 = $5, content_mac = $6
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, title, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            self.id,
            case.title,
            data,
            d.sha256,
            d.sha3,
            d.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
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
        // A soft delete changes the lifecycle state the digest binds, so
        // the row rehashes — otherwise every deleted case would read as
        // tampered.
        let d = record_integrity::digests(&record_integrity::RecordInput {
            pid: self.pid,
            title: &self.title,
            data: &self.data,
            active: false,
            deleted_at_micros: Some(deleted_at.timestamp_micros()),
        });
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE cases
               SET active = false, deleted_at = $2,
                   content_hash = $3, content_hash_sha3 = $4, content_mac = $5
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, title, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            self.id,
            deleted_at,
            d.sha256,
            d.sha3,
            d.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// GDPR Art. 17: replace this row's title and payload with the
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
            "UPDATE cases
             SET title = $2, data = $3, active = false, deleted_at = $4,
                 content_hash = $5, content_hash_sha3 = $6, content_mac = $7
             WHERE id = $1",
            self.id,
            tombstone.title,
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

/// What an erased row is left holding: the tombstone title and payload,
/// the soft-delete stamp, and the digests over exactly those values.
#[derive(Debug, Clone, Copy)]
pub struct Tombstone<'a> {
    /// The tombstone title.
    pub title: &'a str,
    /// The tombstone payload.
    pub data: &'a serde_json::Value,
    /// The soft-delete stamp, truncated to microseconds.
    pub deleted_at: DateTime<FixedOffset>,
    /// The digests over the tombstoned record.
    pub digests: &'a record_integrity::Digests,
}

#[cfg(test)]
mod tests {
    use super::escape_like;

    /// Pins the `LIKE` escaping this model's title search relies on
    /// (`axa::cursor_pagination::escape_like`): literal text is unchanged, `%` and `_`
    /// are escaped so they match literally, and the backslash is escaped
    /// first so a user backslash cannot re-enable a wildcard.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("housing"), "housing");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }
}
