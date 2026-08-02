//! PPM Phase-B visibility request tests (spec/15-roadmap PPM-6/7/8/9):
//! dependencies + the portfolio schedule (violations, critical path,
//! cycle refusal), milestones, allocations + the capacity rollup,
//! saved reports (JSON + CSV), and the ETag-conditional dashboard.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// Create a plan (with an optional `kind` label + dates); returns pid.
/// The former per-collection segment is gone — all plans post to
/// `/api/plans` and carry `kind` only as a descriptive label.
async fn item(
    request: &axum_test::TestServer,
    _collection: &str,
    kind: &str,
    name: &str,
    start: Option<&str>,
    target: Option<&str>,
) -> String {
    let mut body = json!({ "kind": kind, "name": name });
    if let Some(start) = start {
        body["start_date"] = json!(start);
    }
    if let Some(target) = target {
        body["target_date"] = json!(target);
    }
    let created: Value = request.post("/api/plans").json(&body).await.json();
    created["pid"].as_str().expect("pid").to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Dependencies: self/duplicate/cycle refuse; the portfolio schedule
// reports the violation, the critical path, and undated members.
async fn dependencies_schedule_violations_and_critical_path() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let portfolio = item(
            &request,
            "portfolios",
            "Portfolio",
            "Change 2026",
            Some("2026-01"),
            Some("2026-12"),
        )
        .await;
        // Children reference their parent by parent_ref.
        let child = |name: &str, start: &str, end: &str| {
            let request = &request;
            let portfolio = portfolio.clone();
            let name = name.to_string();
            let (start, end) = (start.to_string(), end.to_string());
            async move {
                let created: Value = request
                    .post("/api/plans")
                    .json(&json!({
                        "kind": "Project", "name": name,
                        "parent_ref": portfolio,
                        "start_date": start, "target_date": end,
                    }))
                    .await
                    .json();
                created["pid"].as_str().expect("pid").to_string()
            }
        };
        let a = child("Discovery", "2026-01-01", "2026-03-31").await;
        let b = child("Build", "2026-03-01", "2026-09-30").await; // starts before a ends
        let c = child("Rollout", "2026-10-15", "2026-12-15").await;

        // Self-edge refuses; a→b and b→c create; duplicate refuses;
        // c→a would close a cycle and refuses.
        let dep = |from: &str, to: &str| json!({ "predecessor_pid": from, "successor_pid": to });
        assert_eq!(
            request
                .post("/api/dependencies")
                .json(&dep(&a, &a))
                .await
                .status_code(),
            422
        );
        request
            .post("/api/dependencies")
            .json(&dep(&a, &b))
            .await
            .assert_status_ok();
        request
            .post("/api/dependencies")
            .json(&dep(&b, &c))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post("/api/dependencies")
                .json(&dep(&a, &b))
                .await
                .status_code(),
            422,
            "duplicate edge"
        );
        assert_eq!(
            request
                .post("/api/dependencies")
                .json(&dep(&c, &a))
                .await
                .status_code(),
            422,
            "cycle"
        );

        let schedule: Value = request
            .get(&format!("/api/plans/{portfolio}/schedule"))
            .await
            .json();
        // a→b violates (b starts 2026-03-01 before a ends 2026-03-31).
        let violations = schedule["violations"].as_array().expect("violations");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0]["successor"], b.as_str());
        // The critical path runs a→b→c (the longest dependent chain).
        assert_eq!(
            schedule["critical_path"],
            json!([a.clone(), b.clone(), c.clone()])
        );
        let items = schedule["items"].as_array().expect("items");
        assert_eq!(items.len(), 4, "portfolio + three children");
        assert!(
            items
                .iter()
                .any(|i| i["pid"] == a.as_str() && i["on_critical_path"] == true),
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Milestones: due-order list with overdue flags; complete clears.
async fn milestones_flag_overdue_until_completed() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = item(&request, "projects", "Project", "Milestoned", None, None).await;
        request
            .post(&format!("/api/plans/{pid}/milestones"))
            .json(&json!({ "name": "Design signed off", "due": "2026-01-31" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/plans/{pid}/milestones"))
            .json(&json!({ "name": "Go-live", "due": "2099-12-31" }))
            .await
            .assert_status_ok();
        let listed: Value = request
            .get(&format!("/api/plans/{pid}/milestones"))
            .await
            .json();
        let listed = listed.as_array().expect("milestones");
        assert_eq!(listed[0]["name"], "Design signed off", "due order");
        assert_eq!(listed[0]["overdue"], true);
        assert_eq!(listed[1]["overdue"], false);
        let m_pid = listed[0]["pid"].as_str().expect("m pid");
        request
            .post(&format!("/api/plans/{pid}/milestones/{m_pid}/complete"))
            .await
            .assert_status_ok();
        let after: Value = request
            .get(&format!("/api/plans/{pid}/milestones"))
            .await
            .json();
        assert_eq!(after[0]["done"], true);
        assert_eq!(after[0]["overdue"], false, "done clears overdue");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Allocations sum per person across items; over 100% flags in the
// capacity rollup; bad percent / URN refuse.
async fn capacity_flags_over_allocation() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let one = item(&request, "projects", "Project", "Alloc One", None, None).await;
        let two = item(&request, "products", "Product", "Alloc Two", None, None).await;
        let person = format!("worker:{}", uuid::Uuid::new_v4());
        assert_eq!(
            request
                .post(&format!("/api/plans/{one}/allocations"))
                .json(&json!({ "person_ref": "bob", "percent": 50 }))
                .await
                .status_code(),
            422,
            "bare name is not a URN"
        );
        assert_eq!(
            request
                .post(&format!("/api/plans/{one}/allocations"))
                .json(&json!({ "person_ref": person, "percent": 120 }))
                .await
                .status_code(),
            422
        );
        request
            .post(&format!("/api/plans/{one}/allocations"))
            .json(&json!({ "person_ref": person, "percent": 60, "role": "engineer" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/plans/{two}/allocations"))
            .json(&json!({ "person_ref": person, "percent": 70 }))
            .await
            .assert_status_ok();
        let rollup: Value = request.get("/api/capacity").await.json();
        let row = rollup["people"]
            .as_array()
            .expect("people")
            .iter()
            .find(|p| p["person_ref"] == person.as_str())
            .expect("our person")
            .clone();
        assert_eq!(row["allocated_percent"], 130);
        assert_eq!(row["over_allocated"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Saved reports run synchronously as JSON and CSV with filters and
// field projection; unknown fields refuse at save time.
async fn reports_filter_project_and_render_csv() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        item(
            &request,
            "projects",
            "Project",
            "Alpha, phase 1",
            None,
            Some("2026-06"),
        )
        .await;
        item(&request, "projects", "Project", "Beta", None, None).await;
        assert_eq!(
            request
                .post("/api/reports")
                .json(&json!({ "name": "bad", "collection": "projects", "fields": ["salary"] }))
                .await
                .status_code(),
            422
        );
        let saved: Value = request
            .post("/api/reports")
            .json(&json!({
                "name": "Alpha search",
                "collection": "projects",
                "filters": { "name_like": "alpha" },
                "fields": ["name", "target_date"],
            }))
            .await
            .json();
        let report = saved["pid"].as_str().expect("report pid");
        let ran: Value = request
            .get(&format!("/api/reports/{report}/run"))
            .await
            .json();
        assert_eq!(ran["rows"], 1);
        assert_eq!(ran["data"][0]["name"], "Alpha, phase 1");
        assert_eq!(ran["data"][0]["target_date"], "2026-06");
        let csv = request
            .get(&format!("/api/reports/{report}/run?format=csv"))
            .await;
        assert!(
            csv.headers()
                .get("content-type")
                .is_some_and(|v| v == "text/csv")
        );
        let text = csv.text();
        assert!(text.starts_with("name,target_date\n"));
        assert!(
            text.contains("\"Alpha, phase 1\",2026-06"),
            "comma-bearing name is quoted: {text}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The dashboard: per-collection RAG + stage rollups, site tiles, and
// the ETag conditional cycle (304 until state changes).
async fn dashboard_rolls_up_and_is_conditional() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // A red item (overdue target), a green one, and a materialised
        // risk elsewhere.
        let red = item(
            &request,
            "projects",
            "Project",
            "Late project",
            Some("2025-01-01"),
            Some("2025-12-31"),
        )
        .await;
        item(
            &request,
            "projects",
            "Project",
            "Fine project",
            None,
            Some("2099-12"),
        )
        .await;
        let risky = item(
            &request,
            "programs",
            "Program",
            "Risky programme",
            None,
            None,
        )
        .await;
        request
            .post(&format!("/api/plans/{risky}/risks"))
            .json(&json!({ "title": "Went wrong", "probability": 4, "impact": 4 }))
            .await
            .assert_status_ok();
        let risks: Value = request
            .get(&format!("/api/plans/{risky}/risks"))
            .await
            .json();
        let risk_pid = risks[0]["pid"].as_str().expect("risk pid");
        request
            .post(&format!("/api/plans/{risky}/risks/{risk_pid}/escalate"))
            .await
            .assert_status_ok();

        let first = request.get("/api/at-a-glance").await;
        assert_eq!(first.status_code(), 200);
        let etag = first
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .expect("dashboard ETag")
            .to_string();
        let glance: Value = first.json();
        let projects = glance["collections"]
            .as_array()
            .expect("collections")
            .iter()
            .find(|c| c["collection"] == "Project")
            .expect("Project rollup")
            .clone();
        assert!(
            projects["rag"]["red"].as_u64().unwrap_or(0) >= 1,
            "{projects}"
        );
        assert!(projects["rag"]["green"].as_u64().unwrap_or(0) >= 1);
        assert!(
            glance["site_tiles"]["materialised_risks"]
                .as_u64()
                .unwrap_or(0)
                >= 1
        );

        // Unchanged ⇒ 304; a new gate review changes the fingerprint.
        let unchanged = request
            .get("/api/at-a-glance")
            .add_header("if-none-match", etag.clone())
            .await;
        assert_eq!(unchanged.status_code(), 304);
        request
            .post(&format!("/api/plans/{red}/gate-reviews"))
            .json(&json!({ "gate": "g0_concept", "decision": "approved" }))
            .await
            .assert_status_ok();
        let changed = request
            .get("/api/at-a-glance")
            .add_header("if-none-match", etag)
            .await;
        assert_eq!(changed.status_code(), 200, "stage change re-sends");
    })
    .await;
}
