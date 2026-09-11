//! Conformance to the enrolled template (spec `13-tasks.md` T-14i),
//! end to end against real Postgres: the per-instance
//! `GET /api/instances/{pid}/time-analysis` `conformance` block, and
//! the cohort `GET /api/care-pathways/{pathway}/time-analysis`
//! `conformance` share.
//!
//! `POST /api/instances/{pid}/steps/{step}/complete` always stamps
//! `done_on` at `Utc::now().date_naive()` (§`instances.rs`), so two
//! calls within one test run land on the *same* day-resolution date
//! and cannot exercise a genuine inversion. Completion dates are
//! therefore stamped directly on the `instance_steps` row (and
//! `closed_on` directly on `pathway_instances`), bypassing the HTTP
//! layer for exactly the fields it cannot backdate -- the same
//! precedent `tests/requests/journeys_seed.rs` and
//! `tests/requests/compliance.rs` already set for a fixture the API
//! itself cannot produce.

use care_pathway_service::app::App;
use care_pathway_service::models::_entities::{instance_steps, pathway_instances};
use loco_rs::TestServer;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

async fn seed_pathway(request: &TestServer) -> String {
    let created = request
        .post("/api/care-pathways")
        .json(&json!({
            "name": format!("conformance pathway {}", Uuid::new_v4()),
            "care_setting": "Outpatient",
            "condition_codes": [{"system": "Icd10", "code": "M54"}],
        }))
        .await;
    created.assert_status_ok();
    let template: Value = created.json();
    template["pid"].as_str().expect("pathway pid").to_string()
}

/// Enrol one instance with the given step labels, returning the
/// instance pid and its steps' pids in declared position order.
async fn enroll_with_steps(
    request: &TestServer,
    pathway: &str,
    labels: &[&str],
) -> (String, Vec<String>) {
    let enrolled = request
        .post(&format!("/api/care-pathways/{pathway}/instances"))
        .json(&json!({
            "subject_ref": format!("person:{}", Uuid::new_v4()),
            "steps": labels,
        }))
        .await;
    enrolled.assert_status_ok();
    let instance: Value = enrolled.json();
    let instance_pid = instance["pid"].as_str().expect("instance pid").to_string();

    let got = request.get(&format!("/api/instances/{instance_pid}")).await;
    got.assert_status_ok();
    let body: Value = got.json();
    let mut pairs: Vec<(i64, String)> = body["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| {
            (
                s["position"].as_i64().expect("position"),
                s["pid"].as_str().expect("step pid").to_string(),
            )
        })
        .collect();
    pairs.sort_by_key(|(position, _)| *position);
    (
        instance_pid,
        pairs.into_iter().map(|(_, pid)| pid).collect(),
    )
}

/// Directly stamp one step's completion date, bypassing the
/// always-`now` HTTP endpoint (see this file's own module doc).
async fn stamp_step_done(ctx: &AppContext, step_pid: &str, done_on: chrono::NaiveDate) {
    let row = instance_steps::Entity::find()
        .filter(instance_steps::Column::Pid.eq(Uuid::parse_str(step_pid).expect("step pid")))
        .one(&ctx.db)
        .await
        .expect("query")
        .expect("step exists");
    let mut active: instance_steps::ActiveModel = row.into();
    active.done = ActiveValue::set(true);
    active.done_on = ActiveValue::set(Some(done_on));
    active.update(&ctx.db).await.expect("update step");
}

/// Directly stamp an instance's `closed_on`, bypassing the
/// always-`today` HTTP status transition.
async fn stamp_closed_on(ctx: &AppContext, instance_pid: &str, closed_on: chrono::NaiveDate) {
    let row = pathway_instances::Entity::find()
        .filter(
            pathway_instances::Column::Pid.eq(Uuid::parse_str(instance_pid).expect("instance pid")),
        )
        .one(&ctx.db)
        .await
        .expect("query")
        .expect("instance exists");
    let mut active: pathway_instances::ActiveModel = row.into();
    active.status = ActiveValue::set("completed".to_string());
    active.closed_on = ActiveValue::set(Some(closed_on));
    active.update(&ctx.db).await.expect("update instance");
}

