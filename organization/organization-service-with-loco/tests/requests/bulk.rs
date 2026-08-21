//! Request-level integration tests for the BLK-5 bulk import/export API
//! (`agents/share/bulk-import-export.md`; crate spec §10.7).
//!
//! These boot the real loco app against the `test` environment config, so
//! they require a reachable PostgreSQL instance and are `#[ignore]`d
//! (family convention; run with `cargo test -- --ignored`).
//!
//! `config/test.yaml` sets `workers.mode: ForegroundBlocking`, so
//! `BulkJobWorker::perform_later` runs the job **synchronously** inside
//! the `POST .../import|export` handler — by the time the request
//! returns `202`, the job has already reached its terminal status. No
//! polling is needed; the very next `GET .../{id}` observes the result.

use loco_rs::testing::prelude::*;
use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization};
use organization_service::app::App;
use organization_service::bulk::{csv, jsonl};
use serde_json::json;
use serial_test::serial;

use axum_test::multipart::{MultipartForm, Part};

/// A valid LEI (ISO 7064 MOD 97-10) used across these tests — the same
/// fixture `tests/requests/organizations.rs` uses.
const VALID_LEI: &str = "5493001KJTIIGC8Y1R12";

/// Submit a multipart import: `file` = the encoded bytes, `format` =
/// the wire format token. Returns the parsed `{job_id}` response body.
async fn submit_import(
    request: &axum_test::TestServer,
    bytes: Vec<u8>,
    format: &str,
) -> serde_json::Value {
    let form = MultipartForm::new().add_text("format", format).add_part(
        "file",
        Part::bytes(bytes)
            .file_name(format!("import.{format}"))
            .mime_type("application/octet-stream"),
    );
    let response = request
        .post("/api/organizations/import")
        .multipart(form)
        .await;
    assert_eq!(
        response.status_code(),
        202,
        "import submit should be accepted"
    );
    response.json()
}

/// Submit an export request body and return the parsed `{job_id}`
/// response.
async fn submit_export(
    request: &axum_test::TestServer,
    body: &serde_json::Value,
) -> serde_json::Value {
    let response = request.post("/api/organizations/export").json(body).await;
    assert_eq!(
        response.status_code(),
        202,
        "export submit should be accepted"
    );
    response.json()
}

/// Fetch an import job's status view.
async fn import_status(request: &axum_test::TestServer, job_id: &str) -> serde_json::Value {
    request
        .get(&format!("/api/organizations/import/{job_id}"))
        .await
        .json()
}

/// Fetch an export job's status view.
async fn export_status(request: &axum_test::TestServer, job_id: &str) -> serde_json::Value {
    request
        .get(&format!("/api/organizations/export/{job_id}"))
        .await
        .json()
}

/// Read the bytes an export job wrote, by following its `download_url`
/// (a `file://` reference — the local-only artifact store, so this is a
/// direct filesystem read rather than an HTTP fetch).
fn read_artifact(download_url: &str) -> Vec<u8> {
    let path = download_url
        .strip_prefix("file://")
        .expect("local store always returns a file:// reference");
    std::fs::read(path).expect("read export artifact")
}

/// JSONL import: a fresh LEI-keyed organization creates once, and
/// re-importing the **identical** file upserts in place rather than
/// duplicating it (BLK-5 stable-key idempotency, §10.1 priority 1: LEI).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn jsonl_import_creates_then_reimport_upserts_by_lei() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let org = Organization {
            identifiers: vec![OrgIdentifier {
                scheme: IdentifierScheme::Lei,
                value: VALID_LEI.to_string(),
            }],
            ..Organization::new("Acme Reimport Co")
        };
        let bytes = jsonl::encode(&[(None, org)]).unwrap();

        // First import: one create.
        let accepted = submit_import(&request, bytes.clone(), "jsonl").await;
        let job_id = accepted["job_id"].as_str().expect("job_id");
        let status = import_status(&request, job_id).await;
        assert_eq!(status["status"], "completed", "status: {status:?}");
        assert_eq!(status["rows_total"], 1);
        assert_eq!(status["rows_created"], 1);
        assert_eq!(status["rows_upserted"], 0);

        // Re-import the identical file: upsert, not a second create.
        let accepted2 = submit_import(&request, bytes, "jsonl").await;
        let job_id2 = accepted2["job_id"].as_str().expect("job_id");
        let status2 = import_status(&request, job_id2).await;
        assert_eq!(status2["status"], "completed", "status: {status2:?}");
        assert_eq!(status2["rows_created"], 0, "re-import creates nothing");
        assert_eq!(status2["rows_upserted"], 1, "re-import upserts in place");

        // Ground truth: exactly one organization carries this LEI.
        let hits: serde_json::Value = request
            .get("/api/organizations/search?q=Acme%20Reimport%20Co")
            .await
            .json();
        assert_eq!(
            hits.as_array().unwrap().len(),
            1,
            "the stable key must not have produced a duplicate row"
        );
    })
    .await;
}

