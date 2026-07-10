//! Pure, DB-free projection logic for the cross-service link graph.
//!
//! Everything here is a total function over value types (no I/O, no
//! clock, no database), so it is exhaustively unit-testable and shared
//! by both the event-apply seam ([`crate::events`]) and the read
//! controllers ([`crate::controllers::graph`]).
//!
//! - [`canonical`] — symmetric-kind canonicalisation (spec §6 FR-6).
//! - [`edge_status`] — the integrity lifecycle verdict (spec §6 FR-9).
//! - [`single_view`] — the golden-record walk (spec §6 FR-15).

use std::collections::HashSet;

use entity_ref::{EdgeKind, EntityRef};

/// The integrity-lifecycle status of an edge, derived from the presence
/// of its two endpoints (spec §5.3 / §6 FR-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeStatus {
    /// At least one endpoint has not yet been observed.
    Unverified,
    /// Both endpoints are present and alive.
    Verified,
    /// An endpoint is known-deleted (surfaced, never silently broken).
    Dangling,
}

impl EdgeStatus {
    /// The lowercase wire token for this status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            EdgeStatus::Unverified => "unverified",
            EdgeStatus::Verified => "verified",
            EdgeStatus::Dangling => "dangling",
        }
    }

    /// Parse a wire token into an [`EdgeStatus`], or `None` if unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "unverified" => Some(EdgeStatus::Unverified),
            "verified" => Some(EdgeStatus::Verified),
            "dangling" => Some(EdgeStatus::Dangling),
            _ => None,
        }
    }
}

/// How a stored edge came to exist (spec §5.3). Carried on the wire and
/// persisted verbatim; the aggregator does not interpret it in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// Operator-asserted (confidence 1.0).
    Operator,
    /// Loaded via a bulk import job.
    Import,
    /// Proposed by a matcher (confidence < 1.0; destined for review).
    MatcherSuggested,
}

impl Provenance {
    /// The lowercase wire token for this provenance.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Provenance::Operator => "operator",
            Provenance::Import => "import",
            Provenance::MatcherSuggested => "matcher_suggested",
        }
    }

    /// Parse a wire token into a [`Provenance`], or `None` if unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "operator" => Some(Provenance::Operator),
            "import" => Some(Provenance::Import),
            "matcher_suggested" => Some(Provenance::MatcherSuggested),
            _ => None,
        }
    }
}

/// A minimal, pure view of an edge — just its two endpoints and kind —
/// used by [`single_view`]. Both fields are `Copy` value types, so this
/// is cheap to pass around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeView {
    /// The (already-canonical) "from" endpoint.
    pub from_ref: EntityRef,
    /// The "to" endpoint.
    pub to_ref: EntityRef,
    /// The edge kind.
    pub kind: EdgeKind,
}

/// The result of a golden-record walk (spec §6 FR-15): the unified
/// identity set plus the affiliation edges incident to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleView {
    /// Every ref unified with the start ref via `same_identity`
    /// (always includes the start), sorted by URN for determinism.
    pub identity_refs: Vec<EntityRef>,
    /// The non-`same_identity` edges touching any ref in the identity
    /// set (`employed_by` / `works_at` / `member_of` / `subject_of`).
    pub affiliations: Vec<EdgeView>,
}

/// Canonicalise an edge's endpoints for storage (spec §6 FR-6).
///
/// For a **symmetric** kind (`same_identity`) the pair is ordered by URN
/// string — the lexicographically smaller ref becomes `from` — and the
/// edge is stored `directed = false`, so the pair is stored once
/// regardless of which side emitted the event. Every other kind is
/// returned unchanged with `directed = true`.
#[must_use]
pub fn canonical(from: EntityRef, to: EntityRef, kind: EdgeKind) -> (EntityRef, EntityRef, bool) {
    if kind.is_symmetric() {
        if from.to_string() <= to.to_string() {
            (from, to, false)
        } else {
            (to, from, false)
        }
    } else {
        (from, to, true)
    }
}

/// Repoint one edge from a merged-away record onto the merge survivor
/// (spec §5.3 / §6 FR-12). Any endpoint equal to `merged_from` is
/// replaced with `survivor`, then the edge is re-canonicalised (a
/// symmetric edge may need re-ordering after the swap).
///
/// Returns `None` when the edge collapses to a **self-loop** — both
/// endpoints become the survivor — which the caller drops, since a
/// record is never linked to itself (e.g. the merged duplicate's own
/// `same_identity` edge to the survivor). Otherwise returns the new
/// canonical `(from, to, directed)`.
#[must_use]
pub fn repoint(
    from: EntityRef,
    to: EntityRef,
    kind: EdgeKind,
    merged_from: EntityRef,
    survivor: EntityRef,
) -> Option<(EntityRef, EntityRef, bool)> {
    let new_from = if from == merged_from { survivor } else { from };
    let new_to = if to == merged_from { survivor } else { to };
    if new_from == new_to {
        return None;
    }
    Some(canonical(new_from, new_to, kind))
}

