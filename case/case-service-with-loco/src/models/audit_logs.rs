//! `audit_logs` model — record and query the CRUD audit trail, and
//! maintain its **tamper-evident hash chain**
//! ([`crate::compliance::audit_chain`]).
//!
//! Every row written through [`Model::record_with_context`] binds its own
//! content and the current chain head, so the trail can prove it was not
//! rewritten (HIPAA §164.312(c)). The trail stays **append-only**: the one
//! statement that ever modifies an existing row is
//! [`Model::redact_for_entity`], the GDPR Art. 17 erasure path, and it
//! destroys content while preserving the `hash` / `prev_hash` linkage so
//! verification still succeeds.

use chrono::SubsecRound as _;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryOrder, QuerySelect, Statement};
use uuid::Uuid;

use crate::compliance::audit_chain::{self, ChainInput, ChainReport};

/// Re-export the generated `audit_logs` entity so callers use
/// `models::audit_logs::…` rather than reaching into `_entities`.
pub use super::_entities::audit_logs::{self, ActiveModel, Entity, Model};

/// Postgres advisory-lock key serialising chain appends.
///
/// The chain head must be read and the successor inserted without another
/// writer interleaving, or two rows would claim the same predecessor and
/// verification would report a fork. `pg_advisory_xact_lock` is held to
/// the end of the enclosing transaction, so under the **`outbox`**
/// transport — where the audit row shares the entity mutation's
/// transaction — appends are fully serialised.
///
/// Under the `memory` transport the audit write is a best-effort side
/// channel on a pooled connection, where each statement is its own
/// implicit transaction and the lock is released immediately. Concurrent
/// writers can then fork the chain, which verification reports as a
/// `linkage` break. **A compliance deployment should run
/// `CASE_EVENT_TRANSPORT=outbox`**; this is stated in the entity
/// spec's honest limits rather than papered over.
const CHAIN_LOCK_KEY: i64 = 0x6D78_695F_6175_6469; // "mxi_audi"

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for super::_entities::audit_logs::ActiveModel {}

impl Model {
    /// Record one audit entry. `actor` is the caller's `sub` (user `pid`)
    /// when a verified token was presented, else `None`.
    ///
    /// Generic over [`ConnectionTrait`] so it runs either on the pooled
    /// `&DatabaseConnection` (the best-effort side-channel path, `memory`
    /// transport) **or** on a `&DatabaseTransaction` — under the `outbox`
    /// transport the audit row is written in the *same* transaction as the
    /// entity mutation and its `event_outbox` row, so the three can never
    /// disagree (`agents/share/event-bus.md` §3).
    ///
    /// This is the mutation-path entry point: no request context and never
    /// a disclosure. Read paths use
    /// [`crate::compliance::disclosure::record_access`].
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record<C: ConnectionTrait>(
        db: &C,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        Self::record_with_context(db, entity_pid, action, actor, snapshot, None, false).await
    }

    /// Record one audit entry with its request context and disclosure
    /// flag, chained onto the current head.
    ///
    /// The row's `created_at` is stamped here rather than by the database
    /// default, and **truncated to microseconds**, so the value hashed on
    /// write is byte-identical to the value Postgres returns on read (see
    /// [`crate::compliance::audit_chain`]).
    ///
    /// # Errors
    ///
    /// When reading the chain head or the insert fails.
    pub async fn record_with_context<C: ConnectionTrait>(
        db: &C,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
        context: Option<serde_json::Value>,
        disclosure: bool,
    ) -> ModelResult<Self> {
        // Serialise the read-head/append pair (see `CHAIN_LOCK_KEY`).
        if db.get_database_backend() == DatabaseBackend::Postgres {
            db.execute(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT pg_advisory_xact_lock({CHAIN_LOCK_KEY})"),
            ))
            .await?;
        }
        let (prev_hash, prev_hash_sha3) = Self::heads(db).await?;
        let created_at: chrono::DateTime<chrono::FixedOffset> =
            chrono::Utc::now().trunc_subsecs(6).into();
        // One input, re-pointed at each algorithm's own predecessor:
        // the digests cover byte-identical content while the chains stay
        // independent.
        let mut input = ChainInput {
            prev_hash: prev_hash.as_deref(),
            entity_pid,
            action,
            actor,
            created_at_micros: created_at.timestamp_micros(),
            snapshot: snapshot.as_ref(),
            context: context.as_ref(),
            disclosure,
        };
        let hash = audit_chain::row_hash(&input);
        input.prev_hash = prev_hash_sha3.as_deref();
        let hash_sha3 = audit_chain::row_hash_sha3(&input);
        let entry = audit_logs::ActiveModel {
            created_at: ActiveValue::set(created_at),
            updated_at: ActiveValue::set(created_at),
            entity_pid: ActiveValue::set(entity_pid),
            action: ActiveValue::set(action.to_string()),
            actor: ActiveValue::set(actor.map(ToString::to_string)),
            snapshot: ActiveValue::set(snapshot),
            prev_hash: ActiveValue::set(prev_hash),
            hash: ActiveValue::set(Some(hash)),
            prev_hash_sha3: ActiveValue::set(prev_hash_sha3),
            hash_sha3: ActiveValue::set(Some(hash_sha3)),
            context: ActiveValue::set(context),
            disclosure: ActiveValue::set(disclosure),
            redacted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(entry)
    }

