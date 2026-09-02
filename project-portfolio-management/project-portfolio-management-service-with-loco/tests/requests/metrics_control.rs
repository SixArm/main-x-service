//! Request tests for **Flow Distribution** (FR-31), **Total Project
//! Control** (FR-37) and the **controls register** (FR-38/39).
//!
//! Each assertion below was performed by hand against a running service
//! when the feature landed; these run it in CI instead. The ones worth
//! knowing are the *refusals* — undefined is not zero, unmeasured is
//! not a pass, unclassified is not a feature — because every one of
//! them is a number that would otherwise look measured and be wrong.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

macro_rules! create_plan {
    ($request:expr, $name:expr) => {{
        let created: Value = $request
            .post("/api/plans")
            .json(&json!({ "name": $name }))
            .await
            .json();
        created["pid"]
            .as_str()
            .or_else(|| created["plan"]["pid"].as_str())
            .expect("plan pid")
            .to_string()
    }};
}

// --------------------------------------------------- flow distribution

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Undeclared work is `unclassified` and counted **separately**, never
// folded into `feature`. Absorbing it would flatter the one share a
// reader is most likely to act on.
async fn undeclared_work_is_unclassified_not_a_feature() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Mix plan");
        for (title, flow_type) in [
            ("a feature", Some("feature")),
            ("another feature", Some("feature")),
            ("a bug", Some("defect")),
            ("nobody classified me", None),
        ] {
            let body = flow_type.map_or_else(
                || json!({ "title": title }),
                |t| json!({ "title": title, "flow_type": t }),
            );
            let created: Value = request
                .post(&format!("/api/plans/{plan}/tasks"))
                .json(&body)
                .await
                .json();
            let task = created["pid"].as_str().expect("task pid").to_string();
            // Complete it, so it counts toward the mix of finished work.
            request
                .patch(&format!("/api/plans/{plan}/tasks/{task}"))
                .json(&json!({ "status": "done" }))
                .await;
        }

        let mix: Value = request
            .get(&format!("/api/plans/{plan}/flow-distribution"))
            .await
            .json();
        let shares = mix["distribution"]["shares"].as_array().expect("shares");
        assert_eq!(
            shares.len(),
            5,
            "every type is reported, including unclassified"
        );

        let by = |name: &str| -> Value {
            shares
                .iter()
                .find(|s| s["flow_type"] == name)
                .cloned()
                .unwrap_or(Value::Null)
        };
        assert_eq!(by("unclassified")["count"], 1);
        assert_eq!(
            by("feature")["count"],
            2,
            "the undeclared task must NOT be counted as a feature"
        );
        // No declared intent ⇒ no judgement offered.
        assert_eq!(mix["distribution"]["intent_declared"], false);
        assert!(shares.iter().all(|s| s["gap_basis_points"].is_null()));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A window with nothing finished reports `null` shares, not `0%`: a
// share of nothing is undefined, and zero would read as measured.
async fn an_empty_mix_is_undefined_not_zero() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Empty mix plan");
        let mix: Value = request
            .get(&format!("/api/plans/{plan}/flow-distribution"))
            .await
            .json();
        assert_eq!(mix["distribution"]["total"], 0);
        for share in mix["distribution"]["shares"].as_array().expect("shares") {
            assert!(
                share["basis_points"].is_null(),
                "an empty window must report null, never 0%"
            );
        }
    })
    .await;
}

