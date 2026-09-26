//! T-28d: capacity-at-scale regression guard. Seeds a materially large
//! roster (60 plans, allocations across 40 shared people) and proves
//! `GET /capacity`, `GET /capacity/utilization`, and `GET /at-a-glance`
//! each run a **bounded** number of SQL queries — measured directly
//! via a tracing layer over SeaORM's own driver spans, not timed
//! (timing is noisy under load; a query count is exact and
//! deterministic).
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

/// Counts spans SeaORM's own Postgres driver opens for each round trip
/// (`execute`, `query_one`, `query_all`, `stream`, `begin`, …) — every
/// `#[instrument]`-annotated function in `sea_orm::driver::sqlx_postgres`
/// (checked against `sea-orm` 2.0.2's own source). Filtering on that
/// module-path target means this can't be confused with an
/// identically-named span in an unrelated library.
struct QueryCountLayer {
    counter: Arc<AtomicUsize>,
}

impl<S: tracing::Subscriber> Layer<S> for QueryCountLayer {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::span::Id,
        _ctx: Context<'_, S>,
    ) {
        if attrs.metadata().target().starts_with("sea_orm::driver::") {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Run `f`, returning its result plus the number of SeaORM driver
/// round trips made while it ran.
///
/// Uses `tracing::subscriber::set_default` (a thread-local override,
/// held across the awaited future) rather than
/// `set_global_default` (process-wide, settable only once — and this
/// crate's own `App::init_logger` already claims it at boot). This is
/// sound specifically because this crate's request tests run on
/// `#[tokio::test]`'s default **current-thread** runtime (no
/// `flavor = "multi_thread"` anywhere in this crate): a task can never
/// migrate to a different OS thread mid-await on that runtime, so the
/// thread-local override stays active for every task polled while it
/// is held, not only the top-level one.
///
/// `callsite::rebuild_interest_cache()` is required alongside it:
/// `tracing`'s per-callsite interest is cached the first time each
/// span callsite fires, and an earlier DB-gated test in this same
/// process has already fired every `sea_orm::driver::*` callsite under
/// whatever subscriber `App::init_logger` installed — which does not
/// raise its filter to `trace` for third-party spans. Without a forced
/// rebuild, that stale "not interesting" verdict would stick and this
/// layer would silently see zero spans forever.
async fn count_queries<F, Fut, T>(f: F) -> (T, usize)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let counter = Arc::new(AtomicUsize::new(0));
    let layer = QueryCountLayer {
        counter: counter.clone(),
    };
    let subscriber = tracing_subscriber::registry().with(layer);
    let guard = tracing::subscriber::set_default(subscriber);
    tracing_core::callsite::rebuild_interest_cache();
    let result = f().await;
    drop(guard);
    tracing_core::callsite::rebuild_interest_cache();
    (result, counter.load(Ordering::SeqCst))
}

/// Create one plan and one allocation for it; returns the plan pid.
async fn seed_one(request: &axum_test::TestServer, name: &str, person: &str) -> String {
    let created: Value = request
        .post("/api/plans")
        .json(&json!({ "kind": "Project", "name": name }))
        .await
        .json();
    let pid = created["pid"].as_str().expect("pid").to_string();
    request
        .post(&format!("/api/plans/{pid}/allocations"))
        .json(&json!({ "person_ref": person, "percent": 20 }))
        .await
        .assert_status_ok();
    pid
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// T-28d: query count for /capacity, /capacity/utilization, and
// /at-a-glance is identical at a small roster and at the full 60-plan
// / 40-person scale — proof the request-path queries are bulk reads,
// not a per-row fan-out that would grow with plan count.
async fn capacity_views_do_not_fan_out_with_scale() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let people: Vec<String> = (0..40)
            .map(|_| format!("worker:{}", uuid::Uuid::new_v4()))
            .collect();

        // A small roster first (5 plans): the baseline query count.
        let mut plan_pids = Vec::new();
        for i in 0..5 {
            plan_pids
                .push(seed_one(&request, &format!("Scale {i}"), &people[i % people.len()]).await);
        }

        let (_, capacity_small) =
            count_queries(|| async { request.get("/api/capacity").await }).await;
        let (_, utilization_small) =
            count_queries(|| async { request.get("/api/capacity/utilization?by=person").await })
                .await;
        let (_, glance_small) =
            count_queries(|| async { request.get("/api/at-a-glance").await }).await;
        assert!(
            capacity_small > 0,
            "the layer observed real /capacity queries"
        );
        assert!(
            utilization_small > 0,
            "the layer observed real /capacity/utilization queries"
        );
        assert!(
            glance_small > 0,
            "the layer observed real /at-a-glance queries"
        );

        // Scale up to 60 plans across the same 40 shared people.
        for i in 5..60 {
            plan_pids
                .push(seed_one(&request, &format!("Scale {i}"), &people[i % people.len()]).await);
        }
        assert_eq!(plan_pids.len(), 60);

        let (capacity_body, capacity_large) =
            count_queries(|| async { request.get("/api/capacity").await }).await;
        let (_, utilization_large) =
            count_queries(|| async { request.get("/api/capacity/utilization?by=person").await })
                .await;
        let (_, glance_large) =
            count_queries(|| async { request.get("/api/at-a-glance").await }).await;

        let capacity_json: Value = capacity_body.json();
        assert_eq!(
            capacity_json["people"].as_array().expect("people").len(),
            40,
            "all 40 shared people appear, each over multiple plans"
        );
        assert_eq!(
            capacity_small, capacity_large,
            "GET /capacity's query count must not grow with plan count \
             (was {capacity_small} at 5 plans, {capacity_large} at 60)"
        );
        assert_eq!(
            utilization_small, utilization_large,
            "GET /capacity/utilization's query count must not grow with plan count \
             (was {utilization_small} at 5 plans, {utilization_large} at 60)"
        );
        assert_eq!(
            glance_small, glance_large,
            "GET /at-a-glance's query count must not grow with plan count \
             (was {glance_small} at 5 plans, {glance_large} at 60)"
        );
    })
    .await;
}
