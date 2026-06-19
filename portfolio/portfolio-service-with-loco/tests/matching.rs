//! DB-free tests: the service embeds `portfolio-matcher` directly and
//! persists `WorkItem` as JSON, so both the matcher contract (including
//! the kind gate) and the storage round-trip can be checked without a
//! database.

use portfolio_matcher::{
    IdentifierScheme, MatchingEngine, WorkItem, WorkItemIdentifier, WorkItemKind,
};

/// Pins that the service uses the canonical `portfolio-matcher` engine,
/// not a fork: two same-kind work items sharing a Jira project key hit
/// the deterministic short-circuit and score exactly `1.0`.
#[test]
fn embeds_the_canonical_matcher() {
    let engine = MatchingEngine::default_config();
    let mut a = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    let mut b = WorkItem::new(WorkItemKind::Project, "Apollo migration");
    a.identifiers.push(WorkItemIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "APOLLO".into(),
    });
    b.identifiers.push(WorkItemIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "APOLLO".into(),
    });
    let r = engine.match_work_items(&a, &b);
    assert!((r.score - 1.0).abs() < f64::EPSILON);
    assert!(r.breakdown.deterministic_match);
}

/// Pins the kind gate end to end from the service's perspective: a
/// project and a product with identical names never match (distinct
/// collections), so cross-collection matching is impossible.
#[test]
fn kind_gate_blocks_cross_collection() {
    let engine = MatchingEngine::default_config();
    let a = WorkItem::new(WorkItemKind::Project, "Apollo");
    let b = WorkItem::new(WorkItemKind::Product, "Apollo");
    let r = engine.match_work_items(&a, &b);
    assert!(r.score.abs() < f64::EPSILON);
    assert!(r.breakdown.kind_gate_blocked);
}

/// Pins the storage contract: a `WorkItem` survives a JSON round-trip
/// (`to_value` → `from_value`) unchanged, which is how the row's `data`
/// column persists and reloads the payload verbatim.
#[test]
fn work_item_json_round_trips_for_storage() {
    let mut w = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    w.code = Some("PROJ-2026".into());
    w.owner_org_id = Some("organization:9a2f".into());
    w.portfolio_ref = Some(uuid::Uuid::new_v4().to_string());
    let value = serde_json::to_value(&w).expect("to json");
    let back: WorkItem = serde_json::from_value(value).expect("from json");
    assert_eq!(back.name, w.name);
    assert_eq!(back.kind, w.kind);
    assert_eq!(back.code, w.code);
    assert_eq!(back.portfolio_ref, w.portfolio_ref);
}
