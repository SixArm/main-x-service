//! Oversight bulk-read/export request tests (SEC-PPM-2): `GET
//! /api/auditor/evidence-pack` and `GET /api/auditor/trail` each write an
//! `audit_logs` row for the act of reading/exporting, not just for the
//! rows they read — the same posture `agents/share/bulk-import-export.md`
//! §8 requires of a bulk export.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::Value;
use serial_test::serial;

/// Every `action` recorded in `/api/plans/audit/recent`.
fn recorded_actions(rows: &Value) -> Vec<&str> {
    rows.as_array()
        .expect("audit rows array")
        .iter()
        .map(|r| r["action"].as_str().expect("action"))
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn evidence_pack_export_is_audited() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/auditor/evidence-pack?format=json").await;
        assert_eq!(response.status_code(), 200, "evidence pack should succeed");

        let audit: Value = request.get("/api/plans/audit/recent").await.json();
        let actions = recorded_actions(&audit);
        assert!(
            actions.contains(&"oversight_evidence_pack_exported"),
            "the export act itself should be audited, got {actions:?}"
        );
        let row = audit
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["action"] == "oversight_evidence_pack_exported")
            .expect("the exported-audit row");
        assert_eq!(row["snapshot"]["format"], "json");
        assert!(
            row["snapshot"]["from"].is_string() && row["snapshot"]["to"].is_string(),
            "the audit row should name the requested window, got {row}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn auditor_trail_read_is_audited() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/auditor/trail?limit=10").await;
        assert_eq!(response.status_code(), 200, "auditor trail should succeed");

        let audit: Value = request.get("/api/plans/audit/recent").await.json();
        let actions = recorded_actions(&audit);
        assert!(
            actions.contains(&"oversight_auditor_trail_read"),
            "the read act itself should be audited, got {actions:?}"
        );
        let row = audit
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["action"] == "oversight_auditor_trail_read")
            .expect("the read-audit row");
        assert_eq!(row["snapshot"]["limit"], 10);
    })
    .await;
}