/// Derive an edge's [`EdgeStatus`] from the presence of its two
/// endpoints (spec §6 FR-9/10).
///
/// `None` means "not yet observed"; `Some(true)` alive; `Some(false)`
/// known-deleted. Either endpoint known-deleted ⇒ `Dangling`; both alive
/// ⇒ `Verified`; otherwise ⇒ `Unverified`.
#[must_use]
pub fn edge_status(from_alive: Option<bool>, to_alive: Option<bool>) -> EdgeStatus {
    match (from_alive, to_alive) {
        (Some(false), _) | (_, Some(false)) => EdgeStatus::Dangling,
        (Some(true), Some(true)) => EdgeStatus::Verified,
        _ => EdgeStatus::Unverified,
    }
}

/// The golden-record walk (spec §6 FR-15): from `start`, follow
/// `same_identity` edges (in both directions) to unify person ↔ worker,
/// then collect every affiliation edge incident to the unified identity.
///
/// Pure: the caller supplies the full edge slice; there is no I/O. The
/// returned `identity_refs` and `affiliations` are sorted for a
/// deterministic response.
#[must_use]
pub fn single_view(start: EntityRef, edges: &[EdgeView]) -> SingleView {
    let mut identity: HashSet<EntityRef> = HashSet::new();
    identity.insert(start);

    // Fixed-point expansion over same_identity edges (both directions).
    let mut changed = true;
    while changed {
        changed = false;
        for e in edges {
            if e.kind != EdgeKind::SameIdentity {
                continue;
            }
            let from_in = identity.contains(&e.from_ref);
            let to_in = identity.contains(&e.to_ref);
            if from_in && !to_in {
                identity.insert(e.to_ref);
                changed = true;
            } else if to_in && !from_in {
                identity.insert(e.from_ref);
                changed = true;
            }
        }
    }

    let mut affiliations: Vec<EdgeView> = edges
        .iter()
        .copied()
        .filter(|e| {
            e.kind != EdgeKind::SameIdentity
                && (identity.contains(&e.from_ref) || identity.contains(&e.to_ref))
        })
        .collect();
    affiliations.sort_by(|a, b| {
        (
            a.from_ref.to_string(),
            a.to_ref.to_string(),
            a.kind.as_str(),
        )
            .cmp(&(
                b.from_ref.to_string(),
                b.to_ref.to_string(),
                b.kind.as_str(),
            ))
    });

    let mut identity_refs: Vec<EntityRef> = identity.into_iter().collect();
    identity_refs.sort_by_key(EntityRef::to_string);

    SingleView {
        identity_refs,
        affiliations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity_ref::EntityType;
    use uuid::Uuid;

    fn r(t: EntityType, n: u128) -> EntityRef {
        EntityRef::new(t, Uuid::from_u128(n))
    }

    #[test]
    fn edge_status_covers_the_lifecycle() {
        assert_eq!(
            edge_status(Some(true), Some(true)),
            EdgeStatus::Verified,
            "both alive => verified"
        );
        assert_eq!(edge_status(None, Some(true)), EdgeStatus::Unverified);
        assert_eq!(edge_status(Some(true), None), EdgeStatus::Unverified);
        assert_eq!(edge_status(None, None), EdgeStatus::Unverified);
        // A known-deleted endpoint dominates, even against an alive one.
        assert_eq!(edge_status(Some(false), Some(true)), EdgeStatus::Dangling);
        assert_eq!(edge_status(Some(true), Some(false)), EdgeStatus::Dangling);
        assert_eq!(edge_status(Some(false), None), EdgeStatus::Dangling);
    }

    #[test]
    fn edge_status_token_round_trips() {
        for s in [
            EdgeStatus::Unverified,
            EdgeStatus::Verified,
            EdgeStatus::Dangling,
        ] {
            assert_eq!(EdgeStatus::from_token(s.as_str()), Some(s));
        }
        assert_eq!(EdgeStatus::from_token("nope"), None);
    }

    #[test]
    fn provenance_token_round_trips() {
        for p in [
            Provenance::Operator,
            Provenance::Import,
            Provenance::MatcherSuggested,
        ] {
            assert_eq!(Provenance::from_token(p.as_str()), Some(p));
        }
        assert_eq!(Provenance::from_token("nope"), None);
    }

    #[test]
    fn canonical_leaves_directed_kinds_untouched() {
        let p = r(EntityType::Person, 1);
        let o = r(EntityType::Organization, 2);
        let (from, to, directed) = canonical(p, o, EdgeKind::WorksAt);
        assert_eq!((from, to, directed), (p, o, true));
    }

    #[test]
    fn canonical_orders_symmetric_pair_by_urn_either_way() {
        let person = r(EntityType::Person, 5);
        let worker = r(EntityType::Worker, 5);
        // "person:..." < "worker:..." lexicographically, so person is `from`
        // regardless of which side asserted the edge.
        let a = canonical(person, worker, EdgeKind::SameIdentity);
        let b = canonical(worker, person, EdgeKind::SameIdentity);
        assert_eq!(a, b, "both assertion orders canonicalise identically");
        assert_eq!(a.0, person);
        assert_eq!(a.1, worker);
        assert!(!a.2, "symmetric edges are stored undirected");
    }

    #[test]
    fn single_view_unifies_identity_and_collects_affiliations() {
        let person = r(EntityType::Person, 1);
        let worker = r(EntityType::Worker, 1);
        let org = r(EntityType::Organization, 9);
        let other_org = r(EntityType::Organization, 10);
        let unrelated = r(EntityType::Person, 99);
        let unrelated_org = r(EntityType::Organization, 100);

        let edges = vec![
            EdgeView {
                from_ref: person,
                to_ref: worker,
                kind: EdgeKind::SameIdentity,
            },
            EdgeView {
                from_ref: worker,
                to_ref: org,
                kind: EdgeKind::EmployedBy,
            },
            EdgeView {
                from_ref: person,
                to_ref: other_org,
                kind: EdgeKind::WorksAt,
            },
            // An affiliation of a different identity — must be excluded.
            EdgeView {
                from_ref: unrelated,
                to_ref: unrelated_org,
                kind: EdgeKind::WorksAt,
            },
        ];

        // Starting from the person, the worker is unified via same_identity,
        // so the worker's employer is derivable (person -> worker -> org).
        let view = single_view(person, &edges);
        assert_eq!(view.identity_refs, vec![person, worker]);
        assert_eq!(view.affiliations.len(), 2, "employed_by + works_at only");
        assert!(
            view.affiliations
                .iter()
                .all(|e| e.kind != EdgeKind::SameIdentity)
        );
        assert!(view.affiliations.iter().any(|e| e.to_ref == org));
        assert!(view.affiliations.iter().any(|e| e.to_ref == other_org));
        assert!(
            !view.affiliations.iter().any(|e| e.from_ref == unrelated),
            "another identity's affiliation must not leak in"
        );

        // Starting from the worker gives the same unified view.
        let from_worker = single_view(worker, &edges);
        assert_eq!(from_worker.identity_refs, vec![person, worker]);
    }

    #[test]
    fn repoint_moves_a_directed_edge_onto_the_survivor() {
        let old = r(EntityType::Person, 1);
        let new = r(EntityType::Person, 2);
        let org = r(EntityType::Organization, 9);
        // works_at old->org, old merged into new  =>  works_at new->org.
        let got = repoint(old, org, EdgeKind::WorksAt, old, new);
        assert_eq!(got, Some((new, org, true)));
        // The other endpoint repoints too.
        let got = repoint(org, old, EdgeKind::WorksAt, old, new);
        assert_eq!(got, Some((org, new, true)));
    }

    #[test]
    fn repoint_re_canonicalises_a_symmetric_edge() {
        // same_identity worker:5 <-> person:5 is stored person-first.
        // Merge worker:5 into worker:1 (still "worker:1" > "person:5"),
        // so the survivor pair re-canonicalises person-first again.
        let person = r(EntityType::Person, 5);
        let worker_old = r(EntityType::Worker, 5);
        let worker_new = r(EntityType::Worker, 1);
        let got = repoint(
            person,
            worker_old,
            EdgeKind::SameIdentity,
            worker_old,
            worker_new,
        );
        assert_eq!(got, Some((person, worker_new, false)), "still URN-ordered");
    }

    #[test]
    fn repoint_drops_a_self_loop() {
        // An edge directly connecting the merged-away ref and the survivor
        // collapses to a self-loop and is dropped.
        let old = r(EntityType::Person, 1);
        let new = r(EntityType::Person, 2);
        assert_eq!(repoint(old, new, EdgeKind::WorksAt, old, new), None);
        assert_eq!(repoint(new, old, EdgeKind::WorksAt, old, new), None);
    }

    #[test]
    fn single_view_of_an_isolated_ref_is_just_itself() {
        let lonely = r(EntityType::Thing, 7);
        let edges: Vec<EdgeView> = vec![];
        let view = single_view(lonely, &edges);
        assert_eq!(view.identity_refs, vec![lonely]);
        assert!(view.affiliations.is_empty());
    }

    #[test]
    fn single_view_transitively_unifies_a_chain() {
        // p <-> w1 <-> w2 (two same_identity hops); all three unify.
        let p = r(EntityType::Person, 1);
        let w1 = r(EntityType::Worker, 1);
        let w2 = r(EntityType::Worker, 2);
        let org = r(EntityType::Organization, 3);
        let edges = vec![
            EdgeView {
                from_ref: p,
                to_ref: w1,
                kind: EdgeKind::SameIdentity,
            },
            EdgeView {
                from_ref: w1,
                to_ref: w2,
                kind: EdgeKind::SameIdentity,
            },
            EdgeView {
                from_ref: w2,
                to_ref: org,
                kind: EdgeKind::EmployedBy,
            },
        ];
        let view = single_view(p, &edges);
        assert_eq!(view.identity_refs.len(), 3);
        assert_eq!(view.affiliations.len(), 1);
        assert_eq!(view.affiliations[0].to_ref, org);
    }
}
