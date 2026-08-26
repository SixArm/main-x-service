//! Request tests for **sprint ceremonies** (FR-29), **realized gains**
//! (FR-33) and **strategic performance** (FR-34 / FR-36).
//!
//! The assertions are chosen for the refusals, which is where these
//! features are most easily made to lie: a rewritable commitment, an
//! adoption rate with no denominator, an unmeasured plan reported as a
//! total loss, an NPS without its response count.
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
        created["pid"].as_str().expect("plan pid").to_string()
    }};
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The commitment snapshot is written once and **names** what changed
// afterwards. Without it, scope added mid-sprint is indistinguishable
// from scope committed at the outset — the sprint simply looks like it
// was always that size.
async fn a_commitment_is_written_once_and_names_later_scope_change() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Ceremony plan");
        let sprint: Value = request
            .post(&format!("/api/plans/{plan}/sprints"))
            .json(
                &json!({ "name": "Sprint 1", "starts_on": "2026-08-01", "ends_on": "2026-08-14" }),
            )
            .await
            .json();
        let sprint_pid = sprint["pid"].as_str().expect("sprint pid").to_string();

        let first: Value = request
            .post(&format!("/api/plans/{plan}/tasks"))
            .json(&json!({ "title": "committed work", "sprint_pid": sprint_pid }))
            .await
            .json();
        assert!(first["pid"].as_str().is_some());

        let committed = request
            .post(&format!("/api/sprints/{sprint_pid}/commit"))
            .await;
        assert_eq!(committed.status_code(), 200);

        // A second commit is refused: a rewritable commitment is not one.
        let again = request
            .post(&format!("/api/sprints/{sprint_pid}/commit"))
            .await;
        assert_eq!(again.status_code(), 422);

        // Scope added afterwards is named, not merely counted.
        request
            .post(&format!("/api/plans/{plan}/tasks"))
            .json(&json!({ "title": "added mid-sprint", "sprint_pid": sprint_pid }))
            .await;

        let view: Value = request
            .get(&format!("/api/sprints/{sprint_pid}/commitment"))
            .await
            .json();
        assert_eq!(view["was_committed"], true);
        assert_eq!(view["committed"], 1);
        assert_eq!(view["current"], 2);
        assert_eq!(
            view["added_after_commitment"]
                .as_array()
                .expect("added")
                .len(),
            1,
            "the added task must be named, so the change reads as a change"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// All four ceremonies are reportable, every kind appears even at zero,
// and a second planning or review is refused — that is a re-plan, which
// is a new sprint.
async fn ceremonies_report_every_kind_and_refuse_a_second_planning() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Ceremony kinds plan");
        let sprint: Value = request
            .post(&format!("/api/plans/{plan}/sprints"))
            .json(&json!({ "name": "S", "starts_on": "2026-08-01", "ends_on": "2026-08-14" }))
            .await
            .json();
        let sprint_pid = sprint["pid"].as_str().expect("sprint pid").to_string();

        let planned = request
            .post(&format!("/api/sprints/{sprint_pid}/ceremonies"))
            .json(&json!({ "kind": "planning" }))
            .await;
        assert_eq!(planned.status_code(), 200);
        let twice = request
            .post(&format!("/api/sprints/{sprint_pid}/ceremonies"))
            .json(&json!({ "kind": "planning" }))
            .await;
        assert_eq!(twice.status_code(), 422);

        // A daily can be held many times; it is not a boundary.
        for _ in 0..2 {
            let daily = request
                .post(&format!("/api/sprints/{sprint_pid}/ceremonies"))
                .json(&json!({ "kind": "daily" }))
                .await;
            assert_eq!(daily.status_code(), 200);
        }

        let view: Value = request
            .get(&format!("/api/sprints/{sprint_pid}/ceremonies"))
            .await
            .json();
        let held = view["held"].as_array().expect("held");
        assert_eq!(held.len(), 4, "every kind reported, even at zero");
        let retro = held
            .iter()
            .find(|h| h["kind"] == "retrospective")
            .expect("retrospective row");
        assert_eq!(
            retro["count"], 0,
            "a sprint that never retrospected is a finding, not a missing row"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// **A plan with no value points is `unrealized`, never a total loss.**
// It has not failed to deliver; it has not been measured, and those are
// different findings.
async fn a_plan_with_no_value_is_unrealized_not_a_loss() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Unrealized plan");
        let view: Value = request
            .get(&format!("/api/plans/{plan}/value-realization"))
            .await
            .json();
        let roi = &view["transformation_roi"];
        assert!(roi["basis_points"].is_null());
        assert_eq!(roi["absent"], "unrealized");
        assert_eq!(view["time_to_value"]["absent"], "unrealized");
        assert_eq!(view["adoption"]["measured"], false);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// An adoption rate needs a denominator and its own definition — "active
// user" is the term most easily redefined between two readings, so it is
// refused at write rather than divided at read.
async fn adoption_needs_a_denominator_and_a_definition() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Adoption plan");

        let no_target = request
            .post(&format!("/api/plans/{plan}/adoption"))
            .json(&json!({
                "active_users": 10, "target_users": 0,
                "window_days": 30, "definition": "signed in"
            }))
            .await;
        assert_eq!(no_target.status_code(), 422);

        let no_definition = request
            .post(&format!("/api/plans/{plan}/adoption"))
            .json(&json!({
                "active_users": 10, "target_users": 100,
                "window_days": 30, "definition": "  "
            }))
            .await;
        assert_eq!(no_definition.status_code(), 422);

        let good = request
            .post(&format!("/api/plans/{plan}/adoption"))
            .json(&json!({
                "active_users": 25, "target_users": 100,
                "window_days": 30, "definition": "signed in within the window"
            }))
            .await;
        assert_eq!(good.status_code(), 200);

        let view: Value = request
            .get(&format!("/api/plans/{plan}/value-realization"))
            .await
            .json();
        let adoption = &view["adoption"];
        assert_eq!(adoption["basis_points"], 2_500);
        // Returned with the rate, so it can be compared across readings.
        assert_eq!(adoption["definition"], "signed in within the window");
        assert_eq!(adoption["window_days"], 30);
        assert_eq!(adoption["target_users"], 100);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The Time-to-Value clock stops **once**: a second first-measurable
// value point is refused, because a clock that can restart is not a
// measurement.
async fn the_time_to_value_clock_stops_once() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Clock plan");
        request
            .post(&format!("/api/plans/{plan}/business-case"))
            .json(&json!({ "metric": "savings", "baseline_value": 0, "target_value": 1_000_000 }))
            .await;

        let first = request
            .post(&format!("/api/plans/{plan}/value-points"))
            .json(&json!({ "value": 250_000, "method": "measured", "is_first_measurable": true }))
            .await;
        assert_eq!(first.status_code(), 200);

        let second = request
            .post(&format!("/api/plans/{plan}/value-points"))
            .json(&json!({ "value": 100_000, "method": "measured", "is_first_measurable": true }))
            .await;
        assert_eq!(second.status_code(), 422, "the clock stops once");

        // A later, ordinary value point is fine.
        let ordinary = request
            .post(&format!("/api/plans/{plan}/value-points"))
            .json(&json!({ "value": 100_000, "method": "estimated" }))
            .await;
        assert_eq!(ordinary.status_code(), 200);

        let view: Value = request
            .get(&format!("/api/plans/{plan}/value-realization"))
            .await
            .json();
        // The evidence mix is disclosed: half measured, half estimated.
        assert_eq!(view["transformation_roi"]["realized_minor"], 350_000);
        assert_eq!(
            view["transformation_roi"]["measured_share_basis_points"],
            5_000
        );
        assert_eq!(view["time_to_value"]["observations"], 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// NPS always carries its response count — 100 from two respondents is
// not a finding — and SPI/CPI without a baseline report `null` **with a
// reason**, never 1.0, which would say "exactly on plan".
async fn nps_carries_its_count_and_no_baseline_is_never_on_plan() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Performance plan");

        let empty: Value = request
            .get(&format!("/api/plans/{plan}/performance"))
            .await
            .json();
        assert!(empty["stakeholder"]["nps"]["score"].is_null());
        assert_eq!(empty["stakeholder"]["nps"]["absent"], "no_responses");
        assert!(empty["schedule"]["spi"].is_null());
        assert_eq!(empty["schedule"]["absent"], "no_baseline");

        for (score, role) in [(10, "sponsor"), (9, "user"), (3, "user")] {
            let recorded = request
                .post(&format!("/api/plans/{plan}/satisfaction"))
                .json(&json!({ "instrument": "nps", "score": score, "respondent_role": role }))
                .await;
            assert_eq!(recorded.status_code(), 200);
        }

        let view: Value = request
            .get(&format!("/api/plans/{plan}/performance"))
            .await
            .json();
        let nps = &view["stakeholder"]["nps"];
        assert_eq!(nps["responses"], 3, "the count always ships with the score");
        assert_eq!(nps["promoters"], 2);
        assert_eq!(nps["detractors"], 1);
        // 66% promoters − 33% detractors = 33.
        assert_eq!(nps["score"], 33);
    })
    .await;
}
