//! DB-free tests: the service embeds `project-portfolio-management-matcher` directly and
//! persists `Plan` as JSON, so both the matcher contract (one unified
//! recursive collection, no kind gate) and the storage round-trip can be
//! checked without a database.

use project_portfolio_management_matcher::{
    IdentifierScheme, MatchingEngine, Plan, PlanIdentifier, PlanKind,
};

/// Pins that the service uses the canonical `project-portfolio-management-matcher` engine,
/// not a fork: two same-kind plans sharing a Jira project key hit
/// the deterministic short-circuit and score exactly `1.0`.
#[test]
fn embeds_the_canonical_matcher() {
    let engine = MatchingEngine::default_config();
    let mut a = Plan::new("Apollo platform migration");
    let mut b = Plan::new("Apollo migration");
    a.identifiers.push(PlanIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "APOLLO".into(),
    });
    b.identifiers.push(PlanIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "APOLLO".into(),
    });
    let r = engine.match_plans(&a, &b);
    assert!((r.score - 1.0).abs() < f64::EPSILON);
    assert!(r.breakdown.deterministic_match);
}

/// Pins the unified model end to end from the service's perspective:
/// `kind` is an optional label and never gates matching, so two plans
/// with different kinds but identical names still match.
#[test]
fn different_kinds_still_match() {
    let engine = MatchingEngine::default_config();
    let mut a = Plan::new("Apollo");
    a.kind = Some(PlanKind::Project);
    let mut b = Plan::new("Apollo");
    b.kind = Some(PlanKind::Product);
    let r = engine.match_plans(&a, &b);
    assert!(r.score >= 0.99, "got {}", r.score);
    assert!(r.is_match);
    assert!(!r.breakdown.kind_gate_blocked);
}

/// Pins the storage contract: a `Plan` survives a JSON round-trip
/// (`to_value` → `from_value`) unchanged, which is how the row's `data`
/// column persists and reloads the payload verbatim.
#[test]
fn plan_json_round_trips_for_storage() {
    let mut w = Plan::new("Apollo platform migration");
    w.code = Some("PROJ-2026".into());
    w.owner_org_id = Some("organization:9a2f".into());
    w.parent_ref = Some(uuid::Uuid::new_v4().to_string());
    let value = serde_json::to_value(&w).expect("to json");
    let back: Plan = serde_json::from_value(value).expect("from json");
    assert_eq!(back.name, w.name);
    assert_eq!(back.kind, w.kind);
    assert_eq!(back.code, w.code);
    assert_eq!(back.parent_ref, w.parent_ref);
}