    /// Both chain heads — `(SHA-256, BLAKE3)` — from the same row.
    ///
    /// Read together so an append binds each algorithm's *own*
    /// predecessor. Binding the SHA-256 head into the BLAKE3 digest would
    /// make the second chain's linkage rest on SHA-256's collision
    /// resistance, which is the dependency keeping two algorithms is
    /// meant to avoid.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn heads<C: ConnectionTrait>(
        db: &C,
    ) -> ModelResult<(Option<String>, Option<String>)> {
        let last = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .one(db)
            .await?;
        Ok(match last {
            Some(row) => (row.hash, row.hash_sha3),
            None => (None, None),
        })
    }

    /// The current chain head: the most recent row's `hash`, or `None`
    /// when the trail is empty (or its last row predates the chain, in
    /// which case the next row starts a fresh run — see
    /// [`audit_chain::verify`]).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn head<C: ConnectionTrait>(db: &C) -> ModelResult<Option<String>> {
        let last = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .one(db)
            .await?;
        Ok(last.and_then(|row| row.hash))
    }

    /// Most-recent audit entries, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Audit entries for one case, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: &DatabaseConnection, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::EntityPid.eq(entity_pid))
            .order_by_desc(audit_logs::Column::Id)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Disclosure entries for one case, most-recent first — the
    /// HIPAA §164.528 accounting of disclosures.
    ///
    /// Only rows the access context classified as an outward disclosure
    /// are returned (see [`crate::compliance::disclosure::AccessContext::is_disclosure`]);
    /// ordinary internal accesses are excluded, which is the whole point
    /// of the distinction.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn disclosures_for_entity(
        db: &DatabaseConnection,
        entity_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::EntityPid.eq(entity_pid))
            .filter(audit_logs::Column::Disclosure.eq(true))
            .order_by_desc(audit_logs::Column::Id)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// The newest `limit` rows in **ascending `id` order** — the shape
    /// [`audit_chain::verify`] expects.
    ///
    /// Verifying a suffix of the trail is sound: the run's first row has no
    /// predecessor to check against, and every row after it is fully
    /// checked.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn tail(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let mut rows = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    /// Verify the newest `limit` rows of the chain.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn verify_chain(db: &DatabaseConnection, limit: u64) -> ModelResult<ChainReport> {
        Ok(audit_chain::verify(&Self::tail(db, limit).await?))
    }

    /// GDPR Art. 17: destroy the audit **content** held about one care
    /// pathway while preserving the chain.
    ///
    /// Sets `snapshot = NULL` and stamps `redacted_at` on every not-yet
    /// redacted row for `entity_pid`. The row's `hash` and `prev_hash` are
    /// deliberately **left untouched**, so [`audit_chain::verify`] still
    /// checks linkage across the redacted row and the trail as a whole
    /// keeps verifying — the erasure is real, and the accountability
    /// record that *something happened, when, and by whom* survives.
    ///
    /// `actor` and `action` are retained on purpose: they are the
    /// controller's own accountability record under the Art. 17(3)(b)
    /// legal-obligation carve-out, and they identify the operator rather
    /// than the erased subject.
    ///
    /// Returns the number of rows redacted.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn redact_for_entity<C: ConnectionTrait>(
        db: &C,
        entity_pid: Uuid,
    ) -> ModelResult<u64> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
        let result = audit_logs::Entity::update_many()
            .col_expr(
                audit_logs::Column::Snapshot,
                sea_orm::sea_query::Expr::value(sea_orm::Value::Json(None)),
            )
            .col_expr(
                audit_logs::Column::RedactedAt,
                sea_orm::sea_query::Expr::value(now),
            )
            .filter(audit_logs::Column::EntityPid.eq(entity_pid))
            .filter(audit_logs::Column::RedactedAt.is_null())
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }
}
