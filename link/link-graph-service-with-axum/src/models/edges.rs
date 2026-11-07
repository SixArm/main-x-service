//! `edges` model — the projection helpers over the derived graph
//! (spec §6 FR-4/5/6/7). All writes here are event-driven; there is no
//! world-facing edge write path.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use entity_ref::{EdgeKind, EntityRef, EntityType};

use super::{ModelError, ModelResult};
use crate::events::LinkedEvent;
use crate::graph::{self, EdgeStatus, EdgeView};
use crate::models::entity_presence;

/// A persisted graph edge (spec §10.1). Refs are stored as their
/// `EntityRef` URN strings; a symmetric edge is stored once with the
/// canonical (smaller) ref as `from_ref`. Serialized as-is by the graph
/// endpoints, so the field order and types are the wire shape.
///
/// No `Eq`/`Hash` derive because `confidence` is an `Option<f64>`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// The edge id — equals the source `linked` event's `edge_id`.
    pub edge_id: Uuid,
    /// Canonical "from" endpoint (`EntityRef` URN).
    pub from_ref: String,
    /// "to" endpoint (`EntityRef` URN).
    pub to_ref: String,
    /// Edge kind wire token (closed registry).
    pub kind: String,
    /// `false` for symmetric kinds.
    pub directed: bool,
    /// Optional role (e.g. job title for `employed_by`).
    pub role: Option<String>,
    /// Confidence in `[0,1]`; `1.0` operator-asserted.
    pub confidence: Option<f64>,
    /// Provenance wire token (`operator` | `import` | `matcher_suggested`).
    pub provenance: String,
    /// Affiliation start (nullable).
    pub valid_from: Option<NaiveDate>,
    /// Affiliation end (nullable).
    pub valid_to: Option<NaiveDate>,
    /// Integrity-lifecycle status wire token.
    pub status: String,
    /// When the `linked` event was consumed.
    pub observed_at: DateTime<FixedOffset>,
    /// The envelope `event_id` that produced this row (dedup provenance).
    pub source_event_id: Uuid,
}

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

    /// Whether a lookup in this direction matches on `from_ref`.
    fn matches_from(self) -> bool {
        matches!(self, Direction::Out | Direction::Both)
    }

    /// Whether a lookup in this direction matches on `to_ref`.
    fn matches_to(self) -> bool {
        matches!(self, Direction::In | Direction::Both)
    }
}

/// An `UPDATE` that matched no row is an error, as the `SeaORM` era's
/// "record not updated" was: the caller read the row a moment before.
fn require_updated(done: &sqlx::postgres::PgQueryResult) -> ModelResult<()> {
    if done.rows_affected() == 0 {
        return Err(ModelError::Any("edge not updated: no such row".into()));
    }
    Ok(())
}

/// Every edge with `urn` as either endpoint, on `conn`.
async fn incident_to(conn: &mut PgConnection, urn: &str) -> ModelResult<Vec<Model>> {
    let rows = sqlx::query_as!(
        Model,
        r#"SELECT edge_id, from_ref, to_ref, kind, directed, role, confidence,
                  provenance, valid_from, valid_to, status,
                  observed_at AS "observed_at: _", source_event_id
           FROM edges
           WHERE from_ref = $1 OR to_ref = $1"#,
        urn,
    )
    .fetch_all(conn)
    .await?;
    Ok(rows)
}

