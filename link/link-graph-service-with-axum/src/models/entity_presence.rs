//! `entity_presence` model — the existence oracle (spec §10.2 / §6 FR-8):
//! one row per known `EntityRef`, `alive` toggled by the source service's
//! `created` / `deleted` events.

use axa::sqlx::Db;
use serde::{Deserialize, Serialize};

use entity_ref::EntityRef;

use super::ModelResult;

/// A persisted presence row. The primary key column is `ref` (a SQL /
/// Rust-keyword clash), mapped from the `entity_ref` field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// The `EntityRef` URN (column `ref`).
    pub entity_ref: String,
    /// `true` when last seen `created`; `false` when `deleted`.
    pub alive: bool,
    /// The last envelope `seq` observed for this ref (ordering).
    pub last_seq: i64,
}

impl Model {
    /// Upsert the presence of `r`: `created` ⇒ `alive = true`,
    /// `deleted` ⇒ `alive = false`. Idempotent on the ref primary key.
    ///
    /// Takes a [`Db`], so it runs on the pool or on a connection the
    /// caller holds.
    ///
    /// # Errors
    ///
    /// When the upsert fails.
    pub async fn mark(
        db: impl Into<Db<'_>>,
        r: &EntityRef,
        alive: bool,
        seq: i64,
    ) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        sqlx::query!(
            "INSERT INTO entity_presence (ref, alive, last_seq)
             VALUES ($1, $2, $3)
             ON CONFLICT (ref) DO UPDATE
                 SET alive = excluded.alive,
                     last_seq = excluded.last_seq",
            r.to_string(),
            alive,
            seq,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// The presence row for `r`, if one has been observed.
    ///
    /// # Errors
    ///
    /// When the lookup fails.
    pub async fn find(db: impl Into<Db<'_>>, r: &str) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT ref AS entity_ref, alive, last_seq
               FROM entity_presence
               WHERE ref = $1"#,
            r,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The last-known liveness of `r`: `Some(true)` alive, `Some(false)`
    /// known-deleted, `None` never observed.
    ///
    /// # Errors
    ///
    /// When the lookup fails.
    pub async fn alive(db: impl Into<Db<'_>>, r: &EntityRef) -> ModelResult<Option<bool>> {
        let row = Self::find(db, &r.to_string()).await?;
        Ok(row.map(|m| m.alive))
    }
}
