#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `project-portfolio-management-service`
//! pure request-path work: payload validation, record merge, and the
//! Tantivy retrieval that backs duplicate detection.
//!
//! Run with `cargo bench`. Nothing here touches Postgres — these are the
//! CPU-bound halves of a request, which is exactly the part a database
//! benchmark would hide behind I/O.
//!
//! What the groups are for:
//!
//! - **`validation`** — runs on every create and update. The
//!   `oversized_arrays` case exercises the SEC-M1 input caps: rejecting
//!   an abusive payload must be cheap.
//! - **`merge`** — a whole-record fold; the scaling case shows the cost
//!   sits in the collections it unions.
//! - **`search`** — `index_one_document` is what every create, update,
//!   and merge pays synchronously. The `kind` filter is benchmarked
//!   with and without a value because it is a **search** filter only:
//!   it narrows retrieval and never gates matching, so its cost belongs
//!   here and nowhere near the matcher.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use project_portfolio_management_matcher::{
    Goal, IdentifierScheme, Plan, PlanIdentifier, PlanKind, PlanStatus,
};
use project_portfolio_management_service::{
    merge,
    search::{SearchEngine, SearchMode},
    validation,
};
use uuid::Uuid;

fn goal(title: &str) -> Goal {
    Goal {
        title: title.into(),
        description: None,
        target_date: None,
        status: None,
    }
}

/// A fully populated plan — every validated field present, so validation
/// runs its whole rule set.
fn populated() -> Plan {
    let mut p = Plan::new("Patient Records Modernisation");
    p.kind = Some(PlanKind::Program);
    p.alternate_names = vec!["PRM".into()];
    p.code = Some("PRM-2024".into());
    p.owner_org_id = Some("org-42".into());
    p.owner_org_name = Some("Example NHS Trust".into());
    p.lead_ref = Some("worker:0c4f1e2a".into());
    p.status = Some(PlanStatus::Active);
    p.goals = vec![
        goal("Retire the legacy record store"),
        goal("Cut record retrieval time by half"),
    ];
    p.start_date = Some("2024-01-08".into());
    p.target_date = Some("2026-03-31".into());
    p.keywords = vec!["records".into(), "modernisation".into()];
    p.tags = vec!["strategic".into(), "digital".into()];
    p.identifiers = vec![PlanIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: "PRM".into(),
    }];
    p.same_as = vec!["https://tracker.example/PRM".into()];
    p
}

/// A payload violating several rules at once — the controller collects
/// every problem in one pass, so this is the realistic bad-request shape.
fn invalid() -> Plan {
    let mut p = populated();
    p.name = "   ".into();
    p.start_date = Some("2024-13-45".into());
    p.keywords = vec![String::new(), "records".into()];
    p.identifiers = vec![PlanIdentifier {
        scheme: IdentifierScheme::JiraProjectKey,
        value: String::new(),
    }];
    p
}

/// Deliberately over the SEC-M1 caps: the shape an attacker sends to buy
/// O(n·m) scoring work with one cheap request.
fn oversized() -> Plan {
    let mut p = populated();
    p.name = "x".repeat(64 * 1024);
    p.goals = (0..10_000).map(|i| goal(&format!("goal-{i}"))).collect();
    p.keywords = (0..10_000).map(|i| format!("keyword-{i}")).collect();
    p
}

/// The `idx`-th distinct plan. `kind` cycles on purpose: it is a label
/// and a search filter, never a matching gate.
fn variant(idx: usize) -> Plan {
    let stems = ["Records", "Estates", "Workforce", "Analytics", "Networks"];
    let kinds = [
        PlanKind::Portfolio,
        PlanKind::Project,
        PlanKind::Product,
        PlanKind::Program,
        PlanKind::Practice,
    ];
    let mut p = Plan::new(format!("{} Modernisation {idx}", stems[idx % stems.len()]));
    p.kind = Some(kinds[idx % kinds.len()]);
    p.code = Some(format!("PRM-{:04}", idx % 500));
    p.owner_org_id = Some(format!("org-{}", idx % 20));
    p.goals = vec![goal(&format!("Deliver milestone {}", idx % 60))];
    p.keywords = vec!["records".into(), format!("theme-{}", idx % 30)];
    p.tags = vec!["digital".into()];
    p
}

fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    let valid = populated();
    let invalid = invalid();
    let oversized = oversized();

    group.bench_function("valid_payload", |b| {
        b.iter(|| validation::problems(black_box(&valid)));
    });
    group.bench_function("several_problems", |b| {
        b.iter(|| validation::problems(black_box(&invalid)));
    });
    group.bench_function("oversized_arrays", |b| {
        b.iter(|| validation::problems(black_box(&oversized)));
    });
    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge");
    let main = populated();
    let duplicate = variant(7);
    group.bench_function("typical_pair", |b| {
        b.iter(|| merge::merge_plans(black_box(&main), black_box(&duplicate)));
    });

    // Collection union is where merge's cost lives, so scale that.
    let mut big_main = populated();
    big_main.keywords = (0..500).map(|i| format!("keyword-{i}")).collect();
    big_main.tags = (0..500).map(|i| format!("tag-{i}")).collect();
    let mut big_dup = populated();
    big_dup.keywords = (250..750).map(|i| format!("keyword-{i}")).collect();
    big_dup.tags = (250..750).map(|i| format!("tag-{i}")).collect();
    group.bench_function("many_keywords_half_overlapping", |b| {
        b.iter(|| merge::merge_plans(black_box(&big_main), black_box(&big_dup)));
    });
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = SearchEngine::new(dir.path()).expect("index opens");
    for i in 0..500 {
        engine
            .index_plan(Uuid::new_v4(), &variant(i))
            .expect("index write");
    }

    let mut group = c.benchmark_group("search");
    // Its own index, deliberately: measured against the populated one
    // the number would be dominated by accumulated segment state rather
    // than by what a write costs.
    let write_dir = tempfile::tempdir().expect("temp dir");
    let write_engine = SearchEngine::new(write_dir.path()).expect("index opens");
    group.bench_function("index_one_document", |b| {
        let plan = populated();
        b.iter(|| {
            write_engine
                .index_plan(black_box(Uuid::new_v4()), black_box(&plan))
                .expect("index write");
        });
    });

    for (label, mode) in [
        ("exact", SearchMode::Exact),
        ("fuzzy", SearchMode::Fuzzy),
        ("phonetic", SearchMode::Phonetic),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| {
                engine
                    .search_page(black_box("Records Modernisation"), mode, None, 20, 0)
                    .expect("search")
            });
        });
    }
    // The same query narrowed by `kind`, so the filter's cost is visible
    // rather than folded into the unfiltered number.
    group.bench_function("exact_kind_filtered", |b| {
        b.iter(|| {
            engine
                .search_page(
                    black_box("Records Modernisation"),
                    SearchMode::Exact,
                    Some("project"),
                    20,
                    0,
                )
                .expect("search")
        });
    });

    let probe = populated();
    for &limit in &[10usize, 50] {
        group.throughput(Throughput::Elements(limit as u64));
        group.bench_with_input(
            BenchmarkId::new("duplicate_candidates", limit),
            &limit,
            |b, &limit| {
                b.iter(|| {
                    engine
                        .candidates(black_box(&probe), limit)
                        .expect("candidates")
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_validation, bench_merge, bench_search);
criterion_main!(benches);
