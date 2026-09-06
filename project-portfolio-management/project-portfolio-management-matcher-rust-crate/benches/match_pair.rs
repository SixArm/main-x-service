#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `project-portfolio-management-matcher`
//! crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator exercises: single-pair probabilistic matching, the
//! deterministic short-circuit, batch ranking, and the cost each config
//! preset adds. Numbers are absolute time per call.
//!
//! `kind` is deliberately varied across the candidate set: it is a label,
//! never a matching gate, so a benchmark that held it constant would hide
//! the cost of the component that actually runs.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use project_portfolio_management_matcher::{
    Goal, IdentifierScheme, MatchConfig, MatchingEngine, Plan, PlanIdentifier, PlanKind,
    PlanRelationship, PlanStatus, RelationKind,
};
use std::hint::black_box;

/// Terse `PlanIdentifier` constructor.
fn ident(scheme: IdentifierScheme, value: &str) -> PlanIdentifier {
    PlanIdentifier {
        scheme,
        value: value.into(),
    }
}

/// Terse `Goal` constructor — only the title is scored.
fn goal(title: &str) -> Goal {
    Goal {
        title: title.into(),
        description: None,
        target_date: None,
        status: None,
    }
}

/// The reference plan: every scored component populated (name, code,
/// owner, parent, timeframe, goals, keywords, tags, relationships) so a
/// benchmark exercises the whole pipeline.
fn build_reference() -> Plan {
    let mut p = Plan::new("Patient Records Modernisation");
    p.kind = Some(PlanKind::Program);
    p.alternate_names = vec!["PRM".into()];
    p.code = Some("PRM-2024".into());
    p.owner_org_id = Some("org-42".into());
    p.owner_org_name = Some("Example NHS Trust".into());
    p.lead_ref = Some("worker:0c4f1e2a".into());
    p.parent_ref = Some("plan:aaaa1111".into());
    p.status = Some(PlanStatus::Active);
    p.goals = vec![
        goal("Retire the legacy record store"),
        goal("Cut record retrieval time by half"),
    ];
    p.start_date = Some("2024-01-08".into());
    p.target_date = Some("2026-03-31".into());
    p.keywords = vec!["records".into(), "modernisation".into(), "nhs".into()];
    p.tags = vec!["strategic".into(), "digital".into()];
    p.relationships = vec![PlanRelationship {
        relation: RelationKind::DependsOn,
        plan_id: "plan:bbbb2222".into(),
    }];
    p.in_language = Some("en".into());
    p
}

/// A near-duplicate: the "same plan, different tracker" case that drives
/// the full probabilistic path — fuzzy name, differently-spaced code, a
/// **different** owner org so the owner-scoped rule cannot fire, and a
/// deliberately different `kind` (which must not gate the match).
fn build_near_duplicate() -> Plan {
    let mut p = Plan::new("Patient Record Modernization");
    p.kind = Some(PlanKind::Project);
    p.code = Some("prm 2024".into());
    p.owner_org_id = Some("org-99".into());
    p.owner_org_name = Some("Example Trust".into());
    p.parent_ref = Some("plan:aaaa1111".into());
    p.status = Some(PlanStatus::Active);
    p.goals = vec![goal("Retire legacy record store")];
    p.start_date = Some("2024-02-01".into());
    p.target_date = Some("2026-03-31".into());
    p.keywords = vec!["records".into(), "nhs".into()];
    p.tags = vec!["digital".into()];
    p
}

/// A clearly different plan, for the common "no match" path.
fn build_unrelated() -> Plan {
    let mut p = Plan::new("Estates Boiler Replacement");
    p.kind = Some(PlanKind::Project);
    p.code = Some("EST-0007".into());
    p.owner_org_id = Some("org-7".into());
    p.status = Some(PlanStatus::Completed);
    p.goals = vec![goal("Replace all site boilers")];
    p.start_date = Some("2019-06-01".into());
    p.target_date = Some("2020-12-31".into());
    p.keywords = vec!["estates".into(), "facilities".into()];
    p
}

/// Deterministically synthesise the `idx`-th candidate, so a candidate
/// list of any size is reproducible without randomness. `kind` cycles
/// through the whole enum on purpose (see the module docs).
fn make_candidate(idx: usize) -> Plan {
    let stems = ["Records", "Estates", "Workforce", "Analytics", "Networks"];
    let kinds = [
        PlanKind::Portfolio,
        PlanKind::Project,
        PlanKind::Product,
        PlanKind::Program,
        PlanKind::Practice,
    ];
    let mut p = Plan::new(format!(
        "{} Modernisation {}",
        stems[idx % stems.len()],
        idx % 400
    ));
    p.kind = Some(kinds[idx % kinds.len()]);
    p.code = Some(format!("PRM-{:04}", idx % 500));
    p.owner_org_id = Some(format!("org-{}", idx % 20));
    p.goals = vec![goal(&format!("Deliver milestone {}", idx % 60))];
    p.start_date = Some("2024-01-08".into());
    p.target_date = Some("2026-03-31".into());
    p.keywords = vec!["records".into(), format!("theme-{}", idx % 30)];
    p.tags = vec!["digital".into()];
    p
}

