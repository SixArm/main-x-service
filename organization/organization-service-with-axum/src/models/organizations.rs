//! `organizations` model — CRUD over the stored
//! `organization_matcher::Organization` payload.
//!
//! The full payload is stored verbatim in the `data` JSONB column; `pid`
//! and `name` are denormalised for lookup and listing.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use organization_matcher::Organization as MatchOrg;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// A persisted organization row. Mirrors the `organizations` table; the
/// canonical payload lives in `data` while `pid`/`name` are denormalised
/// for lookup.
///
/// Timestamps are `DateTime<FixedOffset>` (always `+00:00` from
/// Postgres), the type the `SeaORM` entity used, so anything that
/// serializes a row keeps its exact wire form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row insert time (UTC), the column default.
    pub created_at: DateTime<FixedOffset>,
    /// Set to the insert time by the column default. No statement here
    /// bumps it on update, exactly as under `SeaORM`.
    pub updated_at: DateTime<FixedOffset>,
    /// Auto-increment surrogate primary key (internal; never exposed on
    /// the wire — used for stable `ORDER BY id` listing).
    pub id: i64,
    /// Public, externally-stable id (UUID v4). The only identifier the
    /// REST API references; unique per the migration.
    pub pid: Uuid,
    /// Denormalised `Organization.name`, kept in its own column so list
    /// avoids touching the JSON payload.
    pub name: String,
    /// The full `organization_matcher::Organization` payload, stored
    /// verbatim as JSONB; deserialized back via [`Model::to_org`].
    pub data: serde_json::Value,
    /// Soft-delete liveness flag: `true` while active, `false` once
    /// soft-deleted (paired with `deleted_at`).
    pub active: bool,
    /// Soft-delete timestamp: `None` while active, set when soft-deleted.
    /// Active-row queries filter on `deleted_at IS NULL`.
    pub deleted_at: Option<DateTime<FixedOffset>>,
    /// SHA-256 (FIPS 180-4) over this row's integrity pre-image.
    ///
    /// `None` on a row written before the column existed — reported as
    /// unhashed, never as a mismatch, and never back-filled.
    pub content_hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub content_hash_sha3: Option<String>,
    /// HMAC-SHA256 over the same pre-image, as `"<scheme>.<key id>:<hex>"`.
    ///
    /// The only one of the three an adversary holding just this database
    /// cannot recompute: the digests are unkeyed and their pre-image
    /// format is published.
    pub content_mac: Option<String>,
}