/// JSONL export: a created organization round-trips through the export
/// pipeline and encoded bytes.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn jsonl_export_round_trips_a_created_organization() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: serde_json::Value = request
            .post("/api/organizations")
            .json(&json!({"name": "Jsonl Export Co"}))
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let accepted = submit_export(
            &request,
            &json!({"format": "jsonl", "q": "Jsonl Export Co", "masking_profile": "full"}),
        )
        .await;
        let job_id = accepted["job_id"].as_str().expect("job_id");
        let status = export_status(&request, job_id).await;
        assert_eq!(status["status"], "completed", "status: {status:?}");
        assert!(status["rows_total"].as_i64().unwrap() >= 1);

        let bytes = read_artifact(status["download_url"].as_str().expect("download_url"));
        let lines = jsonl::split_lines(&bytes).unwrap();
        let found =
            lines
                .iter()
                .map(|l| jsonl::parse_line(l).unwrap())
                .any(|(_, exported_pid, org)| {
                    exported_pid.map(|p| p.to_string()) == Some(pid.clone())
                        && org.name == "Jsonl Export Co"
                });
        assert!(found, "the created organization must appear in the export");
    })
    .await;
}

/// CSV round trip: import a CSV-encoded organization (keyed by DUNS),
/// then export it back out as CSV and confirm it round-trips.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn csv_import_and_export_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let org = Organization {
            identifiers: vec![OrgIdentifier {
                scheme: IdentifierScheme::Duns,
                value: "150483782".to_string(),
            }],
            jurisdiction: Some("US".to_string()),
            ..Organization::new("Csv Roundtrip Co")
        };
        let bytes = csv::encode(&[(None, org)], b',').unwrap();

        let accepted = submit_import(&request, bytes, "csv").await;
        let job_id = accepted["job_id"].as_str().expect("job_id");
        let status = import_status(&request, job_id).await;
        assert_eq!(status["status"], "completed", "status: {status:?}");
        assert_eq!(status["rows_created"], 1, "errors: {status:?}");

        let export_accepted = submit_export(
            &request,
            &json!({"format": "csv", "q": "Csv Roundtrip Co", "masking_profile": "full"}),
        )
        .await;
        let export_job_id = export_accepted["job_id"].as_str().expect("job_id");
        let export_status_view = export_status(&request, export_job_id).await;
        assert_eq!(export_status_view["status"], "completed");

        let bytes = read_artifact(
            export_status_view["download_url"]
                .as_str()
                .expect("download_url"),
        );
        let rows = csv::decode(&bytes, b',').unwrap();
        assert!(
            rows.iter().any(|r| r
                .as_ref()
                .is_ok_and(|(_, _, org)| org.name == "Csv Roundtrip Co"
                    && org.jurisdiction.as_deref() == Some("US"))),
            "the imported organization must round-trip through the CSV export"
        );
    })
    .await;
}

/// A keyless import row (no LEI/DUNS, no explicit pid) whose name closely
/// matches an existing organization is **still created** (a bulk load
/// must never silently drop legitimate data) **and** queued in the
/// stored review queue with `provenance = "import"` (§6).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn keyless_row_with_a_likely_duplicate_creates_and_queues_for_review() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // A unique name (per test run) so this run's blocking candidates
        // are exactly the records this test creates.
        let name = format!(
            "KeylessDup {}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );

        let existing: serde_json::Value = request
            .post("/api/organizations")
            .json(&json!({"name": name}))
            .await
            .json();
        let existing_pid = existing["pid"].as_str().expect("pid").to_string();

        // The keyless row: identical name, no identifiers, no pid.
        let incoming = Organization::new(name.clone());
        let bytes = jsonl::encode(&[(None, incoming)]).unwrap();

        let accepted = submit_import(&request, bytes, "jsonl").await;
        let job_id = accepted["job_id"].as_str().expect("job_id");
        let status = import_status(&request, job_id).await;
        assert_eq!(status["status"], "completed", "status: {status:?}");
        assert_eq!(status["rows_total"], 1);
        assert_eq!(status["rows_errored"], 0, "errors: {status:?}");
        assert_eq!(
            status["rows_created"], 1,
            "the row is created, not withheld"
        );
        assert_eq!(
            status["rows_to_review"], 1,
            "the likely duplicate is queued for review"
        );

        let queued: serde_json::Value = request
            .get("/api/organizations/review-queue?status=pending")
            .await
            .json();
        let items = queued["items"].as_array().expect("items array");
        let pair = items
            .iter()
            .find(|i| {
                i["organization_id_a"] == existing_pid || i["organization_id_b"] == existing_pid
            })
            .expect("a pending pair references the existing organization");
        assert_eq!(pair["provenance"], "import");
        assert_eq!(pair["detection_method"], "import_duplicate_detection");
        assert!(
            pair["match_score"].as_f64().unwrap() >= 0.7,
            "queued pair: {pair:?}"
        );
    })
    .await;
}