// ------------------------------------------------ total project control

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// DIPP = EMV / CEC, and the three refusals that keep it honest: a zero
// cost-to-complete is `null` **with a reason** rather than infinity, a
// negative cost estimate is refused, and a negative expected monetary
// value is **accepted** because a project worth less than nothing to
// finish is the case the metric exists to expose.
async fn dipp_computes_and_refuses_honestly() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "DIPP plan");
        let recorded = request
            .post(&format!("/api/plans/{plan}/tpc"))
            .json(&json!({
                "currency": "GBP",
                "expected_monetary_value": 2_000_000,
                "cost_estimate_to_complete": 1_000_000
            }))
            .await;
        assert_eq!(recorded.status_code(), 200);

        let report: Value = request
            .get(&format!("/api/plans/{plan}/tpc/report"))
            .await
            .json();
        assert_eq!(
            report["report"]["computed_dipp"], 20_000,
            "2.0 in basis points"
        );
        assert_eq!(report["report"]["band"], "at_or_above_break_even");
        assert_eq!(report["report"]["asserted"], true);

        // A negative cost estimate to complete does not exist.
        let negative_cost = request
            .post(&format!("/api/plans/{plan}/tpc"))
            .json(&json!({
                "currency": "GBP",
                "expected_monetary_value": 1,
                "cost_estimate_to_complete": -1
            }))
            .await;
        assert_eq!(negative_cost.status_code(), 422);

        // A negative EMV is legitimate and is not clamped.
        let losing = create_plan!(request, "Value-destroying plan");
        request
            .post(&format!("/api/plans/{losing}/tpc"))
            .json(&json!({
                "currency": "GBP",
                "expected_monetary_value": -500_000,
                "cost_estimate_to_complete": 1_000_000
            }))
            .await;
        let report: Value = request
            .get(&format!("/api/plans/{losing}/tpc/report"))
            .await
            .json();
        assert_eq!(report["report"]["band"], "value_destroying");

        // Nothing left to spend is the end of a project, not an
        // infinitely good one.
        let finished = create_plan!(request, "Nothing left to spend");
        request
            .post(&format!("/api/plans/{finished}/tpc"))
            .json(&json!({
                "currency": "GBP",
                "expected_monetary_value": 500_000,
                "cost_estimate_to_complete": 0
            }))
            .await;
        let report: Value = request
            .get(&format!("/api/plans/{finished}/tpc/report"))
            .await
            .json();
        assert!(report["report"]["computed_dipp"].is_null());
        assert_eq!(
            report["report"]["computed_dipp_undefined"],
            "zero_denominator"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A plan with no observation reports `unmeasured`, never a zero — the
// absent-evidence rule that runs through every metric here.
async fn a_plan_with_no_tpc_observation_is_unmeasured() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "No TPC plan");
        let report: Value = request
            .get(&format!("/api/plans/{plan}/tpc/report"))
            .await
            .json();
        assert_eq!(report["measured"], false);
        assert!(report["reason"].as_str().is_some_and(|r| !r.is_empty()));
    })
    .await;
}