/// Single-pair probabilistic matching across three regimes: an identical
/// clone (best case), a fuzzy near-match (full pipeline), and an
/// unrelated pair (mostly-zero sub-scores).
fn bench_match_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_pair");
    let reference = build_reference();
    let near = build_near_duplicate();
    let unrelated = build_unrelated();
    let engine = MatchingEngine::default_config();

    group.bench_function("identical_clone", |b| {
        let clone = reference.clone();
        b.iter(|| engine.match_plans(black_box(&reference), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_plans(black_box(&reference), black_box(&near)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_plans(black_box(&reference), black_box(&unrelated)));
    });
    group.finish();
}

/// The deterministic paths — the cheapest decisions the engine makes and
/// the ones a duplicate check hits most often: a shared Jira project key
/// (globally unique) and a shared owner-scoped code.
fn bench_deterministic(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic");
    let engine = MatchingEngine::default_config();

    let mut a = build_reference();
    let mut b = build_unrelated();
    a.identifiers
        .push(ident(IdentifierScheme::JiraProjectKey, "PRM"));
    b.identifiers
        .push(ident(IdentifierScheme::JiraProjectKey, "PRM"));
    group.bench_function("shared_jira_key", |bencher| {
        bencher.iter(|| engine.match_plans(black_box(&a), black_box(&b)));
    });

    let left = build_reference();
    let mut right = build_near_duplicate();
    right.owner_org_id.clone_from(&left.owner_org_id);
    group.bench_function("same_owner_code", |bencher| {
        bencher.iter(|| engine.match_plans(black_box(&left), black_box(&right)));
    });
    group.finish();
}

/// `rank` at 10/100/1000 candidates, with `Throughput::Elements` so
/// Criterion reports per-candidate cost.
fn bench_rank(c: &mut Criterion) {
    let mut group = c.benchmark_group("rank");
    let engine = MatchingEngine::default_config();
    let query = build_reference();

    for &n in &[10usize, 100, 1000] {
        let candidates: Vec<Plan> = (0..n).map(make_candidate).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &candidates, |b, cands| {
            b.iter(|| engine.rank(black_box(&query), black_box(cands)));
        });
    }
    group.finish();
}

/// The same fuzzy pair under each shipped preset, exposing what the
/// chosen thresholds and weights cost per call.
fn bench_config_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_variants");
    let reference = build_reference();
    let near = build_near_duplicate();

    for (label, engine) in [
        ("default", MatchingEngine::default_config()),
        ("strict", MatchingEngine::new(MatchConfig::strict())),
        ("lenient", MatchingEngine::new(MatchConfig::lenient())),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| engine.match_plans(black_box(&reference), black_box(&near)));
        });
    }
    group.finish();
}

/// For each of the four unbounded array fields
/// (`goals`/`keywords`/`relationships`/`tags`), two records held fixed
/// except that one field, grown to `n` entries (10/100/1000) — the
/// other array fields stay at their `build_reference`/
/// `build_near_duplicate` size, so each field's own O(n·m) cost is
/// visible on its own axis rather than conflated with the others
/// (AGENTS.md golden rule 8: none of the four has a length cap of its
/// own). Roughly half the entries overlap between the two sides, so
/// the Jaccard set intersection/union does real work rather than
/// short-circuiting on an empty intersection.
fn bench_field_arrays(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_arrays");
    let engine = MatchingEngine::default_config();

    for &n in &[10usize, 100, 1000] {
        group.throughput(Throughput::Elements(n as u64));

        let mut a = build_reference();
        let mut b = build_near_duplicate();
        a.goals = (0..n).map(|i| goal(&format!("Milestone {i}"))).collect();
        b.goals = (0..n)
            .map(|i| {
                goal(&if i.is_multiple_of(2) {
                    format!("Milestone {i}")
                } else {
                    format!("Other milestone {i}")
                })
            })
            .collect();
        group.bench_with_input(BenchmarkId::new("goals", n), &(a, b), |bencher, (a, b)| {
            bencher.iter(|| engine.match_plans(black_box(a), black_box(b)));
        });

        let mut a = build_reference();
        let mut b = build_near_duplicate();
        a.keywords = (0..n).map(|i| format!("kw-{i}")).collect();
        b.keywords = (0..n)
            .map(|i| {
                if i.is_multiple_of(2) {
                    format!("kw-{i}")
                } else {
                    format!("other-kw-{i}")
                }
            })
            .collect();
        group.bench_with_input(
            BenchmarkId::new("keywords", n),
            &(a, b),
            |bencher, (a, b)| {
                bencher.iter(|| engine.match_plans(black_box(a), black_box(b)));
            },
        );

        let mut a = build_reference();
        let mut b = build_near_duplicate();
        a.tags = (0..n).map(|i| format!("tag-{i}")).collect();
        b.tags = (0..n)
            .map(|i| {
                if i.is_multiple_of(2) {
                    format!("tag-{i}")
                } else {
                    format!("other-tag-{i}")
                }
            })
            .collect();
        group.bench_with_input(BenchmarkId::new("tags", n), &(a, b), |bencher, (a, b)| {
            bencher.iter(|| engine.match_plans(black_box(a), black_box(b)));
        });

        let mut a = build_reference();
        let mut b = build_near_duplicate();
        a.relationships = (0..n)
            .map(|i| PlanRelationship {
                relation: RelationKind::DependsOn,
                plan_id: format!("plan:{i:08x}"),
            })
            .collect();
        b.relationships = (0..n)
            .map(|i| PlanRelationship {
                relation: RelationKind::DependsOn,
                plan_id: if i.is_multiple_of(2) {
                    format!("plan:{i:08x}")
                } else {
                    format!("plan:other-{i:08x}")
                },
            })
            .collect();
        group.bench_with_input(
            BenchmarkId::new("relationships", n),
            &(a, b),
            |bencher, (a, b)| {
                bencher.iter(|| engine.match_plans(black_box(a), black_box(b)));
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_match_pair,
    bench_deterministic,
    bench_rank,
    bench_config_variants,
    bench_field_arrays
);
criterion_main!(benches);
