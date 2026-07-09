//! `edges` model — the projection helpers over the derived graph
//! (spec §6 FR-4/5/6/7). All writes here are event-driven; there is no
//! world-facing edge write path.

use chrono::{DateTime, FixedOffset};
use loco_rs::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, Condition, ConnectionTrait};
use uuid::Uuid;

use entity_ref::{EdgeKind, EntityRef};

use crate::events::LinkedEvent;
use crate::graph::{self, EdgeStatus, EdgeView};
use crate::models::entity_presence;

pub use super::_entities::edges::{self, ActiveModel, Column, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for ActiveModel {}

/// Neighbour-lookup direction for [`Model::neighbors`] (spec §6 FR-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Edges where `ref` is the `from` endpoint.
    Out,
    /// Edges where `ref` is the `to` endpoint.
    In,
    /// Edges where `ref` is either endpoint.
    Both,
}

impl Direction {
    /// Parse the `direction` query token, defaulting to `Both` when
    /// absent/empty; `None` for an unrecognised value (⇒ `400`).
    #[must_use]
    pub fn parse(token: Option<&str>) -> Option<Self> {
        match token {
            None | Some("" | "both") => Some(Direction::Both),
            Some("out") => Some(Direction::Out),
            Some("in") => Some(Direction::In),
            Some(_) => None,
        }
    }
}

impl Model {
    /// Project a stored row into the pure [`EdgeView`] used by
    /// [`crate::graph::single_view`]. Rows whose refs or kind fail to
    /// parse are dropped (`None`) — the read-model should never hold
    /// such a row, but the read path stays total if one appears.
    #[must_use]
    pub fn to_view(&self) -> Option<EdgeView> {
        Some(EdgeView {
            from_ref: self.from_ref.parse().ok()?,
            to_ref: self.to_ref.parse().ok()?,
            kind: EdgeKind::from_token(&self.kind)?,
        })
    }