// ------------------------------------------------------------- controls

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The timing decides what a failing control may do, and a control
// naming a metric the service does not produce is refused **at
// registration** — left registered it would read `unmeasured` forever,
// which is indistinguishable from passing.
async fn a_control_must_be_evaluable_and_declares_its_response() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Controls plan");

        let unknown_metric = request
            .post(&format!("/api/plans/{plan}/controls"))
            .json(&json!({
                "name": "Bogus", "timing": "concurrent",
                "metric": "invented_metric", "target_value": 10,
                "comparator": "at_least"
            }))
            .await;
        assert_eq!(unknown_metric.status_code(), 422);

        let feedforward: Value = request
            .post(&format!("/api/plans/{plan}/controls"))
            .json(&json!({
                "name": "Gate readiness", "timing": "feedforward",
                "metric": "gate_readiness", "target_value": 10_000,
                "comparator": "at_least", "cadence_days": 7
            }))
            .await
            .json();
        assert_eq!(
            feedforward["permitted_response"], "block",
            "only a feedforward control may block"
        );

        let feedback: Value = request
            .post(&format!("/api/plans/{plan}/controls"))
            .json(&json!({
                "name": "Benefits review", "timing": "feedback",
                "metric": "roi", "target_value": 0, "comparator": "at_least"
            }))
            .await
            .json();
        assert_eq!(
            feedback["permitted_response"], "record",
            "a feedback control judges finished work and may only record"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// `unmeasured` is a **third verdict**: excluded from the pass rate
// rather than counted as either half. And a failing reading with no
// action is reported as **unanswered** until one is recorded — "fix
// problems" is the fourth step of the process, so a control that only
// measures is half-built.
async fn unmeasured_is_not_a_pass_and_a_failure_stays_unanswered() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Coverage plan");
        let control: Value = request
            .post(&format!("/api/plans/{plan}/controls"))
            .json(&json!({
                "name": "Flow floor", "timing": "concurrent",
                "metric": "flow_efficiency", "target_value": 1_500,
                "comparator": "at_least"
            }))
            .await
            .json();
        let control_pid = control["pid"].as_str().expect("control pid").to_string();

        let failing: Value = request
            .post(&format!("/api/controls/{control_pid}/readings"))
            .json(&json!({ "value": 600, "method": "automatic" }))
            .await
            .json();
        assert_eq!(failing["verdict"], "fail");
        assert_eq!(failing["gap"], -900);
        let reading = failing["pid"].as_str().expect("reading pid").to_string();

        let unmeasured: Value = request
            .post(&format!("/api/controls/{control_pid}/readings"))
            .json(&json!({}))
            .await
            .json();
        assert_eq!(
            unmeasured["verdict"], "unmeasured",
            "a reading with no value is never a pass"
        );

        let before: Value = request
            .get(&format!("/api/plans/{plan}/controls/coverage"))
            .await
            .json();
        assert_eq!(before["coverage"]["unanswered_failures"], 1);
        assert_eq!(before["coverage"]["unmeasured"], 1);
        assert_eq!(
            before["coverage"]["pass_rate_basis_points"], 0,
            "one fail of one *measured* reading; the unmeasured one is excluded"
        );
        let timings = before["coverage"]["by_timing"]
            .as_array()
            .expect("by_timing");
        assert_eq!(timings.len(), 3, "every timing appears, even at zero");

        // Accepting the failure answers it.
        let accepted = request
            .post(&format!("/api/readings/{reading}/actions"))
            .json(&json!({
                "kind": "accept",
                "description": "Known gap, deferred to the next gate",
                "reason": "scheduled"
            }))
            .await;
        assert_eq!(accepted.status_code(), 200);

        let after: Value = request
            .get(&format!("/api/plans/{plan}/controls/coverage"))
            .await
            .json();
        assert_eq!(after["coverage"]["unanswered_failures"], 0);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A control that has **never produced a reading** is the most important
// number the register reports: it is indistinguishable from one that
// always passes, unless somebody counts it.
async fn a_control_that_never_fired_is_reported() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Never-read plan");
        request
            .post(&format!("/api/plans/{plan}/controls"))
            .json(&json!({
                "name": "Never read", "timing": "feedback",
                "metric": "roi", "target_value": 0, "comparator": "at_least"
            }))
            .await;
        let coverage: Value = request
            .get(&format!("/api/plans/{plan}/controls/coverage"))
            .await
            .json();
        assert_eq!(coverage["coverage"]["never_read"], 1);
        assert!(
            coverage["coverage"]["pass_rate_basis_points"].is_null(),
            "no measured readings ⇒ null, never 0%"
        );
    })
    .await;
}

