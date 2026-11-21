//! Request-level tests for the bulk import/export API
//! (`agents/share/bulk-import-export.md`). The config runs jobs in the
//! foreground, so a job has reached its terminal status by the time its
//! `202` returns. `#[ignore]`d: boots the app against PostgreSQL.

use axa_test::prelude::*;
use axum_test::multipart::{MultipartForm, Part};
use case_matcher::Case;
use case_service::app::App;
use case_service::bulk::{csv, jsonl, row::BulkCaseRow};
use serde_json::{Value, json};
use serial_test::serial;

fn keyed(title: &str, agency: &str, number: &str) -> BulkCaseRow {
    BulkCaseRow {
        pid: None,
        case: Case {
            agency_id: Some(agency.to_string()),
            case_number: Some(number.to_string()),
            ..Case::new(title)
        },
    }
}

fn form(bytes: Vec<u8>, format: &str, dry_run: bool) -> MultipartForm {
    let mut form = MultipartForm::new().add_text("format", format);
    if dry_run {
        form = form.add_text("dry_run", "true");
    }
    form.add_part(
        "file",
        Part::bytes(bytes).file_name(format!("import.{format}")),
    )
}

async fn import(request: &axa_test::TestServer, bytes: Vec<u8>, format: &str) -> Value {
    let r = request
        .post("/api/cases/import")
        .multipart(form(bytes, format, false))
        .await;
    assert_eq!(r.status_code(), 202, "{}", r.text());
    let id = r.json::<Value>()["job_id"].as_str().unwrap().to_string();
    request.get(&format!("/api/cases/import/{id}")).await.json()
}

async fn export(request: &axa_test::TestServer, body: &Value) -> Value {
    let r = request.post("/api/cases/export").json(body).await;
    assert_eq!(r.status_code(), 202, "{}", r.text());
    let id = r.json::<Value>()["job_id"].as_str().unwrap().to_string();
    request.get(&format!("/api/cases/export/{id}")).await.json()
}

fn artifact(status: &Value) -> Vec<u8> {
    let url = status["download_url"].as_str().expect("download_url");
    std::fs::read(url.strip_prefix("file://").expect("file:// reference")).unwrap()
}

fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

/// A keyed row creates once; the identical file again upserts in place,
/// so re-running an import never duplicates (the `(agency_id,
/// case_number)` stable key).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn keyed_import_creates_then_reimport_upserts() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let number = unique("BULK");
        let bytes = jsonl::encode(&[keyed("Bulk keyed case", "agency-x", &number)]).unwrap();
        let first = import(&request, bytes.clone(), "jsonl").await;
        assert_eq!(first["status"], "completed", "{first}");
        assert_eq!(
            (
                first["rows_created"].as_i64(),
                first["rows_upserted"].as_i64()
            ),
            (Some(1), Some(0))
        );
        let again = import(&request, bytes, "jsonl").await;
        assert_eq!(
            (
                again["rows_created"].as_i64(),
                again["rows_upserted"].as_i64()
            ),
            (Some(0), Some(1)),
            "{again}"
        );
    })
    .await;
}

