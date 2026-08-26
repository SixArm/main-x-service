//! Request tests for **recorded effort** (FR-28) and **utilisation**
//! (FR-35), including per person.
//!
//! The load-bearing test here is
//! [`someone_on_leave_reports_null_not_zero_percent`]: leave leaves the
//! denominator rather than sitting in it, so a person absent for the
//! whole window is *unavailable*, not *idle*. Reporting 0% there would
//! be the most defamatory number this service could publish, and it is
//! the obligation most easily lost in a refactor.
//!
//! There is deliberately **no test asserting a per-person cycle time**,
//! because no endpoint serves one: the 2026-08-25 decision narrowed the
//! family refusal to utilisation alone.
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
// Effort rolls up per plan, task and assignee, and every roll-up is
// labelled **asserted** — it is typed by a person, unlike a transition
// timestamp. Uncategorised effort stays visible rather than being
// folded into `opex`, which would flatter the capitalisable share.
async fn effort_rolls_up_and_is_labelled_asserted() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Effort plan");
        for (actor, minutes, category) in [
            (
                "person:aaaaaaaa-0000-0000-0000-000000000001",
                120,
                Some("capex"),
            ),
            (
                "person:aaaaaaaa-0000-0000-0000-000000000001",
                60,
                Some("opex"),
            ),
            ("person:aaaaaaaa-0000-0000-0000-000000000002", 30, None),
        ] {
            let body = category.map_or_else(
                || json!({ "actor_ref": actor, "spent_on": "2026-08-20", "minutes": minutes }),
                |c| {
                    json!({ "actor_ref": actor, "spent_on": "2026-08-20",
                            "minutes": minutes, "category": c, "billable": true })
                },
            );
            let created = request
                .post(&format!("/api/plans/{plan}/time-entries"))
                .json(&body)
                .await;
            assert_eq!(created.status_code(), 200);
        }

        let rollup: Value = request
            .get(&format!("/api/plans/{plan}/effort"))
            .await
            .json();
        assert_eq!(rollup["plan"]["minutes"], 210);
        assert_eq!(rollup["plan"]["capex_minutes"], 120);
        assert_eq!(rollup["plan"]["opex_minutes"], 60);
        assert_eq!(
            rollup["plan"]["unclassified_minutes"], 30,
            "uncategorised effort must stay visible"
        );
        assert_eq!(rollup["plan"]["asserted"], true);
        assert_eq!(
            rollup["by_assignee"].as_array().expect("by_assignee").len(),
            2
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A day cannot hold more than 1440 minutes, and an actor must be a real
// EntityRef. Both are refused at write rather than producing a roll-up
// nobody can reconcile.
async fn an_impossible_entry_is_refused() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Refusal plan");
        let too_long = request
            .post(&format!("/api/plans/{plan}/time-entries"))
            .json(&json!({
                "actor_ref": "person:aaaaaaaa-0000-0000-0000-000000000001",
                "spent_on": "2026-08-20", "minutes": 2_000
            }))
            .await;
        assert_eq!(too_long.status_code(), 422);

        let bad_actor = request
            .post(&format!("/api/plans/{plan}/time-entries"))
            .json(&json!({
                "actor_ref": "someone", "spent_on": "2026-08-20", "minutes": 60
            }))
            .await;
        assert_eq!(bad_actor.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// **The obligation-2 test.** Somebody on leave for the whole window
// reports `null` with a reason, **never 0%** — leave is absence of
// capacity, not failure to use it. And the figure always ships its own
// denominator, so nobody has to assume it was 100%.
async fn someone_on_leave_reports_null_not_zero_percent() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Utilisation plan");
        let worker = "person:bbbbbbbb-0000-0000-0000-000000000001";
        let absent = "person:bbbbbbbb-0000-0000-0000-000000000002";

        // A generous declared capacity, so the suppression floor is
        // cleared and the *reason* under test is leave, not smallness.
        let declared = request
            .post("/api/working-time")
            .json(&json!({ "minutes_per_day": 480, "working_days_per_week": 5 }))
            .await;
        assert_eq!(declared.status_code(), 200);

        request
            .post(&format!("/api/plans/{plan}/time-entries"))
            .json(&json!({
                "actor_ref": worker, "spent_on": "2026-08-20", "minutes": 480
            }))
            .await;

        // The absent person has no effort rows at all.
        let leave = request
            .post("/api/non-working")
            .json(&json!({
                "person_ref": absent, "starts_on": "2026-07-01",
                "ends_on": "2026-12-31", "kind": "leave"
            }))
            .await;
        assert_eq!(leave.status_code(), 200);

        let view: Value = request
            .get("/api/capacity/utilization?by=person&window_days=28")
            .await
            .json();
        let rows = view["utilisation"].as_array().expect("utilisation rows");

        let absentee = rows
            .iter()
            .find(|r| r["actor_ref"] == absent)
            .expect("a person on leave must still be reported, not silently absent");
        assert!(
            absentee["basis_points"].is_null(),
            "leave must never read as 0% utilisation"
        );
        assert_eq!(absentee["unavailable"], "all_non_working");
        assert_eq!(
            absentee["available_minutes"], 0,
            "leave left the denominator"
        );
        assert!(absentee["declared_minutes"].as_i64().unwrap_or(0) > 0);

        let present = rows
            .iter()
            .find(|r| r["actor_ref"] == worker)
            .expect("the working person is reported");
        // The figure always ships its own working.
        assert_eq!(present["effort_minutes"], 480);
        assert!(present["available_minutes"].as_i64().unwrap_or(0) > 0);
        assert_eq!(present["asserted"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// With no declared capacity at all the denominator is **unknown**, which
// is a different answer from zero and from being on leave — so it has
// its own reason.
async fn no_declared_capacity_is_not_zero_utilisation() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let view: Value = request
            .get("/api/capacity/utilization?by=team&window_days=28")
            .await
            .json();
        let utilisation = &view["utilisation"];
        assert!(utilisation["basis_points"].is_null());
        assert!(
            utilisation["unavailable"] == "no_declared_capacity"
                || utilisation["unavailable"] == "all_non_working",
            "an unknown denominator is never reported as 0%"
        );
    })
    .await;
}