// ------------------------------------------------------------ OKR

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Progress runs from the declared baseline to the target, a check-in
// moves the current value, and **the baseline never moves** — progress
// measured from a moving baseline is not progress.
async fn a_check_in_moves_the_current_value_never_the_baseline() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let objective: Value = request
            .post("/api/objectives")
            .json(&json!({ "title": "Reduce defects", "period": "2026-Q3" }))
            .await
            .json();
        let objective_pid = objective["pid"]
            .as_str()
            .expect("objective pid")
            .to_string();

        // A `decrease` key result: 100 defects down to 0.
        let kr: Value = request
            .post(&format!("/api/objectives/{objective_pid}/key-results"))
            .json(&json!({
                "title": "Open defects", "metric": "number",
                "direction": "decrease", "start_value": 100, "target_value": 0
            }))
            .await
            .json();
        let kr_pid = kr["pid"].as_str().expect("key result pid").to_string();

        // It starts at the baseline, so it is 0% done — not 100%, which
        // seeding `current_value` at zero would have reported.
        let listed: Value = request
            .get(&format!("/api/objectives/{objective_pid}/key-results"))
            .await
            .json();
        assert_eq!(listed[0]["progress_basis_points"], 0);

        let checked: Value = request
            .post(&format!("/api/key-results/{kr_pid}/check-ins"))
            .json(&json!({ "value": 25, "confidence": 40 }))
            .await
            .json();
        assert_eq!(checked["current_value"], 25);
        assert_eq!(
            checked["progress_basis_points"], 7_500,
            "100 -> 25 against a target of 0 is 75% done"
        );

        // The baseline is untouched: if it had moved to 25, progress
        // would read 0% again.
        let after: Value = request
            .get(&format!("/api/objectives/{objective_pid}/key-results"))
            .await
            .json();
        assert_eq!(after[0]["key_result"]["start_value"], 100);
        assert_eq!(after[0]["progress_basis_points"], 7_500);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// An objective with no measurable key result reports `unmeasured` and is
// **excluded** from the plan's weighted score — it must neither drag the
// plan down nor silently lift it. And a key result with nowhere to
// travel is refused at write rather than reading `unmeasured` for a
// quarter.
async fn an_unmeasured_objective_is_excluded_not_scored_zero() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "OKR plan");

        let measured: Value = request
            .post("/api/objectives")
            .json(&json!({ "title": "Measured", "period": "2026-Q3" }))
            .await
            .json();
        let measured_pid = measured["pid"].as_str().expect("pid").to_string();
        let empty: Value = request
            .post("/api/objectives")
            .json(&json!({ "title": "Nobody measured this", "period": "2026-Q3" }))
            .await
            .json();
        let empty_pid = empty["pid"].as_str().expect("pid").to_string();

        // A key result with no distance to travel is refused up front.
        let no_range = request
            .post(&format!("/api/objectives/{measured_pid}/key-results"))
            .json(&json!({
                "title": "Nowhere to go", "metric": "number",
                "direction": "increase", "start_value": 5, "target_value": 5
            }))
            .await;
        assert_eq!(no_range.status_code(), 422);

        let kr: Value = request
            .post(&format!("/api/objectives/{measured_pid}/key-results"))
            .json(&json!({
                "title": "Signups", "metric": "number",
                "direction": "increase", "start_value": 0, "target_value": 100
            }))
            .await
            .json();
        let kr_pid = kr["pid"].as_str().expect("pid").to_string();
        request
            .post(&format!("/api/key-results/{kr_pid}/check-ins"))
            .json(&json!({ "value": 100 }))
            .await;

        // Align both, weighting the *unmeasured* one far more heavily.
        // Alignment weight is 1–5 (`strategy::valid_weight`), so 5 against
        // 1 is the heaviest imbalance the contract allows — enough to
        // show that an unmeasured objective does not drag the score.
        let linked = request
            .post(&format!("/api/plans/{plan}/objectives"))
            .json(&json!({ "objective_pid": measured_pid, "weight": 1 }))
            .await;
        assert_eq!(linked.status_code(), 200);
        let linked = request
            .post(&format!("/api/plans/{plan}/objectives"))
            .json(&json!({ "objective_pid": empty_pid, "weight": 5 }))
            .await;
        assert_eq!(linked.status_code(), 200);

        let okr: Value = request.get(&format!("/api/plans/{plan}/okr")).await.json();
        assert_eq!(okr["measured"], true);
        assert_eq!(
            okr["score_basis_points"], 10_000,
            "an unmeasured objective weighted 5x must not drag the score down"
        );
        let objectives = okr["objectives"].as_array().expect("objectives");
        let unmeasured = objectives
            .iter()
            .find(|o| o["objective_pid"] == empty_pid.as_str())
            .expect("the unmeasured objective is still reported");
        assert_eq!(unmeasured["measured"], false);
        assert!(unmeasured["score_basis_points"].is_null(), "null, never 0");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A `maintain` key result needs a band — a band is what the direction