/// Remove one edge by id, on `conn`. Returns the rows deleted.
async fn delete_by_id(conn: &mut PgConnection, edge_id: Uuid) -> ModelResult<u64> {
    let done = sqlx::query!("DELETE FROM edges WHERE edge_id = $1", edge_id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
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

    /// The edge with `edge_id`, if present.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find(db: impl Into<Db<'_>>, edge_id: Uuid) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT edge_id, from_ref, to_ref, kind, directed, role, confidence,
                      provenance, valid_from, valid_to, status,
                      observed_at AS "observed_at: _", source_event_id
               FROM edges
               WHERE edge_id = $1"#,
            edge_id,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Apply a `linked` event: canonicalise the endpoints (spec §6 FR-6),
    /// derive the integrity status from endpoint presence (spec §6 FR-9),
    /// and upsert one row keyed by `edge_id` (idempotent — spec §6 FR-4).
    ///
    /// # Errors
    ///
    /// When a presence lookup or the upsert fails.
    pub async fn apply_linked(
        db: impl Into<Db<'_>>,
        ev: &LinkedEvent,
        source_event_id: Uuid,
        observed_at: DateTime<FixedOffset>,
    ) -> ModelResult<Model> {
        let mut conn = db.into().acquire().await?;
        let (from, to, directed) = graph::canonical(ev.from_ref, ev.to_ref, ev.edge_kind);
        let from_alive = entity_presence::Model::alive(&mut *conn, &from).await?;
        let to_alive = entity_presence::Model::alive(&mut *conn, &to).await?;
        let status = graph::edge_status(from_alive, to_alive);

        sqlx::query!(
            "INSERT INTO edges
                 (edge_id, from_ref, to_ref, kind, directed, role, confidence,
                  provenance, valid_from, valid_to, status, observed_at, source_event_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (edge_id) DO UPDATE
                 SET from_ref = excluded.from_ref,
                     to_ref = excluded.to_ref,
                     kind = excluded.kind,
                     directed = excluded.directed,
                     role = excluded.role,
                     confidence = excluded.confidence,
                     provenance = excluded.provenance,
                     valid_from = excluded.valid_from,
                     valid_to = excluded.valid_to,
                     status = excluded.status,
                     observed_at = excluded.observed_at,
                     source_event_id = excluded.source_event_id",
            ev.edge_id,
            from.to_string(),
            to.to_string(),
            ev.edge_kind.as_str(),
            directed,
            ev.role.as_deref(),
            ev.confidence,
            ev.provenance,
            ev.valid_from,
            ev.valid_to,
            status.as_str(),
            observed_at,
            source_event_id,
        )
        .execute(&mut *conn)
        .await?;

        Self::find(&mut *conn, ev.edge_id)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }

    /// Apply an `unlinked` event: remove the edge identified by `edge_id`
    /// (spec §6 FR-5). Returns the number of rows deleted (0 when the
    /// edge was already absent — still idempotent).
    ///
    /// # Errors
    ///
    /// When the delete fails.
    pub async fn apply_unlinked(db: impl Into<Db<'_>>, edge_id: Uuid) -> ModelResult<u64> {
        let mut conn = db.into().acquire().await?;
        delete_by_id(&mut conn, edge_id).await
    }

    /// Repoint every edge referencing a merged-away record onto the merge
    /// survivor (spec §5.3 / §6 FR-12 — the central fix-up "one aggregator
    /// helps" with). For each edge touching `merged_from`: swap that
    /// endpoint for `survivor`, re-canonicalise ([`graph::repoint`]), then
    /// - **drop** it if it collapsed to a self-loop, or
    /// - **drop** it if the repointed (from, to, kind) already exists on
    ///   another edge (de-duplication), or
    /// - otherwise **update** its endpoints, `directed`, and status.
    ///
    /// Returns the number of edges changed (updated or dropped).
    ///
    /// # Errors
    ///
    /// When a query, update, or delete fails.
    pub async fn repoint_all(
        db: impl Into<Db<'_>>,
        merged_from: &EntityRef,
        survivor: &EntityRef,
    ) -> ModelResult<u64> {
        let mut conn = db.into().acquire().await?;
        let rows = incident_to(&mut conn, &merged_from.to_string()).await?;

        let mut changed = 0u64;
        for row in rows {
            let (Ok(from), Ok(to)) = (
                row.from_ref.parse::<EntityRef>(),
                row.to_ref.parse::<EntityRef>(),
            ) else {
                continue;
            };
            let Some(kind) = EdgeKind::from_token(&row.kind) else {
                continue;
            };

            match graph::repoint(from, to, kind, *merged_from, *survivor) {
                // Self-loop after repoint ⇒ drop the edge.
                None => {
                    delete_by_id(&mut conn, row.edge_id).await?;
                    changed += 1;
                }
                Some((new_from, new_to, directed)) => {
                    let from_urn = new_from.to_string();
                    let to_urn = new_to.to_string();
                    // De-dup: a different edge already holds this canonical
                    // (from, to, kind) ⇒ drop this one rather than collide.
                    let duplicate = sqlx::query_scalar!(
                        "SELECT edge_id FROM edges
                         WHERE from_ref = $1 AND to_ref = $2 AND kind = $3 AND edge_id <> $4
                         LIMIT 1",
                        from_urn,
                        to_urn,
                        kind.as_str(),
                        row.edge_id,
                    )
                    .fetch_optional(&mut *conn)
                    .await?;
                    if duplicate.is_some() {
                        delete_by_id(&mut conn, row.edge_id).await?;
                        changed += 1;
                        continue;
                    }
                    // Repoint in place, recomputing status against the new
                    // endpoints' presence.
                    let from_alive = entity_presence::Model::alive(&mut *conn, &new_from).await?;
                    let to_alive = entity_presence::Model::alive(&mut *conn, &new_to).await?;
                    let status = graph::edge_status(from_alive, to_alive);
                    let done = sqlx::query!(
                        "UPDATE edges SET from_ref = $2, to_ref = $3, directed = $4, status = $5
                         WHERE edge_id = $1",
                        row.edge_id,
                        from_urn,
                        to_urn,
                        directed,
                        status.as_str(),
                    )
                    .execute(&mut *conn)
                    .await?;
                    require_updated(&done)?;
                    changed += 1;
                }
            }
        }
        Ok(changed)
    }

    /// Recompute the integrity status of every edge incident to `r` from
    /// current endpoint presence, and persist any change (spec §6
    /// FR-10 — a target deleted after the edge formed flips it to
    /// `dangling`). Called after a presence flip.
    ///
    /// # Errors
    ///
    /// When a query or update fails.
    pub async fn recompute_status_for(db: impl Into<Db<'_>>, r: &EntityRef) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        let rows = incident_to(&mut conn, &r.to_string()).await?;
        for row in rows {
            let from_alive = match row.from_ref.parse::<EntityRef>() {
                Ok(ref e) => entity_presence::Model::alive(&mut *conn, e).await?,
                Err(_) => None,
            };
            let to_alive = match row.to_ref.parse::<EntityRef>() {
                Ok(ref e) => entity_presence::Model::alive(&mut *conn, e).await?,
                Err(_) => None,
            };
            let status = graph::edge_status(from_alive, to_alive).as_str();
            if status != row.status {
                let done = sqlx::query!(
                    "UPDATE edges SET status = $2 WHERE edge_id = $1",
                    row.edge_id,
                    status,
                )
                .execute(&mut *conn)
                .await?;
                require_updated(&done)?;
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
    pub async fn neighbors(
        db: impl Into<Db<'_>>,
        r: &EntityRef,
        kind: Option<EdgeKind>,
        direction: Direction,
    ) -> ModelResult<Vec<Model>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT edge_id, from_ref, to_ref, kind, directed, role, confidence,
                      provenance, valid_from, valid_to, status,
                      observed_at AS "observed_at: _", source_event_id
               FROM edges
               WHERE (($2 AND from_ref = $1) OR ($3 AND to_ref = $1))
                 AND ($4::text IS NULL OR kind = $4)"#,
            r.to_string(),
            direction.matches_from(),
            direction.matches_to(),
            kind.map(EdgeKind::as_str),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Filtered edge list for `GET /api/edges` (spec §6 FR-14). Any
    /// subset of `from` / `to` / `kind` / `status` may be applied.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn filter(
        db: impl Into<Db<'_>>,
        from: Option<&EntityRef>,
        to: Option<&EntityRef>,
        kind: Option<EdgeKind>,
        status: Option<EdgeStatus>,
    ) -> ModelResult<Vec<Model>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT edge_id, from_ref, to_ref, kind, directed, role, confidence,
                      provenance, valid_from, valid_to, status,
                      observed_at AS "observed_at: _", source_event_id
               FROM edges
               WHERE ($1::text IS NULL OR from_ref = $1)
                 AND ($2::text IS NULL OR to_ref = $2)
                 AND ($3::text IS NULL OR kind = $3)
                 AND ($4::text IS NULL OR status = $4)"#,
            from.map(ToString::to_string),
            to.map(ToString::to_string),
            kind.map(EdgeKind::as_str),
            status.map(EdgeStatus::as_str),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Load every edge in the read-model, as pure [`EdgeView`]s, for the
    /// `single-view` walk (spec §6 FR-15). v1 scans the table; a
    /// same-identity-scoped load is a future optimisation.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_views(db: impl Into<Db<'_>>) -> ModelResult<Vec<EdgeView>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT edge_id, from_ref, to_ref, kind, directed, role, confidence,
                      provenance, valid_from, valid_to, status,
                      observed_at AS "observed_at: _", source_event_id
               FROM edges"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows.iter().filter_map(Model::to_view).collect())
    }

    /// The `edge_id` of every stored edge, for reconciliation (spec T-20 /
    /// design §8). v1 scans the table; a paged/replay diff is a future
    /// optimisation.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_edge_ids(
        db: impl Into<Db<'_>>,
    ) -> ModelResult<std::collections::HashSet<Uuid>> {
        let mut conn = db.into().acquire().await?;
        let ids = sqlx::query_scalar!("SELECT edge_id FROM edges")
            .fetch_all(&mut *conn)
            .await?;
        Ok(ids.into_iter().collect())
    }

    /// The `edge_id` of every stored edge **originating from** `entity`
    /// (its canonical `from_ref` is a `<entity>:…` URN), for **entity-
    /// scoped** reconciliation (SEC-B1 / design §8).
    ///
    /// Reconciliation diffs one service's authoritative edges against the
    /// read-model; it must only ever consider the edges that service is
    /// authoritative for. Diffing against the *global* set makes every
    /// other entity's edges look "extra" and deletes them (a per-entity
    /// pass wipes the graph). Because a directed edge's owner is its
    /// from-side, and a symmetric `same_identity` edge canonicalises its
    /// `from_ref` to the lexicographically-smaller URN (`person` < `worker`,
    /// so the person-asserted edge canonicalises to `person:…`), scoping by
    /// the canonical `from_ref` entity correctly selects exactly the source
    /// service's edges for the two live sources (`case` `subject_of`,
    /// `person` `same_identity`). v1 scans the table like [`all_edge_ids`],
    /// and filters in Rust ([`from_ref_is_entity`]) rather than with SQL
    /// `LIKE`, whose `_` would be a wildcard in `care_pathway`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn edge_ids_from_entity(
        db: impl Into<Db<'_>>,
        entity: EntityType,
    ) -> ModelResult<std::collections::HashSet<Uuid>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query!("SELECT edge_id, from_ref FROM edges")
            .fetch_all(&mut *conn)
            .await?;
        Ok(rows
            .into_iter()
            .filter(|r| from_ref_is_entity(&r.from_ref, entity))
            .map(|r| r.edge_id)
            .collect())
    }

    /// Count edges in each integrity [`EdgeStatus`], for the
    /// `link_graph_edges{status}` metric gauge (spec §9 / T-21).
    ///
    /// # Errors
    ///
    /// When a count query fails.
    pub async fn count_by_status(db: impl Into<Db<'_>>) -> ModelResult<Vec<(EdgeStatus, u64)>> {
        let mut conn = db.into().acquire().await?;
        let mut out = Vec::with_capacity(3);
        for status in [
            EdgeStatus::Unverified,
            EdgeStatus::Verified,
            EdgeStatus::Dangling,
        ] {
            let n = sqlx::query_scalar!(
                r#"SELECT count(*) AS "n!" FROM edges WHERE status = $1"#,
                status.as_str(),
            )
            .fetch_one(&mut *conn)
            .await?;
            out.push((status, u64::try_from(n).unwrap_or(0)));
        }
        Ok(out)
    }
}

