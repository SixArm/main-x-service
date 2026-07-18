//! Demo binary — a runnable walkthrough of the `project-portfolio-management-matcher`
//! public API. Not part of the public API's `SemVer` surface.

#![forbid(unsafe_code)] // demo binary needs no `unsafe` either.
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

/// Use mimalloc as the global allocator on MUSL static builds, where it
/// noticeably outperforms the default allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use project_portfolio_management_matcher::{
    Goal, IdentifierScheme, MatchConfig, MatchingEngine, WorkItem, WorkItemIdentifier, WorkItemKind,
};

/// Demo entry point: prints five representative scenarios (fuzzy name +
/// goal, the kind gate, a deterministic Jira-key short-circuit, a
/// same-owner code match, and an unrelated pair). Side effects: writes to
/// stdout only.
fn main() {
    let engine = MatchingEngine::new(MatchConfig::default());

    println!("== project-portfolio-management-matcher demo ==\n");

    // 1. Fuzzy name + shared goal title (same kind).
    let mut a = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    a.goals = vec![Goal {
        title: "Cut p95 latency".into(),
        ..Default::default()
    }];
    let mut b = WorkItem::new(WorkItemKind::Project, "Apollo platform migrate");
    b.goals = vec![Goal {
        title: "cut p95 latency".into(),
        ..Default::default()
    }];
    let r = engine.match_work_items(&a, &b);
    println!(
        "name+goal      : score={:.3}  {:?}  is_match={}",
        r.score, r.confidence, r.is_match
    );

    // 2. The kind gate: a Project and a Product never match.
    let a = WorkItem::new(WorkItemKind::Project, "Apollo");
    let b = WorkItem::new(WorkItemKind::Product, "Apollo");
    let r = engine.match_work_items(&a, &b);
    println!(
        "kind gate      : score={:.3}  kind_gate_blocked={}",
        r.score, r.breakdown.kind_gate_blocked
    );

    // 3. Deterministic Jira project key short-circuit despite different names.
    let mut a = WorkItem::new(WorkItemKind::Project, "Migrate billing");
    let mut b = WorkItem::new(WorkItemKind::Project, "Billing system rework");
    a.identifiers.push(WorkItemIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "BILL".into(),
    });
    b.identifiers.push(WorkItemIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "bill".into(),
    });
    let r = engine.match_work_items(&a, &b);
    println!(
        "jira key       : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 4. Same-owner code match.
    let mut a = WorkItem::new(WorkItemKind::Program, "Delivery v1");
    let mut b = WorkItem::new(WorkItemKind::Program, "Delivery v2");
    a.owner_org_id = Some("organization:9a2f".into());
    b.owner_org_id = Some("organization:9a2f".into());
    a.code = Some("PROG-2026".into());
    b.code = Some("prog 2026".into());
    let r = engine.match_work_items(&a, &b);
    println!(
        "owner+code     : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 5. Unrelated, same kind.
    let a = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    let b = WorkItem::new(WorkItemKind::Project, "Cafeteria refit programme");
    let r = engine.match_work_items(&a, &b);
    println!(
        "unrelated      : score={:.3}  is_match={}",
        r.score, r.is_match
    );
}
