//! `entity_presence` model — the existence oracle (spec §10.2 / §6 FR-8).

use loco_rs::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ConnectionTrait};

use entity_ref::EntityRef;

pub use super::_entities::entity_presence::{self, ActiveModel, Column, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Upsert the presence of `r`: `created` ⇒ `alive = true`,
    /// `deleted` ⇒ `alive = false`. Idempotent on the ref primary key.
    ///
    /// Generic over [`ConnectionTrait`] so it runs on a pooled
    /// connection or inside a transaction.
    ///
    /// # Errors
    ///
    /// When the upsert fails.
    pub async fn mark<C: ConnectionTrait>(
        db: &C,
        r: &EntityRef,
        alive: bool,
        seq: i64,
    ) -> ModelResult<()> {
        let am = entity_presence::ActiveModel {
            entity_ref: ActiveValue::set(r.to_string()),
            alive: ActiveValue::set(alive),
            last_seq: ActiveValue::set(seq),
        };
        Entity::insert(am)
            .on_conflict(
                OnConflict::column(Column::EntityRef)
                    .update_columns([Column::Alive, Column::LastSeq])
                    .to_owned(),
            )
            .exec(db)
            .await?;
        Ok(())
    }

    /// The last-known liveness of `r`: `Some(true)` alive, `Some(false)`
    /// known-deleted, `None` never observed.
    ///
    /// # Errors
    ///
    /// When the lookup fails.
    pub async fn alive<C: ConnectionTrait>(db: &C, r: &EntityRef) -> ModelResult<Option<bool>> {
        let row = Entity::find_by_id(r.to_string()).one(db).await?;
        Ok(row.map(|m| m.alive))
    }
}