    /// Apply a `linked` event: canonicalise the endpoints (spec §6 FR-6),
    /// derive the integrity status from endpoint presence (spec §6 FR-9),
    /// and upsert one row keyed by `edge_id` (idempotent — spec §6 FR-4).
    ///
    /// # Errors
    ///
    /// When a presence lookup or the upsert fails.
    pub async fn apply_linked<C: ConnectionTrait>(
        db: &C,
        ev: &LinkedEvent,
        source_event_id: Uuid,
        observed_at: DateTime<FixedOffset>,
    ) -> ModelResult<Model> {
        let (from, to, directed) = graph::canonical(ev.from_ref, ev.to_ref, ev.edge_kind);
        let from_alive = entity_presence::Model::alive(db, &from).await?;
        let to_alive = entity_presence::Model::alive(db, &to).await?;
        let status = graph::edge_status(from_alive, to_alive);

        let am = edges::ActiveModel {
            edge_id: ActiveValue::set(ev.edge_id),
            from_ref: ActiveValue::set(from.to_string()),
            to_ref: ActiveValue::set(to.to_string()),
            kind: ActiveValue::set(ev.edge_kind.as_str().to_string()),
            directed: ActiveValue::set(directed),
            role: ActiveValue::set(ev.role.clone()),
            confidence: ActiveValue::set(ev.confidence),
            provenance: ActiveValue::set(ev.provenance.clone()),
            valid_from: ActiveValue::set(ev.valid_from),
            valid_to: ActiveValue::set(ev.valid_to),
            status: ActiveValue::set(status.as_str().to_string()),
            observed_at: ActiveValue::set(observed_at),
            source_event_id: ActiveValue::set(source_event_id),
        };
        Entity::insert(am)
            .on_conflict(
                OnConflict::column(Column::EdgeId)
                    .update_columns([
                        Column::FromRef,
                        Column::ToRef,
                        Column::Kind,
                        Column::Directed,
                        Column::Role,
                        Column::Confidence,
                        Column::Provenance,
                        Column::ValidFrom,
                        Column::ValidTo,
                        Column::Status,
                        Column::ObservedAt,
                        Column::SourceEventId,
                    ])
                    .to_owned(),
            )
            .exec(db)
            .await?;

        let row = Entity::find_by_id(ev.edge_id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        Ok(row)
    }

    /// Apply an `unlinked` event: remove the edge identified by `edge_id`
    /// (spec §6 FR-5). Returns the number of rows deleted (0 when the
    /// edge was already absent — still idempotent).
    ///
    /// # Errors
    ///
    /// When the delete fails.
    pub async fn apply_unlinked<C: ConnectionTrait>(db: &C, edge_id: Uuid) -> ModelResult<u64> {
        let res = Entity::delete_by_id(edge_id).exec(db).await?;
        Ok(res.rows_affected)
    }

    /// Recompute the integrity status of every edge incident to `r` from
    /// current endpoint presence, and persist any change (spec §6
    /// FR-10 — a target deleted after the edge formed flips it to
    /// `dangling`). Called after a presence flip.
    ///
    /// # Errors
    ///
    /// When a query or update fails.
    pub async fn recompute_status_for<C: ConnectionTrait>(
        db: &C,
        r: &EntityRef,
    ) -> ModelResult<()> {
        let urn = r.to_string();
        let rows = Entity::find()
            .filter(
                Condition::any()
                    .add(Column::FromRef.eq(urn.as_str()))
                    .add(Column::ToRef.eq(urn.as_str())),
            )
            .all(db)
            .await?;
        for row in rows {
            let from_alive = match row.from_ref.parse::<EntityRef>() {
                Ok(ref e) => entity_presence::Model::alive(db, e).await?,
                Err(_) => None,
            };
            let to_alive = match row.to_ref.parse::<EntityRef>() {
                Ok(ref e) => entity_presence::Model::alive(db, e).await?,
                Err(_) => None,
            };
            let status = graph::edge_status(from_alive, to_alive)
                .as_str()
                .to_string();
            if status != row.status {
                let mut am: edges::ActiveModel = row.into();
                am.status = ActiveValue::set(status);
                am.update(db).await?;
            }
        }
        Ok(())
    }

    /// Edges incident to `r`, in the requested [`Direction`], optionally
    /// filtered by `kind` (spec §6 FR-7/13). Both-direction lookups use
    /// the `edges_from` and `edges_to` indexes.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn neighbors<C: ConnectionTrait>(
        db: &C,
        r: &EntityRef,
        kind: Option<EdgeKind>,
        direction: Direction,
    ) -> ModelResult<Vec<Model>> {
        let urn = r.to_string();
        let endpoint = match direction {
            Direction::Out => Condition::all().add(Column::FromRef.eq(urn.as_str())),
            Direction::In => Condition::all().add(Column::ToRef.eq(urn.as_str())),
            Direction::Both => Condition::any()
                .add(Column::FromRef.eq(urn.as_str()))
                .add(Column::ToRef.eq(urn.as_str())),
        };
        let mut cond = Condition::all().add(endpoint);
        if let Some(k) = kind {
            cond = cond.add(Column::Kind.eq(k.as_str()));
        }
        let rows = Entity::find().filter(cond).all(db).await?;
        Ok(rows)
    }

    /// Filtered edge list for `GET /api/edges` (spec §6 FR-14). Any
    /// subset of `from` / `to` / `kind` / `status` may be applied.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn filter<C: ConnectionTrait>(
        db: &C,
        from: Option<&EntityRef>,
        to: Option<&EntityRef>,
        kind: Option<EdgeKind>,
        status: Option<EdgeStatus>,
    ) -> ModelResult<Vec<Model>> {
        let mut cond = Condition::all();
        if let Some(f) = from {
            cond = cond.add(Column::FromRef.eq(f.to_string()));
        }
        if let Some(t) = to {
            cond = cond.add(Column::ToRef.eq(t.to_string()));
        }
        if let Some(k) = kind {
            cond = cond.add(Column::Kind.eq(k.as_str()));
        }
        if let Some(s) = status {
            cond = cond.add(Column::Status.eq(s.as_str()));
        }
        let rows = Entity::find().filter(cond).all(db).await?;
        Ok(rows)
    }

    /// Load every edge in the read-model, as pure [`EdgeView`]s, for the
    /// `single-view` walk (spec §6 FR-15). v1 scans the table; a
    /// same-identity-scoped load is a future optimisation.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_views<C: ConnectionTrait>(db: &C) -> ModelResult<Vec<EdgeView>> {
        let rows = Entity::find().all(db).await?;
        Ok(rows.iter().filter_map(Model::to_view).collect())
    }
}