/// What a JSONL and a CSV import created comes back out of the export,
/// through the artifact store. The default export is masked, so it holds
/// each case's title but not its case number; the `full` profile holds
/// both (enforcement is off here, so `full` needs no elevation).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn jsonl_and_csv_round_trip_through_export() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // Titles and case numbers are distinct tokens, so each can be
        // looked for on its own.
        let (a, b) = (unique("NUMJ"), unique("NUMC"));
        let (ta, tb) = (unique("Jsonl trip"), unique("Csv trip"));
        let s = import(
            &request,
            jsonl::encode(&[keyed(&ta, "agency-r", &a)]).unwrap(),
            "jsonl",
        )
        .await;
        assert_eq!(s["rows_created"], 1, "{s}");
        let s = import(
            &request,
            csv::encode(&[keyed(&tb, "agency-r", &b)], b',').unwrap(),
            "csv",
        )
        .await;
        assert_eq!(s["rows_created"], 1, "{s}");

        for format in ["jsonl", "csv"] {
            let masked = export(&request, &json!({ "format": format, "limit": 500 })).await;
            assert_eq!(masked["status"], "completed", "{masked}");
            let text = String::from_utf8(artifact(&masked)).unwrap();
            assert!(
                text.contains(&ta) && text.contains(&tb),
                "{format}: both titles"
            );
            assert!(
                !text.contains(&a) && !text.contains(&b),
                "{format}: case numbers are masked"
            );

            let full = export(
                &request,
                &json!({ "format": format, "limit": 500, "masking_profile": "full" }),
            )
            .await;
            assert_eq!(full["status"], "completed", "{full}");
            let text = String::from_utf8(artifact(&full)).unwrap();
            assert!(
                text.contains(&a) && text.contains(&b),
                "{format}: the full export carries case numbers"
            );
        }
    })
    .await;
}

/// A missing file and an unknown masking profile are `400`; a dry run
/// writes nothing; a repeated `Idempotency-Key` is the same job; an
/// unknown job id, and an import's id asked for as an export, are `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn bulk_refusals_dry_run_and_idempotency() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let r = request
            .post("/api/cases/import")
            .multipart(MultipartForm::new().add_text("format", "jsonl"))
            .await;
        assert_eq!(r.status_code(), 400, "{}", r.text());
        assert!(r.text().contains("missing_file"), "{}", r.text());
        let r = request
            .post("/api/cases/export")
            .json(&json!({ "format": "jsonl", "masking_profile": "partial" }))
            .await;
        assert_eq!(r.status_code(), 400, "{}", r.text());
        assert!(r.text().contains("unsupported_masking_profile"));
        let r = request
            .post("/api/cases/export")
            .json(&json!({ "format": "xml" }))
            .await;
        assert_eq!(r.status_code(), 400);

        let number = unique("DRY");
        let bytes = jsonl::encode(&[keyed("Dry run case", "agency-d", &number)]).unwrap();
        let r = request
            .post("/api/cases/import")
            .multipart(form(bytes.clone(), "jsonl", true))
            .await;
        assert_eq!(r.status_code(), 202, "{}", r.text());
        let job_id = r.json::<Value>()["job_id"].as_str().unwrap().to_string();
        let status: Value = request
            .get(&format!("/api/cases/import/{job_id}"))
            .await
            .json();
        assert_eq!(status["status"], "completed", "{status}");
        let real = import(&request, bytes.clone(), "jsonl").await;
        assert_eq!(
            real["rows_created"], 1,
            "the dry run wrote nothing, so this creates: {real}"
        );

        let key = unique("idem");
        let submit = || {
            request
                .post("/api/cases/import")
                .add_header("idempotency-key", key.as_str())
                .multipart(form(bytes.clone(), "jsonl", true))
        };
        let first = submit().await.json::<Value>()["job_id"].clone();
        assert_eq!(
            first,
            submit().await.json::<Value>()["job_id"],
            "a retried import is the same job"
        );
        let ex = || {
            request
                .post("/api/cases/export")
                .add_header("idempotency-key", key.as_str())
                .json(&json!({ "format": "jsonl" }))
        };
        let e1 = ex().await.json::<Value>()["job_id"].clone();
        assert_eq!(
            e1,
            ex().await.json::<Value>()["job_id"],
            "a retried export is the same job"
        );

        let listed: Value = request.get("/api/cases/bulk-jobs").await.json();
        assert!(listed.to_string().contains(&job_id), "{listed}");

        let unknown = uuid::Uuid::new_v4();
        for uri in [
            format!("/api/cases/import/{unknown}"),
            format!("/api/cases/export/{unknown}"),
            format!("/api/cases/export/{job_id}"),
        ] {
            assert_eq!(request.get(&uri).await.status_code(), 404, "{uri}");
        }
    })
    .await;
}
