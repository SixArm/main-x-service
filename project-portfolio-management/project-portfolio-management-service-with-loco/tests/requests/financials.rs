//! Request tests for the **phased budget baseline** and the **EAC/ETC
//! forecast** (T-28b, `spec/13-tasks.md`). Pins the acceptance
//! criteria verbatim: a plan without a baseline reports `null` +
//! a reason unchanged; a re-baseline preserves its predecessor and the
//! old figure stays reproducible; a rollup over two currencies reports
//! two rows, never one sum.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// Seed one plan and return its pid.
async fn seed_plan(request: &axum_test::TestServer, name: &str) -> String {
    let created = request
        .post("/api/plans")
        .json(&json!({ "kind": "Project", "name": format!("{name} {}", uuid::Uuid::new_v4()) }))
        .await;
    created.assert_status_ok();
    let plan: Value = created.json();
    plan["pid"].as_str().expect("plan pid").to_string()
}

/// Create a plan nested under `parent`, returning its pid.
async fn seed_child(request: &axum_test::TestServer, parent: &str, name: &str) -> String {
    let created = request
        .post("/api/plans")
        .json(&json!({ "kind": "Project", "name": format!("{name} {}", uuid::Uuid::new_v4()), "parent_ref": parent }))
        .await;
    created.assert_status_ok();
    let plan: Value = created.json();
    plan["pid"].as_str().expect("child pid").to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_plan_without_a_baseline_reports_null_with_a_reason() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request, "Unbaselined").await;
        let body: Value = request
            .get(&format!("/api/plans/{plan}/financials/forecast"))
            .await
            .json();
        assert!(body["forecast"]["currency"].is_null());
        assert_eq!(body["forecast"]["absent"], "no_currency_signal");
        assert!(body["forecast"]["estimate_at_completion_minor"].is_null());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn registering_a_baseline_unlocks_a_forecast_from_its_remaining_periods() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request, "Baselined").await;

        let registered: Value = request
            .post(&format!("/api/plans/{plan}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [
                    { "period_start": "2020-01-01", "period_end": "2020-03-31", "planned_minor": 100_000 },
                    { "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 500_000 },
                ],
            }))
            .await
            .json();
        assert_eq!(registered["version"], 1);

        request
            .post(&format!("/api/plans/{plan}/budget-lines"))
            .json(&json!({ "category": "capex", "description": "spend", "currency": "GBP", "planned_minor": 0 }))
            .await
            .assert_status_ok();

        let forecast: Value = request
            .get(&format!("/api/plans/{plan}/financials/forecast"))
            .await
            .json();
        assert_eq!(forecast["forecast"]["currency"], "GBP");
        assert_eq!(forecast["forecast"]["etc_source"], "baseline_remaining");
        // The 2020 period has elapsed; only the 2099 one is remaining.
        assert_eq!(forecast["forecast"]["estimate_to_complete_minor"], 500_000);
        assert_eq!(forecast["forecast"]["estimate_at_completion_minor"], 500_000);
        assert_eq!(forecast["forecast"]["baseline_version"], 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_tpc_observation_takes_precedence_over_the_baseline() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request, "TpcWins").await;
        request
            .post(&format!("/api/plans/{plan}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 999_000 }],
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/plans/{plan}/tpc"))
            .json(&json!({
                "currency": "GBP", "expected_monetary_value": 0,
                "cost_estimate_to_complete": 42_000,
            }))
            .await
            .assert_status_ok();

        let forecast: Value = request
            .get(&format!("/api/plans/{plan}/financials/forecast"))
            .await
            .json();
        assert_eq!(forecast["forecast"]["etc_source"], "tpc");
        assert_eq!(forecast["forecast"]["estimate_to_complete_minor"], 42_000);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_rebaseline_needs_a_reason_and_the_predecessor_stays_reproducible() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request, "Rebaselined").await;
        request
            .post(&format!("/api/plans/{plan}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 100_000 }],
            }))
            .await
            .assert_status_ok();

        // Silently re-baselining is refused — a re-baseline with no
        // stated reason is indistinguishable from a mistake.
        let no_reason = request
            .post(&format!("/api/plans/{plan}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 200_000 }],
            }))
            .await;
        assert_eq!(no_reason.status_code(), 422);

        let rebaselined: Value = request
            .post(&format!("/api/plans/{plan}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 200_000 }],
                "reason": "scope grew",
            }))
            .await
            .json();
        assert_eq!(rebaselined["version"], 2);

        let history: Value = request
            .get(&format!("/api/plans/{plan}/budget-baselines"))
            .await
            .json();
        let versions: Vec<Value> = history.as_array().expect("history").clone();
        assert_eq!(versions.len(), 2, "the predecessor is preserved, not overwritten");
        // Newest first.
        assert_eq!(versions[0]["version"], 2);
        assert_eq!(versions[0]["reason"], "scope grew");
        assert_eq!(versions[1]["version"], 1);
        assert!(versions[1]["reason"].is_null(), "the first baseline needed no reason");
        assert_eq!(
            versions[1]["periods"][0]["planned_minor"], 100_000,
            "the old version's own figure is reproducible from that version"
        );

        // The live forecast now uses the newer version.
        let forecast: Value = request
            .get(&format!("/api/plans/{plan}/financials/forecast"))
            .await
            .json();
        assert_eq!(forecast["forecast"]["baseline_version"], 2);
        assert_eq!(forecast["forecast"]["estimate_to_complete_minor"], 200_000);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_rollup_never_merges_two_currencies_into_one_sum() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let root = seed_plan(&request, "RollupRoot").await;
        let gbp_child = seed_child(&request, &root, "GbpChild").await;
        let usd_child = seed_child(&request, &root, "UsdChild").await;

        request
            .post(&format!("/api/plans/{gbp_child}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 10_000 }],
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/plans/{usd_child}/budget-baselines"))
            .json(&json!({
                "currency": "USD",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 20_000 }],
            }))
            .await
            .assert_status_ok();

        let body: Value = request
            .get(&format!("/api/financials/forecast?plan={root}"))
            .await
            .json();
        let rows: Vec<Value> = body["rollup"]["rows"].as_array().expect("rows").clone();
        assert_eq!(rows.len(), 2, "two currencies, two rows, never one sum");
        let currencies: Vec<&str> = rows.iter().filter_map(|r| r["currency"].as_str()).collect();
        assert!(currencies.contains(&"GBP"));
        assert!(currencies.contains(&"USD"));
        // The root itself has no baseline; disclosed, not silently
        // absent from the response.
        assert_eq!(body["rollup"]["no_currency_signal"], 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn performance_names_the_missing_earned_value_signal_once_a_baseline_exists() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request, "PerformancePlan").await;

        let before: Value = request
            .get(&format!("/api/plans/{plan}/performance"))
            .await
            .json();
        assert_eq!(before["schedule"]["absent"], "no_baseline");

        request
            .post(&format!("/api/plans/{plan}/budget-baselines"))
            .json(&json!({
                "currency": "GBP",
                "periods": [{ "period_start": "2099-01-01", "period_end": "2099-12-31", "planned_minor": 1_000 }],
            }))
            .await
            .assert_status_ok();

        let after: Value = request
            .get(&format!("/api/plans/{plan}/performance"))
            .await
            .json();
        assert!(
            after["schedule"]["spi"].is_null(),
            "still null — a baseline alone is not an earned-value signal"
        );
        assert_eq!(
            after["schedule"]["absent"], "no_earned_value_signal",
            "the reason must stay true: it is no longer really \"no baseline\""
        );
    })
    .await;
}
