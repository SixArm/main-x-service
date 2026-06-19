//! Integration tests driving the public re-exported surface only —
//! everything reachable via `use portfolio_matcher::…`.

use portfolio_matcher::{
    Confidence, Goal, IdentifierScheme, MatchConfig, MatchingEngine, RelationKind, WorkItem,
    WorkItemIdentifier, WorkItemKind, WorkItemRelationship,
};

fn ident(scheme: IdentifierScheme, value: &str) -> WorkItemIdentifier {
    WorkItemIdentifier {
        scheme,
        value: value.into(),
    }
}

fn project(name: &str) -> WorkItem {
    WorkItem::new(WorkItemKind::Project, name)
}

// Pins the kind gate across every cross-kind pairing: different kinds
// never match, even with identical names.
#[test]
fn kind_gate_blocks_all_cross_kind_pairs() {
    let engine = MatchingEngine::default_config();
    let kinds = [
        WorkItemKind::Portfolio,
        WorkItemKind::Project,
        WorkItemKind::Product,
        WorkItemKind::Program,
    ];
    for ka in kinds {
        for kb in kinds {
            let a = WorkItem::new(ka, "Identical name");
            let b = WorkItem::new(kb, "Identical name");
            let r = engine.match_work_items(&a, &b);
            if ka == kb {
                assert!(r.is_match, "{ka:?}=={kb:?} identical names should match");
                assert!(!r.breakdown.kind_gate_blocked);
            } else {
                assert!(r.score.abs() < 1e-9, "{ka:?} vs {kb:?} must be 0.0");
                assert!(r.breakdown.kind_gate_blocked, "{ka:?} vs {kb:?}");
            }
        }
    }
}

// Pins R-0 across the whole deterministic scheme set: each globally
// unique scheme short-circuits to 1.0 / High (same kind).
#[test]
fn r0_fires_for_every_deterministic_scheme() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Uri,
        IdentifierScheme::Uuid,
        IdentifierScheme::JiraProjectKey,
        IdentifierScheme::AsanaGid,
        IdentifierScheme::TrelloBoardId,
        IdentifierScheme::MsProjectId,
        IdentifierScheme::GitHubProjectId,
        IdentifierScheme::LinearId,
    ] {
        let mut a = project("Left");
        let mut b = project("Right");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_work_items(&a, &b);
        assert!(
            r.breakdown.deterministic_match,
            "{scheme:?} should short-circuit"
        );
        assert!((r.score - 1.0).abs() < 1e-9);
        assert_eq!(r.confidence, Confidence::High);
    }
}

// Pins the converse of R-0: owner-scoped (Code / LocalId) and Custom
// schemes are NOT globally unique, so a shared value must not
// short-circuit.
#[test]
fn owner_scoped_and_custom_schemes_do_not_short_circuit() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Code,
        IdentifierScheme::LocalId,
        IdentifierScheme::Custom("LegacySystem".into()),
    ] {
        let mut a = project("Alpha project");
        let mut b = project("Beta project");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_work_items(&a, &b);
        assert!(
            !r.breakdown.deterministic_match,
            "{scheme:?} must not short-circuit"
        );
    }
}

// Pins R-1: same owner_org_id + equal (after normalisation) code
// short-circuits to 1.0, despite differing names.
#[test]
fn same_owner_code_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = project("Delivery v1");
    let mut b = project("Delivery v2");
    a.owner_org_id = Some("organization:9a2f".into());
    b.owner_org_id = Some("organization:9a2f".into());
    a.code = Some("PROJ-2026".into());
    b.code = Some("proj 2026".into());
    let r = engine.match_work_items(&a, &b);
    assert!((r.score - 1.0).abs() < 1e-9);
    assert!(r.breakdown.deterministic_match);
}

// Pins the owner gate: identical codes under different owners neither
// short-circuit nor contribute a component score.
#[test]
fn code_does_not_cross_match_across_owners() {
    let engine = MatchingEngine::default_config();
    let mut a = project("Alpha");
    let mut b = project("Beta");
    a.owner_org_id = Some("organization:1".into());
    b.owner_org_id = Some("organization:2".into());
    a.code = Some("PROJ-2026".into());
    b.code = Some("PROJ-2026".into());
    let r = engine.match_work_items(&a, &b);
    assert!(!r.breakdown.deterministic_match);
    assert!(r.breakdown.code_score.is_none());
}

// Pins R-2: an overlapping same_as URL short-circuits to 1.0.
#[test]
fn same_as_overlap_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = project("Alpha");
    let mut b = project("Omega");
    a.same_as = vec!["https://pm.example.com/p/APOLLO".into()];
    b.same_as = vec!["  https://pm.example.com/p/APOLLO/  ".into()];
    let r = engine.match_work_items(&a, &b);
    assert!((r.score - 1.0).abs() < 1e-9);
    assert!(r.breakdown.deterministic_match);
}