/// A `LIMIT` / `OFFSET` bound: Postgres takes `BIGINT`, the callers count
/// in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Read-side and write-side helpers on an `organizations` row.
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
    /// Takes a [`Db`], so the caller passes either the pool (memory
    /// transport) or its own open transaction (outbox transport — so the
    /// row and its `event_outbox` row share one commit boundary).
    ///
    /// # Errors
    ///
    /// When serialization or the insert fails.
    pub async fn create(db: impl Into<Db<'_>>, org: &MatchOrg) -> ModelResult<Self> {
        let data = serde_json::to_value(org).map_err(|e| ModelError::Any(e.into()))?;
        // Mint the public id here (not a DB default) so it is known
        // before the insert, returned to the caller, and bound into the
        // digests below.
        let pid = Uuid::new_v4();
        // All three digests from one call, so none can be stamped
        // without the others (see `record_integrity::digests`). Computed
        // inline rather than stamped afterwards, so one statement writes
        // the row and its integrity values together — there is no window
        // in which a row exists unhashed. A new record is live, so the
        // digests bind `deleted_at` as `None`.
        let digests = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid,
                name: &org.name,
                data: &data,
                active: true,
                deleted_at_micros: None,
            },
        );
        let mut conn = db.into().acquire().await?;
        // `id`, `created_at` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO organizations
                   (pid, name, data, active, deleted_at,
                    content_hash, content_hash_sha3, content_mac)
               VALUES ($1, $2, $3, true, NULL, $4, $5, $6)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            pid,
            org.name,
            data,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Find an active organization by its public id.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when no active row has that id;
    /// [`ModelError::Any`] when `pid` is not a UUID; a database error
    /// when the query fails.
    pub async fn find_by_pid(db: &PgPool, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        // Soft-deleted rows are invisible to lookups.
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM organizations
               WHERE pid = $1 AND deleted_at IS NULL"#,
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
               FROM organizations
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

    /// The active organization owning an identifier, if any: a JSONB
    /// containment match of `needle` (a one-element array of
    /// `{"scheme": …, "value": …}`) against `data->'identifiers'`.
    /// Returns the first match.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_identifiers_containing(
        db: &PgPool,
        needle: &serde_json::Value,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM organizations
               WHERE deleted_at IS NULL AND data -> 'identifiers' @> $1::jsonb
               LIMIT 1"#,
            needle,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// List active organizations (most-recent first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        Self::list_paged(db, limit, 0).await
    }

    /// One page of active organizations (most-recent first).
    ///
    /// The ordering is by descending `id` — insertion order — which is
    /// stable enough to page through as long as rows are only appended.
    /// A row inserted *during* a page walk shifts everything after it by
    /// one. A signed cursor carries an offset underneath, so the list can
    /// repeat or skip one row then, as [`agents/share/restful.md`] says;
    /// `id` is unique, so it is never more than one.
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
               FROM organizations
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

    /// One page of active organizations **oldest first** — the search
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
               FROM organizations
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

    /// The most recently modified rows, **including soft-deleted ones**
    /// (integrity verification covers every row), newest `updated_at`
    /// first, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recently_updated(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, data, active,
                      deleted_at AS "deleted_at: _",
                      content_hash, content_hash_sha3, content_mac
               FROM organizations
               ORDER BY updated_at DESC
               LIMIT $1"#,
            to_i64(limit),
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
        let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM organizations"#)
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
            r#"SELECT count(*) AS "n!" FROM organizations WHERE deleted_at IS NULL"#
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Replace the payload of this organization, returning the updated
    /// row.
    ///
    /// Takes a [`Db`], so the update can run on a transaction alongside
    /// the outbox insert (see [`Model::create`]).
    ///
    /// # Errors
    ///
    /// When serialization or the update fails.
    pub async fn update_data(self, db: impl Into<Db<'_>>, org: &MatchOrg) -> ModelResult<Self> {
        let data = serde_json::to_value(org).map_err(|e| ModelError::Any(e.into()))?;
        // Re-digest over the *new* content. Leaving the old digests would
        // report every legitimate edit as tampering, which is as damaging
        // as missing a real one.
        let d = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid: self.pid,
                name: &org.name,
                data: &data,
                active: self.active,
                deleted_at_micros: deleted_at_micros(self.deleted_at.as_ref()),
            },
        );
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE organizations
               SET name = $2, data = $3,
                   content_hash = $4, content_hash_sha3 = $5, content_mac = $6
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            self.id,
            org.name,
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
    /// When the update fails.
    pub async fn soft_delete(self, db: impl Into<Db<'_>>) -> ModelResult<Self> {
        // This is a soft-delete stamp, not domain time.
        let stamp: DateTime<FixedOffset> = chrono::Utc::now().into();
        // `active` and `deleted_at` are both in the pre-image, so a
        // soft delete changes the digest. Not re-digesting here would
        // make every soft-deleted row look tampered with.
        let d = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                pid: self.pid,
                name: &self.name,
                data: &self.data,
                active: false,
                deleted_at_micros: deleted_at_micros(Some(&stamp)),
            },
        );
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE organizations
               SET active = false, deleted_at = $2,
                   content_hash = $3, content_hash_sha3 = $4, content_mac = $5
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, data, active,
                         deleted_at AS "deleted_at: _",
                         content_hash, content_hash_sha3, content_mac"#,
            self.id,
            stamp,
            d.sha256,
            d.sha3,
            d.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }
}

/// A soft-delete stamp as epoch microseconds, the form the digest
/// pre-image binds.
///
/// Microseconds because that is what Postgres stores; binding a finer
/// precision than the column keeps would make a row's digest fail to
/// reproduce after a round-trip.
fn deleted_at_micros(stamp: Option<&DateTime<FixedOffset>>) -> Option<i64> {
    stamp.map(DateTime::timestamp_micros)
}

// The `ILIKE '%q%'` name search (and its `escape_like` wildcard guard,
// SEC-G4) lived here until search moved to Tantivy (spec §13). Both were
// removed rather than left dormant: this crate now issues no `LIKE`
// query at all, so an escaper with no caller would only invite a future
// caller to assume it was still wired in. The sibling care-pathway /
// case services keep theirs, because they still search with `ILIKE`.
//
// This module's remaining behaviour is all database access, exercised by
// the Postgres-gated suite (`tests/requests/organizations.rs`); there is
// no pure logic left here to unit-test.