fn day(offset: i64) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid date")
        .checked_add_signed(chrono::Duration::days(offset))
        .expect("valid date")
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one round trip over every acceptance bullet
async fn conformance_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pathway = seed_pathway(&request).await;

        // Acceptance: completing steps in template order scores 1.0
        // with zero inversions.
        let (forward_pid, forward_steps) =
            enroll_with_steps(&request, &pathway, &["consent", "assess", "treat"]).await;
        stamp_step_done(&ctx, &forward_steps[0], day(0)).await;
        stamp_step_done(&ctx, &forward_steps[1], day(1)).await;
        stamp_step_done(&ctx, &forward_steps[2], day(2)).await;
        let report: Value = request
            .get(&format!("/api/instances/{forward_pid}/time-analysis"))
            .await
            .json();
        assert_eq!(report["conformance"]["declared_pairs"], 2);
        assert_eq!(report["conformance"]["pairs_in_order"], 2);
        assert!((report["conformance"]["ratio"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        for pair in report["conformance"]["pairs"].as_array().unwrap() {
            assert_eq!(pair["verdict"], "in_order", "{pair}");
        }

        // Acceptance: reverse order scores 0.
        let (reverse_pid, reverse_steps) =
            enroll_with_steps(&request, &pathway, &["consent", "assess", "treat"]).await;
        stamp_step_done(&ctx, &reverse_steps[0], day(2)).await;
        stamp_step_done(&ctx, &reverse_steps[1], day(1)).await;
        stamp_step_done(&ctx, &reverse_steps[2], day(0)).await;
        let report: Value = request
            .get(&format!("/api/instances/{reverse_pid}/time-analysis"))
            .await
            .json();
        assert!((report["conformance"]["ratio"].as_f64().unwrap() - 0.0).abs() < 1e-9);
        for pair in report["conformance"]["pairs"].as_array().unwrap() {
            assert_eq!(pair["verdict"], "inverted", "{pair}");
        }

        // Acceptance: a skipped step is reported as skipped, not as
        // an inversion.
        let (skip_pid, skip_steps) =
            enroll_with_steps(&request, &pathway, &["consent", "assess", "treat"]).await;
        stamp_step_done(&ctx, &skip_steps[0], day(0)).await;
        // skip_steps[1] ("assess") is left undone.
        stamp_step_done(&ctx, &skip_steps[2], day(1)).await;
        let report: Value = request
            .get(&format!("/api/instances/{skip_pid}/time-analysis"))
            .await
            .json();
        assert_eq!(report["conformance"]["skipped_positions"], json!([1]));
        for pair in report["conformance"]["pairs"].as_array().unwrap() {
            assert_eq!(pair["verdict"], "skipped", "{pair}");
        }

        // Acceptance: an instance with one declared step reports
        // `null` (no pairs) with the reason.
        let (one_pid, one_steps) = enroll_with_steps(&request, &pathway, &["consent"]).await;
        stamp_step_done(&ctx, &one_steps[0], day(0)).await;
        let report: Value = request
            .get(&format!("/api/instances/{one_pid}/time-analysis"))
            .await
            .json();
        assert_eq!(report["conformance"]["declared_pairs"], 0);
        assert_eq!(report["conformance"]["ratio"], Value::Null);
        assert_eq!(
            report["conformance"]["reason"],
            "fewer than two declared steps"
        );
        assert!(
            report["conformance"]["pairs"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        // Escalation events are carried alongside the ratio, never
        // subtracted from it.
        let (escalation_pid, escalation_steps) =
            enroll_with_steps(&request, &pathway, &["consent", "assess"]).await;
        stamp_step_done(&ctx, &escalation_steps[0], day(0)).await;
        stamp_step_done(&ctx, &escalation_steps[1], day(1)).await;
        for _ in 0..2 {
            request
                .post(&format!("/api/instances/{escalation_pid}/events"))
                .json(&json!({ "kind": "escalation" }))
                .await
                .assert_status_ok();
        }
        let report: Value = request
            .get(&format!("/api/instances/{escalation_pid}/time-analysis"))
            .await
            .json();
        assert_eq!(report["conformance"]["escalation_events"], 2);
        assert!((report["conformance"]["ratio"].as_f64().unwrap() - 1.0).abs() < 1e-9);

        // A step completed after the instance's own closure is
        // flagged, independent of the ratio.
        let (closure_pid, closure_steps) =
            enroll_with_steps(&request, &pathway, &["consent", "assess"]).await;
        stamp_step_done(&ctx, &closure_steps[0], day(0)).await;
        stamp_step_done(&ctx, &closure_steps[1], day(10)).await;
        stamp_closed_on(&ctx, &closure_pid, day(5)).await;
        let report: Value = request
            .get(&format!("/api/instances/{closure_pid}/time-analysis"))
            .await
            .json();
        assert_eq!(report["conformance"]["completed_after_closure"], json!([1]));
    })
    .await;
}

/// The cohort's fully-conformant share, and that it clears the
/// suppression floor at exactly `min_cell_count` instances (default
/// 5) -- one below and this would be withheld under the same decision
/// `survival`/`split` already are.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn cohort_conformance_share_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pathway = seed_pathway(&request).await;

        // Three fully-conformant (ratio 1.0) instances.
        for _ in 0..3 {
            let (_pid, steps) = enroll_with_steps(&request, &pathway, &["a", "b"]).await;
            stamp_step_done(&ctx, &steps[0], day(0)).await;
            stamp_step_done(&ctx, &steps[1], day(1)).await;
        }
        // Two non-conformant (ratio 0.0) instances.
        for _ in 0..2 {
            let (_pid, steps) = enroll_with_steps(&request, &pathway, &["a", "b"]).await;
            stamp_step_done(&ctx, &steps[0], day(1)).await;
            stamp_step_done(&ctx, &steps[1], day(0)).await;
        }

        let report: Value = request
            .get(&format!("/api/care-pathways/{pathway}/time-analysis"))
            .await
            .json();
        assert_eq!(report["suppressed"], false, "5 clears the default floor");
        assert_eq!(report["conformance"]["instances"], 5);
        assert_eq!(report["conformance"]["with_ratio"], 5);
        assert_eq!(report["conformance"]["fully_conformant"], 3);
        assert!(
            (report["conformance"]["share"].as_f64().unwrap() - 0.6).abs() < 1e-9,
            "{}",
            report["conformance"]["share"]
        );

        // Not carried on constraints -- a documented scope decision,
        // not an omission.
        let constraints: Value = request
            .get(&format!("/api/care-pathways/{pathway}/constraints"))
            .await
            .json();
        assert!(constraints.get("conformance").is_none());
    })
    .await;
}
