//! Time-based-analysis request tests: transitions are written by the
//! existing task endpoints, and the derived per-task, plan, constraint,
//! aging-WIP and flow views read them back. Pins the contract in
//! `spec/time-based-analysis.md` §14.3.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// Seed one plan and return its pid.
async fn seed_plan(request: &axum_test::TestServer) -> String {
    let created = request
        .post("/api/plans")
        .json(&json!({ "kind": "Project", "name": format!("Flow {}", uuid::Uuid::new_v4()) }))
        .await;
    created.assert_status_ok();
    let plan: Value = created.json();
    plan["pid"].as_str().expect("plan pid").to_string()
}

/// Create a task on a plan and return its pid.
async fn seed_task(request: &axum_test::TestServer, plan: &str, title: &str) -> String {
    let created = request
        .post(&format!("/api/plans/{plan}/tasks"))
        .json(&json!({ "title": title }))
        .await;
    created.assert_status_ok();
    let task: Value = created.json();
    task["pid"].as_str().expect("task pid").to_string()
}

/// Move a task and assert the move was accepted.
async fn move_task(request: &axum_test::TestServer, plan: &str, task: &str, status: &str) {
    request
        .patch(&format!("/api/plans/{plan}/tasks/{task}"))
        .json(&json!({ "status": status }))
        .await
        .assert_status_ok();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded board, the whole TBA surface
async fn time_based_analysis_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request).await;
        let task = seed_task(&request, &plan, "Build the thing").await;

        // ── Creating a task opens its log. Without this the analysis
        // would silently begin the item's life at its first later move.
        let log: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{task}/transitions"))
            .await
            .json();
        let entries = log["transitions"].as_array().expect("transitions");
        assert_eq!(entries.len(), 1, "creation writes the opening transition");
        assert_eq!(entries[0]["from_status"], Value::Null);
        assert_eq!(entries[0]["to_status"], "todo");
        assert_eq!(entries[0]["backfilled"], false, "observed, not synthesised");

        // ── Each accepted move appends one transition.
        move_task(&request, &plan, &task, "in_progress").await;
        move_task(&request, &plan, &task, "in_review").await;
        move_task(&request, &plan, &task, "in_progress").await; // rework
        move_task(&request, &plan, &task, "done").await;
        let log: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{task}/transitions"))
            .await
            .json();
        let entries = log["transitions"].as_array().expect("transitions");
        assert_eq!(entries.len(), 5);
        let pairs: Vec<(&str, &str)> = entries
            .iter()
            .skip(1)
            .map(|e| {
                (
                    e["from_status"].as_str().unwrap_or(""),
                    e["to_status"].as_str().unwrap_or(""),
                )
            })
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("todo", "in_progress"),
                ("in_progress", "in_review"),
                ("in_review", "in_progress"),
                ("in_progress", "done"),
            ]
        );

        // ── A no-op move writes nothing: the log must record moves that
        // happened, not requests that were made.
        move_task(&request, &plan, &task, "done").await;
        let log: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{task}/transitions"))
            .await
            .json();
        assert_eq!(
            log["transitions"].as_array().expect("transitions").len(),
            5,
            "a no-op move appends nothing"
        );

        // ── A refused move writes nothing either.
        assert_eq!(
            request
                .patch(&format!("/api/plans/{plan}/tasks/{task}"))
                .json(&json!({ "status": "sideways" }))
                .await
                .status_code(),
            422,
            "unknown status refused"
        );
        let log: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{task}/transitions"))
            .await
            .json();
        assert_eq!(
            log["transitions"].as_array().expect("transitions").len(),
            5,
            "a refused move must not reach the log"
        );

        // ── Per-task analysis.
        let analysis: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{task}/time-analysis"))
            .await
            .json();
        let a = &analysis["analysis"];
        assert_eq!(a["finished"], true);
        assert_eq!(a["transitions"], 5);
        assert_eq!(a["backfilled"], 0);
        assert_eq!(a["rework_count"], 1, "in_review → in_progress");
        assert_eq!(a["first_pass"], false);
        // The statuses partition the lead time exactly (spec §12.3).
        let status_total: i64 = a["by_status"]
            .as_array()
            .expect("by_status")
            .iter()
            .map(|s| s["ms"].as_i64().unwrap_or(0))
            .sum();
        assert_eq!(status_total, a["lead_time_ms"].as_i64().expect("lead"));
        let category_total: i64 = a["by_category"]
            .as_array()
            .expect("by_category")
            .iter()
            .map(|c| c["ms"].as_i64().unwrap_or(0))
            .sum();
        assert_eq!(category_total, a["lead_time_ms"].as_i64().expect("lead"));
        // Cycle time is never longer than lead time.
        assert!(
            a["cycle_time_ms"].as_i64().unwrap_or(0) <= a["lead_time_ms"].as_i64().unwrap_or(0),
            "cycle ≤ lead"
        );
        // `done` is terminal, so it is not one of the statuses.
        assert!(
            !a["by_status"]
                .as_array()
                .expect("by_status")
                .iter()
                .any(|s| s["status"] == "done"),
            "`done` is not an interval"
        );
        // The classification in force travels with every figure.
        assert_eq!(analysis["classification"]["overridden"], false);
        assert_eq!(
            analysis["classification"]["classes"]["in_progress"],
            "value_adding"
        );

        // ── A task that never started has a lead time but no cycle time.
        let fresh = seed_task(&request, &plan, "Not started yet").await;
        let idle: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{fresh}/time-analysis"))
            .await
            .json();
        assert_eq!(idle["analysis"]["cycle_time_ms"], Value::Null);
        assert!(
            idle["analysis"]["cycle_time_reason"].is_string(),
            "a null must say why"
        );
        assert_eq!(idle["analysis"]["flow_efficiency"]["value"], Value::Null);

        // ── Plan cohort.
        let cohort: Value = request
            .get(&format!("/api/plans/{plan}/time-analysis"))
            .await
            .json();
        let summary = &cohort["plan_analysis"];
        assert_eq!(summary["tasks"], 2);
        assert_eq!(summary["finished"], 1);
        assert_eq!(summary["not_started"], 1);
        assert_eq!(summary["rework_count"], 1);
        assert_eq!(
            summary["rolled_first_pass_yield"], 0.0,
            "the one finished item bounced"
        );
        // Lead time is always reported beside cycle time.
        assert!(summary["cycle_time"].is_object() && summary["lead_time"].is_object());
        // One finished item is far below the SLE minimum sample.
        assert_eq!(
            cohort["service_level_expectation"]["within_ms"],
            Value::Null
        );
        assert!(cohort["service_level_expectation"]["reason"].is_string());

        assert_eq!(
            request
                .get(&format!("/api/plans/{plan}/time-analysis?sle_percentile=2"))
                .await
                .status_code(),
            422,
            "a percentile outside [0,1] is refused, not clamped"
        );
        assert_eq!(
            request
                .get(&format!("/api/plans/{plan}/time-analysis?target_days=0"))
                .await
                .status_code(),
            422
        );

        // ── Constraints.
        let constraints: Value = request
            .get(&format!("/api/plans/{plan}/constraints"))
            .await
            .json();
        let findings = constraints["findings"].as_array().expect("findings");
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f["rule"] == "rework"));
        let recoverable: Vec<i64> = findings
            .iter()
            .map(|f| f["recoverable_ms"].as_i64().unwrap_or(0))
            .collect();
        assert!(
            recoverable.windows(2).all(|w| w[0] >= w[1]),
            "ordered by recoverable time: {recoverable:?}"
        );

        // ── Aging WIP lists the open item even with no SLE to compare.
        let aging: Value = request
            .get(&format!("/api/plans/{plan}/aging-wip"))
            .await
            .json();
        let rows = aging["aging"].as_array().expect("aging");
        assert!(
            rows.is_empty(),
            "the only open item never started, so it has no age yet"
        );

        move_task(&request, &plan, &fresh, "in_progress").await;
        let aging: Value = request
            .get(&format!("/api/plans/{plan}/aging-wip"))
            .await
            .json();
        let rows = aging["aging"].as_array().expect("aging");
        assert_eq!(rows.len(), 1, "now it is in progress and aging");
        assert_eq!(
            rows[0]["aging"]["sle_ratio"],
            Value::Null,
            "no expectation yet, so no ratio — and it is still listed"
        );
        assert_eq!(rows[0]["aging"]["past_sle"], false);

        // ── Flow.
        let flow: Value = request
            .get(&format!("/api/plans/{plan}/flow?window_days=30"))
            .await
            .json();
        assert_eq!(flow["flow"]["window_days"], 30);
        assert_eq!(flow["flow"]["arrivals"], 2);
        assert_eq!(flow["flow"]["completions"], 1);
        assert_eq!(flow["flow"]["work_in_progress"], 1);
        assert!(flow["flow"]["interpretation"].is_string());
        // Column occupancy accompanies the flow figures.
        let columns = flow["columns"].as_array().expect("columns");
        assert_eq!(columns.len(), 5, "one per board status");
        assert!(columns.iter().all(|c| c["over_limit"] == false));
        assert_eq!(
            request
                .get(&format!("/api/plans/{plan}/flow?window_days=0"))
                .await
                .status_code(),
            422
        );

        // ── The classification map is published.
        let classes: Value = request.get("/api/flow-classes").await.json();
        assert_eq!(classes["default"]["todo"], "unnecessary_non_value_adding");
        assert_eq!(
            classes["default"]["in_review"],
            "necessary_non_value_adding"
        );
        assert_eq!(classes["finished_status"], "done");
        assert!(classes["minimum_sle_sample"].as_u64().unwrap_or(0) >= 2);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn cycle_time_and_lead_time_are_reported_separately() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request).await;
        let task = seed_task(&request, &plan, "Sat in the backlog").await;
        move_task(&request, &plan, &task, "in_progress").await;
        move_task(&request, &plan, &task, "done").await;

        let analysis: Value = request
            .get(&format!("/api/plans/{plan}/tasks/{task}/time-analysis"))
            .await
            .json();
        let a = &analysis["analysis"];
        // Both are present, and neither is labelled as the other. In a
        // test they are milliseconds apart; what is pinned is that the
        // API never returns one without the other, because quoting the
        // cycle time as delivery time is the commonest misreport in the
        // field (spec §6.1).
        assert!(a["lead_time_ms"].is_i64() && a["cycle_time_ms"].is_i64());
        assert!(a["queue_time_ms"].is_i64(), "the backlog dwell is named");
        assert!(
            a["cycle_time_ms"].as_i64().unwrap_or(0) <= a["lead_time_ms"].as_i64().unwrap_or(0)
        );
        assert!(
            analysis["note"]
                .as_str()
                .unwrap_or_default()
                .contains("different numbers"),
            "the response explains the distinction rather than assuming it"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_flow_gauges_publish_only_what_may_be_published() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let plan = seed_plan(&request).await;

        // Two tasks: below the board floor, so the plan is counted as
        // suppressed rather than labelled. `/metrics.prom` stays
        // scrapeable under enforcement, and a flow efficiency over two
        // tasks describes two people's week.
        for i in 0..2 {
            seed_task(&request, &plan, &format!("Task {i}")).await;
        }
        let set = project_portfolio_management_service::flow_metrics::refresh_once(&ctx)
            .await
            .expect("refresh");
        assert!(
            !set.rows.iter().any(|row| row.plan_pid == plan),
            "a two-task board must not be labelled: {set:?}"
        );
        assert!(set.suppressed_plans >= 1, "and it must be counted");

        // Three more clears the floor of five.
        for i in 2..5 {
            seed_task(&request, &plan, &format!("Task {i}")).await;
        }
        let set = project_portfolio_management_service::flow_metrics::refresh_once(&ctx)
            .await
            .expect("refresh");
        let row = set
            .rows
            .iter()
            .find(|row| row.plan_pid == plan)
            .expect("the plan is exported once its board clears the floor");
        assert_eq!(row.tasks, 5);
        assert_eq!(
            row.cycle_time_p85_days, None,
            "nothing has finished, so the expectation is a refusal — and the \
             gauge inherits it rather than inventing a number"
        );

        let body = project_portfolio_management_service::metrics::Metrics::global().render();
        assert!(
            body.contains(&format!(r#"ppm_flow_work_in_progress{{plan="{plan}"}}"#)),
            "missing the labelled series in: {body}"
        );
        assert!(
            !body.contains(&format!(r#"ppm_flow_cycle_time_p85_days{{plan="{plan}"}}"#)),
            "a refused p85 must not render at all: {body}"
        );
        assert!(body.contains("ppm_flow_plans_suppressed"));
        assert!(body.contains("ppm_flow_plans_dropped"));
        assert!(body.contains("ppm_flow_last_refresh_timestamp_seconds"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_forecast_samples_throughput_and_refuses_a_thin_history() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = seed_plan(&request).await;
        for i in 0..3 {
            let task = seed_task(&request, &plan, &format!("Task {i}")).await;
            move_task(&request, &plan, &task, "in_progress").await;
            move_task(&request, &plan, &task, "done").await;
        }

        // ── The default window is twelve periods, so the history is a
        // full-length but sparse sample (three completions in one
        // period, zero in the rest). That is a real sample, not a thin
        // one — it must forecast rather than refuse.
        let forecast: Value = request
            .get(&format!("/api/plans/{plan}/forecast?items=3&seed=1"))
            .await
            .json();
        assert_eq!(
            forecast["throughput_history"]
                .as_array()
                .expect("history")
                .len(),
            12
        );
        assert!(
            forecast["batch"]["p85_days"].is_number(),
            "a sparse history still forecasts: {}",
            forecast["batch"]
        );
        assert!(forecast["horizon"]["at_least_items"].is_number());

        // Nothing is open, so the default batch is zero items — which
        // takes zero days rather than being a refusal.
        let empty_batch: Value = request
            .get(&format!("/api/plans/{plan}/forecast"))
            .await
            .json();
        assert_eq!(empty_batch["open_items"], 0);
        assert_eq!(empty_batch["batch"]["p85_days"], 0.0);

        // ── A genuinely thin history refuses in both directions, with a
        // reason rather than a confident number from nothing.
        let thin: Value = request
            .get(&format!(
                "/api/plans/{plan}/forecast?items=5&history_periods=3"
            ))
            .await
            .json();
        assert_eq!(thin["batch"]["p85_days"], Value::Null);
        assert!(
            thin["batch"]["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("noise"),
            "{}",
            thin["batch"]
        );
        assert_eq!(thin["horizon"]["at_least_items"], Value::Null);
        assert!(thin["horizon"]["reason"].is_string());

        // ── The response names its input, so nobody reads it as a
        // cycle-time forecast, and states the reversed percentile
        // direction for the how-many answer.
        assert!(
            forecast["note"]
                .as_str()
                .unwrap_or_default()
                .contains("throughput history")
        );
        assert!(
            forecast["horizon"]["note"]
                .as_str()
                .unwrap_or_default()
                .contains("15th percentile")
        );

        // ── Parameter bounds are refused, not silently clamped.
        for query in [
            "period_days=0",
            "period_days=500",
            "history_periods=0",
            "history_periods=9999",
            "trials=999999999",
        ] {
            assert_eq!(
                request
                    .get(&format!("/api/plans/{plan}/forecast?{query}"))
                    .await
                    .status_code(),
                422,
                "refused: {query}"
            );
        }

        // ── The same question gives the same answer.
        let again: Value = request
            .get(&format!("/api/plans/{plan}/forecast?items=3&seed=1"))
            .await
            .json();
        assert_eq!(forecast["batch"], again["batch"]);
        assert_eq!(forecast["horizon"], again["horizon"]);
    })
    .await;
}

/// Create a plan nested under `parent`, returning its pid.
async fn seed_child(request: &axum_test::TestServer, parent: &str, name: &str) -> String {
    let created = request
        .post("/api/plans")
        .json(&json!({ "kind": "Project", "name": name, "parent_ref": parent }))
        .await;
    created.assert_status_ok();
    let plan: Value = created.json();
    plan["pid"].as_str().expect("child pid").to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded tree, the whole rollup surface
async fn the_rollup_unions_the_tree_and_keeps_the_children_comparable() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // A portfolio with two projects, one of which has a
        // sub-project — so the walk has to reach depth 2.
        let portfolio = seed_plan(&request).await;
        let alpha = seed_child(&request, &portfolio, "Alpha").await;
        let beta = seed_child(&request, &portfolio, "Beta").await;
        let alpha_sub = seed_child(&request, &alpha, "Alpha sub").await;

        // Alpha: two tasks, both finished. Beta: one task, still open.
        // Alpha-sub: one finished. The portfolio itself: one open.
        for (plan, finish, count) in [
            (&alpha, true, 2),
            (&beta, false, 1),
            (&alpha_sub, true, 1),
            (&portfolio, false, 1),
        ] {
            for i in 0..count {
                let task = seed_task(&request, plan, &format!("T{i}")).await;
                move_task(&request, plan, &task, "in_progress").await;
                if finish {
                    move_task(&request, plan, &task, "done").await;
                }
            }
        }

        let rollup: Value = request
            .get(&format!("/api/plans/{portfolio}/rollup"))
            .await
            .json();

        // The whole tree, root included.
        assert_eq!(rollup["tree"]["plans"], 4);
        assert_eq!(rollup["tree"]["max_depth"], 2);
        assert_eq!(rollup["tree"]["truncated"], false);
        assert_eq!(rollup["tree"]["revisits"], 0);

        // The combined figures are the union of every task under the
        // portfolio — five tasks, three finished.
        assert_eq!(rollup["combined"]["tasks"], 5);
        assert_eq!(rollup["combined"]["finished"], 3);
        assert_eq!(rollup["combined"]["work_in_progress"], 2);

        // Each plan is still reported on its own, so the child that
        // differs stays visible rather than being averaged away.
        let by_plan = rollup["by_plan"].as_array().expect("by_plan");
        assert_eq!(by_plan.len(), 4);
        let row = |pid: &str| {
            by_plan
                .iter()
                .find(|row| row["plan"]["pid"] == pid)
                .unwrap_or_else(|| panic!("missing {pid}"))
                .clone()
        };
        assert_eq!(row(&alpha)["tasks"], 2);
        assert_eq!(row(&alpha)["depth"], 1);
        assert_eq!(row(&alpha_sub)["depth"], 2);
        assert_eq!(row(&beta)["finished"], 0);
        assert_eq!(row(&portfolio)["depth"], 0, "the root counts too");

        // A leaf rolls up to itself.
        let leaf: Value = request
            .get(&format!("/api/plans/{beta}/rollup"))
            .await
            .json();
        assert_eq!(leaf["tree"]["plans"], 1);
        assert_eq!(leaf["combined"]["tasks"], 1);

        // A depth cap is refused out of range and reported when it fires.
        assert_eq!(
            request
                .get(&format!("/api/plans/{portfolio}/rollup?depth=0"))
                .await
                .status_code(),
            422
        );
        assert_eq!(
            request
                .get(&format!("/api/plans/{portfolio}/rollup?depth=99"))
                .await
                .status_code(),
            422
        );
        let shallow: Value = request
            .get(&format!("/api/plans/{portfolio}/rollup?depth=1"))
            .await
            .json();
        assert_eq!(shallow["tree"]["plans"], 3, "the sub-project is not walked");
        assert_eq!(shallow["tree"]["truncated"], true);
        assert!(
            shallow["tree"]["truncation_note"].is_string(),
            "a cap that fires silently reads as full coverage"
        );
        assert_eq!(
            shallow["combined"]["tasks"], 4,
            "and the combined figures cover only what was walked"
        );
    })
    .await;
}