// means — and a currency-valued one must name its currency, because this
// service converts between none of them.
async fn a_key_result_must_be_measurable_to_be_declared() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let objective: Value = request
            .post("/api/objectives")
            .json(&json!({ "title": "Guarded", "period": "2026-Q3" }))
            .await
            .json();
        let pid = objective["pid"].as_str().expect("pid").to_string();

        let no_band = request
            .post(&format!("/api/objectives/{pid}/key-results"))
            .json(&json!({
                "title": "Uptime", "metric": "percent",
                "direction": "maintain", "start_value": 9_900, "target_value": 9_900
            }))
            .await;
        assert_eq!(no_band.status_code(), 422, "`maintain` needs a tolerance");

        let no_currency = request
            .post(&format!("/api/objectives/{pid}/key-results"))
            .json(&json!({
                "title": "Revenue", "metric": "currency",
                "direction": "increase", "start_value": 0, "target_value": 1_000_000
            }))
            .await;
        assert_eq!(no_currency.status_code(), 422, "currency must be named");

        let good = request
            .post(&format!("/api/objectives/{pid}/key-results"))
            .json(&json!({
                "title": "Revenue", "metric": "currency", "currency": "GBP",
                "direction": "increase", "start_value": 0, "target_value": 1_000_000
            }))
            .await;
        assert_eq!(good.status_code(), 200);
    })
    .await;
}

// -------------------------------------------- control action conversion

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// T-26: a control action converts into the work store that already
// exists (a task) rather than becoming a fifth one. The converted task
// lands on the control's own plan, in that plan's workflow-initial
// state, carrying the action's description as its title — and a
// second conversion attempt, or one on a closed action, is refused
// rather than silently creating a duplicate task.
async fn converting_a_control_action_creates_a_task_on_the_plan() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Controls conversion plan");
        let control: Value = request
            .post(&format!("/api/plans/{plan}/controls"))
            .json(&json!({
                "name": "Flow floor", "timing": "concurrent",
                "metric": "flow_efficiency", "target_value": 1_500,
                "comparator": "at_least"
            }))
            .await
            .json();
        let control_pid = control["pid"].as_str().expect("control pid").to_string();

        let failing: Value = request
            .post(&format!("/api/controls/{control_pid}/readings"))
            .json(&json!({ "value": 600, "method": "automatic" }))
            .await
            .json();
        let reading = failing["pid"].as_str().expect("reading pid").to_string();

        let action: Value = request
            .post(&format!("/api/readings/{reading}/actions"))
            .json(&json!({
                "kind": "correct",
                "description": "Rebalance the sprint to clear the flow-efficiency gap",
            }))
            .await
            .json();
        let action_pid = action["pid"].as_str().expect("action pid").to_string();

        let converted: Value = request
            .post(&format!("/api/actions/{action_pid}/convert"))
            .await
            .json();
        let task_pid = converted["task_pid"]
            .as_str()
            .expect("convert returns task_pid")
            .to_string();

        let tasks: Value = request
            .get(&format!("/api/plans/{plan}/tasks"))
            .await
            .json();
        let task = tasks["tasks"]
            .as_array()
            .expect("tasks array")
            .iter()
            .find(|t| t["pid"] == task_pid)
            .expect("the converted task is on the control's own plan");
        assert_eq!(
            task["title"], "Rebalance the sprint to clear the flow-efficiency gap",
            "the task's title is the action's description"
        );

        // A second conversion of the same action is refused, not a
        // second task.
        let again = request
            .post(&format!("/api/actions/{action_pid}/convert"))
            .await;
        assert_eq!(
            again.status_code(),
            422,
            "an already-converted action cannot be converted twice"
        );

        // An unknown action is a plain 404, not a 500.
        let unknown = request
            .post("/api/actions/00000000-0000-0000-0000-000000000000/convert")
            .await;
        assert_eq!(unknown.status_code(), 404);
    })
    .await;
}