/// True when a stored `from_ref` URN (`<type>:<uuid>`) is owned by
/// `entity` — an exact `"<entity>:"` prefix match. The trailing colon
/// disambiguates sibling tokens (`course:` vs `courseinstance:`), and an
/// exact string compare (not SQL `LIKE`) avoids `_` in `care_pathway`
/// acting as a wildcard. Used to scope reconciliation to one entity (SEC-B1).
fn from_ref_is_entity(from_ref: &str, entity: EntityType) -> bool {
    let mut prefix = String::with_capacity(entity.as_str().len() + 1);
    prefix.push_str(entity.as_str());
    prefix.push(':');
    from_ref.starts_with(&prefix)
}

#[cfg(test)]
mod tests {
    use super::from_ref_is_entity;
    use entity_ref::EntityType;

    #[test]
    fn from_ref_scoping_matches_only_the_owning_entity() {
        assert!(from_ref_is_entity("case:0c4f-…", EntityType::Case));
        assert!(from_ref_is_entity("person:abcd-…", EntityType::Person));
        // A different entity's URN is not selected — this is the SEC-B1
        // guarantee: a case reconcile never scoops up person edges.
        assert!(!from_ref_is_entity("person:abcd-…", EntityType::Case));
        assert!(!from_ref_is_entity("case:0c4f-…", EntityType::Person));
    }

    #[test]
    fn from_ref_scoping_disambiguates_sibling_tokens() {
        // `course:` must not match a `courseinstance:` URN (prefix trap)…
        assert!(!from_ref_is_entity("courseinstance:x", EntityType::Course));
        assert!(from_ref_is_entity(
            "courseinstance:x",
            EntityType::CourseInstance
        ));
        // …and the `_` in `care_pathway` is a literal, not a LIKE wildcard.
        assert!(from_ref_is_entity(
            "care_pathway:x",
            EntityType::CarePathway
        ));
        assert!(!from_ref_is_entity(
            "careXpathway:x",
            EntityType::CarePathway
        ));
    }
}
