//! Stalled journeys (aging WIP, spec `13-tasks.md` T-14j), end to end
//! against real Postgres: `GET /api/instances/stalled?idle_days=N`.
//!
//! `POST /api/care-pathways/{pathway}/instances` always stamps
//! `enrolled_on` at `Utc::now().date_naive()`, with no override — so a
//! freshly-enrolled instance's own floor activity is always "today",
//! which would mask a deliberately old segment's staleness in the
//! fold (`enrolled_on` newer than the segment is not a scenario that
//! can happen in real use, where enrolment always precedes recorded
//! activity). Each instance's `enrolled_on` is therefore backdated
//! directly on the model to before any segment this test records for
//! it — the same precedent `tests/requests/conformance.rs` set for a
//! field the live API cannot backdate. Segment timestamps themselves
//! *are* settable through the live API (`POST .../segments` takes an
//! explicit `started_at`/`ended_at`, as `tests/requests/tba.rs`'s own
//! `closed_instance` helper already relies on), so this test needs no
//! bypass for those.

use care_pathway_service::app::App;
use care_pathway_service::models::_entities::pathway_instances;
use loco_rs::TestServer;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

/// `days_ago` days before `now`, as an RFC 3339 instant — relative to
/// the real wall clock (not a fixed reference date, unlike
/// `tests/requests/tba.rs`'s own `day()` helper): this test's
/// assertions are about staleness *as of now*, so the fixture must
/// move with it.
fn days_ago(now: chrono::DateTime<chrono::Utc>, days_ago: i64) -> String {
    (now - chrono::Duration::days(days_ago)).to_rfc3339()
}

async fn seed_pathway(request: &TestServer) -> String {
    let created = request
        .post("/api/care-pathways")
        .json(&json!({
            "name": format!("stalled pathway {}", Uuid::new_v4()),
            "care_setting": "Outpatient",
            "condition_codes": [{"system": "Icd10", "code": "M54"}],
        }))
        .await;
    created.assert_status_ok();
    let template: Value = created.json();
    template["pid"].as_str().expect("pathway pid").to_string()
}

async fn enroll(request: &TestServer, pathway: &str) -> String {
    let enrolled = request
        .post(&format!("/api/care-pathways/{pathway}/instances"))
        .json(&json!({ "subject_ref": format!("person:{}", Uuid::new_v4()) }))
        .await;
    enrolled.assert_status_ok();
    let instance: Value = enrolled.json();
    instance["pid"].as_str().expect("instance pid").to_string()
}

/// Directly backdate `enrolled_on`, bypassing the always-`today` HTTP
/// enrolment endpoint (see this file's own module doc).
async fn backdate_enrolled_on(
    ctx: &AppContext,
    instance_pid: &str,
    enrolled_on: chrono::NaiveDate,
) {
    let row = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::Pid.eq(Uuid::parse_str(instance_pid).expect("pid")))
        .one(&ctx.db)
        .await
        .expect("query")
        .expect("instance exists");
    let mut active: pathway_instances::ActiveModel = row.into();
    active.enrolled_on = ActiveValue::set(enrolled_on);
    active.clock_start_at = ActiveValue::set(Some(
        enrolled_on
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
            .and_utc()
            .into(),
    ));
    active.update(&ctx.db).await.expect("update instance");
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn stalled_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pathway = seed_pathway(&request).await;
        let now = chrono::Utc::now();

        // A: last activity (a closed segment) 61 days ago.
        let a_pid = enroll(&request, &pathway).await;
        backdate_enrolled_on(
            &ctx,
            &a_pid,
            (now - chrono::Duration::days(70)).date_naive(),
        )
        .await;
        request
            .post(&format!("/api/instances/{a_pid}/segments"))
            .json(&json!({
                "label": "triage", "stage": "triage", "category": "value_adding",
                "started_at": days_ago(now, 62), "ended_at": days_ago(now, 61),
            }))
            .await
            .assert_status_ok();

        // B: an open segment started 5 days ago -- recent, not stalled.
        let b_pid = enroll(&request, &pathway).await;
        backdate_enrolled_on(
            &ctx,
            &b_pid,
            (now - chrono::Duration::days(10)).date_naive(),
        )
        .await;
        request
            .post(&format!("/api/instances/{b_pid}/segments"))
            .json(&json!({
                "label": "treatment", "stage": "treatment", "category": "value_adding",
                "started_at": days_ago(now, 5),
            }))
            .await
            .assert_status_ok();

        // C: same stale segment as A, but closed -- never listed.
        let c_pid = enroll(&request, &pathway).await;
        backdate_enrolled_on(
            &ctx,
            &c_pid,
            (now - chrono::Duration::days(70)).date_naive(),
        )
        .await;
        request
            .post(&format!("/api/instances/{c_pid}/segments"))
            .json(&json!({
                "label": "triage", "stage": "triage", "category": "value_adding",
                "started_at": days_ago(now, 62), "ended_at": days_ago(now, 61),
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{c_pid}/status"))
            .json(&json!({ "to": "completed", "outcome": "improved" }))
            .await
            .assert_status_ok();

        // Acceptance: 61 days idle is listed at idle_days=60, not 90.
        let at_60: Value = request
            .get("/api/instances/stalled?idle_days=60")
            .await
            .json();
        assert_eq!(at_60["idle_days"], 60);
        let pids_60: Vec<&str> = at_60["stalled"]
            .as_array()
            .expect("stalled array")
            .iter()
            .map(|r| r["pid"].as_str().expect("pid"))
            .collect();
        assert!(pids_60.contains(&a_pid.as_str()), "{pids_60:?}");
        assert!(!pids_60.contains(&b_pid.as_str()), "{pids_60:?}");
        assert!(
            !pids_60.contains(&c_pid.as_str()),
            "closed, never listed: {pids_60:?}"
        );

        let at_90: Value = request
            .get("/api/instances/stalled?idle_days=90")
            .await
            .json();
        let pids_90: Vec<&str> = at_90["stalled"]
            .as_array()
            .expect("stalled array")
            .iter()
            .map(|r| r["pid"].as_str().expect("pid"))
            .collect();
        assert!(
            !pids_90.contains(&a_pid.as_str()),
            "61 days is not older than 90: {pids_90:?}"
        );

        // Default idle_days is 60, echoed.
        let default: Value = request.get("/api/instances/stalled").await.json();
        assert_eq!(default["idle_days"], 60);

        // Each row names its last-activity source and idle_days.
        let a_row = at_60["stalled"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["pid"] == a_pid)
            .expect("a's row");
        assert_eq!(a_row["last_activity_source"], "segment_end");
        assert!(a_row["idle_days"].as_i64().unwrap() >= 61);
    })
    .await;
}