/// Export masking (§8): the default (masked) profile redacts sensitive
/// fields; the privileged `full` profile leaves them intact. Every
/// export writes an audit row (SEC-B8), even the masked default.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn export_masks_by_default_and_full_is_unmasked_and_is_audited() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let name = format!(
            "MaskedExport {}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );
        let created: serde_json::Value = request
            .post("/api/organizations")
            .json(&json!({
                "name": name,
                "telephone": "+44 20 7946 0958",
                "email": "accounts@acme.example",
            }))
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        // Default (masked) export.
        let accepted = submit_export(&request, &json!({"format": "jsonl", "q": name})).await;
        let job_id = accepted["job_id"].as_str().expect("job_id").to_string();
        let status = export_status(&request, &job_id).await;
        assert_eq!(status["status"], "completed", "status: {status:?}");
        let bytes = read_artifact(status["download_url"].as_str().expect("download_url"));
        let masked = jsonl::split_lines(&bytes)
            .unwrap()
            .iter()
            .map(|l| jsonl::parse_line(l).unwrap())
            .find(|(_, exported_pid, _)| exported_pid.map(|p| p.to_string()) == Some(pid.clone()))
            .expect("exported record present")
            .2;
        assert_eq!(
            masked.telephone.as_deref(),
            Some("+** ** **** 0958"),
            "default export masks the telephone"
        );

        // Full (privileged) export — no auth enforcement in this test
        // env, so `authorize_record` is a no-op and this needs no token.
        let full_accepted = submit_export(
            &request,
            &json!({"format": "jsonl", "q": name, "masking_profile": "full"}),
        )
        .await;
        let full_job_id = full_accepted["job_id"]
            .as_str()
            .expect("job_id")
            .to_string();
        let full_status = export_status(&request, &full_job_id).await;
        let full_bytes = read_artifact(full_status["download_url"].as_str().expect("download_url"));
        let unmasked = jsonl::split_lines(&full_bytes)
            .unwrap()
            .iter()
            .map(|l| jsonl::parse_line(l).unwrap())
            .find(|(_, exported_pid, _)| exported_pid.map(|p| p.to_string()) == Some(pid.clone()))
            .expect("exported record present")
            .2;
        assert_eq!(
            unmasked.telephone.as_deref(),
            Some("+44 20 7946 0958"),
            "full export leaves the telephone unmasked"
        );

        // The audit row is written, keyed by the export job id.
        let audit: serde_json::Value = request
            .get(&format!("/api/organizations/{job_id}/audit"))
            .await
            .json();
        let entries = audit.as_array().expect("audit array");
        assert!(
            entries.iter().any(|e| e["action"] == "bulk_exported"),
            "audit: {entries:?}"
        );
    })
    .await;
}

/// `include_soft_deleted=true` is rejected at the handler — before a job
/// is ever created — rather than leaking or silently ignoring the flag
/// (deferred, §12).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn export_rejects_include_soft_deleted() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/organizations/export")
            .json(&json!({"include_soft_deleted": true}))
            .await;
        assert_eq!(response.status_code(), 400);
    })
    .await;
}

/// An unsupported format token is `400` on both import and export,
/// before any job is enqueued.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn unsupported_format_is_400() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/organizations/export")
            .json(&json!({"format": "parquet"}))
            .await;
        assert_eq!(
            response.status_code(),
            400,
            "Parquet is out of scope for BLK-5"
        );

        let form = MultipartForm::new()
            .add_text("format", "parquet")
            .add_part("file", Part::bytes(b"irrelevant".to_vec()));
        let response = request
            .post("/api/organizations/import")
            .multipart(form)
            .await;
        assert_eq!(response.status_code(), 400);
    })
    .await;
}

/// `GET .../bulk-jobs` lists recent jobs, newest first, across both
/// kinds.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn bulk_jobs_lists_recent_jobs() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let org = Organization::new("Bulk Jobs List Co");
        let bytes = jsonl::encode(&[(None, org)]).unwrap();
        let accepted = submit_import(&request, bytes, "jsonl").await;
        let job_id = accepted["job_id"].as_str().expect("job_id").to_string();

        let jobs: serde_json::Value = request.get("/api/organizations/bulk-jobs").await.json();
        let jobs = jobs.as_array().expect("jobs array");
        assert!(
            jobs.iter().any(|j| j["id"] == job_id),
            "the submitted job must appear in the recent list"
        );
    })
    .await;
}