// Pins the parent-portfolio supporting signal: two child work items with
// fuzzy names but a shared parent portfolio score higher than the same
// pair with different parents.
#[test]
fn shared_parent_portfolio_corroborates() {
    let engine = MatchingEngine::default_config();
    let mut a = project("Apollo platform migration");
    let mut b = project("Apollo platform migrate");
    a.portfolio_ref = Some("portfolio-1".into());
    b.portfolio_ref = Some("portfolio-1".into());
    let shared = engine.match_work_items(&a, &b);
    b.portfolio_ref = Some("portfolio-2".into());
    let different = engine.match_work_items(&a, &b);
    assert!(shared.breakdown.portfolio_score == Some(1.0));
    assert!(different.breakdown.portfolio_score == Some(0.0));
    assert!(shared.score > different.score);
}

// Pins the full breakdown is populated on a rich probabilistic match and
// that every component is in range.
#[test]
fn breakdown_is_populated_and_in_range() {
    let engine = MatchingEngine::default_config();
    let mut a = project("Apollo platform migration");
    let mut b = project("Apollo platform migrate");
    a.owner_org_id = Some("organization:9a2f".into());
    b.owner_org_id = Some("organization:9a2f".into());
    a.portfolio_ref = Some("portfolio-1".into());
    b.portfolio_ref = Some("portfolio-1".into());
    a.goals = vec![Goal {
        title: "Cut latency".into(),
        ..Default::default()
    }];
    b.goals = vec![Goal {
        title: "Cut latency".into(),
        ..Default::default()
    }];
    a.start_date = Some("2026-01-01".into());
    b.start_date = Some("2026-02-01".into());
    a.keywords = vec!["infra".into()];
    b.keywords = vec!["infra".into()];
    a.tags = vec!["q1".into()];
    b.tags = vec!["q1".into()];
    a.relationships = vec![WorkItemRelationship {
        relation: RelationKind::DependsOn,
        work_item_id: "proj-9".into(),
    }];
    b.relationships = vec![WorkItemRelationship {
        relation: RelationKind::DependsOn,
        work_item_id: "proj-9".into(),
    }];
    let r = engine.match_work_items(&a, &b);
    let bd = &r.breakdown;
    for s in [
        bd.name_score,
        bd.goals_score,
        bd.owner_org_score,
        bd.portfolio_score,
        bd.timeframe_score,
        bd.keywords_score,
        bd.relationships_score,
        bd.tags_score,
    ] {
        let v = s.expect("component present");
        assert!((0.0..=1.0).contains(&v), "out of range: {v}");
    }
    assert!(r.is_match, "rich corroboration should match: {}", r.score);
}

// Pins the threshold presets change `is_match` without changing the raw
// score.
#[test]
fn presets_only_change_is_match() {
    // Construct a deterministic medium-band score independent of the
    // Jaro-Winkler internals: identical name (1.0 @ 0.30) plus a differing
    // owner org (0.0 @ 0.10) renormalises to (0.30)/(0.40) = 0.75 — above
    // the lenient threshold (0.70) but below strict (0.95).
    let mut a = project("Apollo platform migration");
    let mut b = project("Apollo platform migration");
    a.owner_org_id = Some("organization:1".into());
    b.owner_org_id = Some("organization:2".into());
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_work_items(&a, &b);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_work_items(&a, &b);
    assert!((lenient.score - strict.score).abs() < 1e-9);
    assert!((lenient.score - 0.75).abs() < 1e-9, "got {}", lenient.score);
    assert!(lenient.is_match);
    assert!(!strict.is_match);
}

// Pins the service bridge contract: a `WorkItem` round-trips losslessly
// through serde_json (the API DTO is this very type, persisted as JSONB).
#[test]
fn work_item_json_round_trip() {
    let mut w = project("Apollo platform migration");
    w.code = Some("PROJ-2026".into());
    w.owner_org_id = Some("organization:9a2f".into());
    w.portfolio_ref = Some("portfolio-1".into());
    w.goals = vec![Goal {
        title: "Cut latency".into(),
        ..Default::default()
    }];
    w.keywords = vec!["infra".into()];
    w.tags = vec!["q1".into()];
    w.relationships = vec![WorkItemRelationship {
        relation: RelationKind::DependsOn,
        work_item_id: "proj-9".into(),
    }];
    let json = serde_json::to_value(&w).expect("serialize");
    let back: WorkItem = serde_json::from_value(json).expect("deserialize");
    let again = serde_json::to_value(&back).expect("re-serialize");
    assert_eq!(serde_json::to_value(&w).unwrap(), again);
}
