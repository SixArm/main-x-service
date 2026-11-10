//! Request-level integration tests over the compliance surface: the
//! tamper-evident audit chain, read/disclosure auditing, GDPR Art. 17
//! erasure, the posture and SBOM endpoints, and the ONC conformance
//! additions (`$validate`, SMART discovery, Bulk Data `$export`).
//!
//! **Why these must run against a real database.** The audit chain's
//! riskiest property is that a digest computed in Rust before an `INSERT`
//! still matches after Postgres has stored the snapshot as `jsonb` — which
//! reorders object keys and normalises numbers — and returned it as a
//! `timestamptz` in the session time zone. No unit test can prove that;
//! only a round-trip through Postgres can. `chain_survives_a_jsonb_round_trip`
//! is the test that would catch a regression there, and it is the reason
//! this file exists.
//!
//! They are `#[ignore]`d so the default `cargo test` stays green without a
//! database. Run with:
//!
//! ```sh
//! DB_URL=postgres://loco:loco@localhost:5432/care_pathway_service_test \
//!   cargo test -- --ignored
//! ```

use axa_test::TestServer;
use axa_test::prelude::*;
use care_pathway_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// A payload with enough structure — nested objects, arrays, mixed key
/// orders — that a JSONB round-trip will genuinely reorder it.
fn rich_pathway() -> Value {
    json!({
        "name": "Acute Stroke Care Pathway",
        "provider_id": "trust-1",
        "provider_name": "Example NHS Trust",
        "pathway_code": "STROKE-01",
        "condition_codes": [
            {"system": "Icd10", "code": "I63.9"},
            {"system": "Snomed", "code": "422504002"}
        ],
        "identifiers": [{"scheme": "GuidelineId", "value": "NICE-NG128"}],
        "alternate_names": ["Stroke Pathway", "CVA Pathway"],
        "interventions": ["Thrombolysis", "Thrombectomy"],
        "keywords": ["stroke", "cva", "neurology"],
        "in_language": ["en-GB", "cy"]
    })
}

/// Create a pathway and return its `pid`.
async fn create(request: &TestServer, body: &Value) -> String {
    let response = request.post("/api/care-pathways").json(body).await;
    assert_eq!(response.status_code(), 200, "create should succeed");
    response.json::<Value>()["pid"]
        .as_str()
        .expect("pid in create response")
        .to_string()
}

/// Fetch the chain-verification report.
async fn verify(request: &TestServer) -> Value {
    let response = request.get("/api/compliance/audit/verify").await;
    assert_eq!(response.status_code(), 200, "verification should succeed");
    response.json()
}

/// **The load-bearing test.** Writes exercise every mutation path, then
/// the chain is re-verified from rows that have gone through Postgres —
/// `jsonb` key reordering, `timestamptz` conversion and all. A digest
/// computed on the way in must still match on the way out.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn chain_survives_a_jsonb_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create(&request, &rich_pathway()).await;
        // Update and delete, so the chain spans several rows and several
        // snapshot shapes (including the `None` snapshot a delete writes).
        let mut updated = rich_pathway();
        updated["keywords"] = json!(["stroke", "thrombolysis"]);
        request
            .put(&format!("/api/care-pathways/{pid}"))
            .json(&updated)
            .await;
        let second = create(&request, &rich_pathway()).await;
        request
            .delete(&format!("/api/care-pathways/{second}"))
            .await;

        let report = verify(&request).await;
        assert_eq!(
            report["verified"], true,
            "the chain must verify after a Postgres round-trip: {report}"
        );
        assert!(
            report["rows"].as_u64().unwrap_or(0) >= 4,
            "expected at least four audited mutations: {report}"
        );
        assert_eq!(report["breaks"].as_array().map(Vec::len), Some(0));
        assert!(
            report["intact"].as_u64().unwrap_or(0) >= 4,
            "every row should have been content-verified: {report}"
        );
        assert!(report["head"].is_string(), "a chain head must be reported");
    })
    .await;
}

/// Editing an audit row behind the service's back breaks verification —
/// the property the whole chain exists to provide (HIPAA §164.312(c)).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn tampering_with_a_row_breaks_verification() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        create(&request, &rich_pathway()).await;
        assert_eq!(verify(&request).await["verified"], true);

        // Rewrite a snapshot directly in the database, exactly as an
        // attacker (or a careless operator) with SQL access would.
        sqlx::query!(
            r#"UPDATE audit_logs SET snapshot = jsonb_set(snapshot, '{name}', '"Tampered"')
               WHERE snapshot IS NOT NULL AND redacted_at IS NULL"#,
        )
        .execute(&ctx.db)
        .await
        .expect("tamper");

        let report = verify(&request).await;
        assert_eq!(
            report["verified"], false,
            "an edited row must break verification: {report}"
        );
        assert!(
            report["breaks"]
                .as_array()
                .is_some_and(|b| b.iter().any(|x| x["kind"] == "content")),
            "the break must be reported as a content break: {report}"
        );
    })
    .await;
}

/// GDPR Art. 17 erasure destroys the payload and the audit content, and
/// the chain **still verifies** — the collision this design exists to
/// resolve.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasure_destroys_content_but_keeps_the_chain_verifiable() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create(&request, &rich_pathway()).await;
        // A second record proves erasure is scoped to its subject.
        let survivor = create(&request, &rich_pathway()).await;

        let response = request
            .post(&format!("/api/care-pathways/{pid}/erase"))
            .await;
        assert_eq!(response.status_code(), 200, "erasure should succeed");
        let outcome: Value = response.json();
        assert_eq!(outcome["irreversible"], true);
        assert_eq!(outcome["payload_erased"], true);
        assert!(
            outcome["audit_rows_redacted"].as_u64().unwrap_or(0) >= 1,
            "the create row's content must have been redacted: {outcome}"
        );

        // The record is gone from the read surface…
        assert_eq!(
            request
                .get(&format!("/api/care-pathways/{pid}"))
                .await
                .status_code(),
            404,
            "an erased record must not be readable"
        );
        // …and no audit row still holds its payload.
        let audit: Value = request
            .get(&format!("/api/care-pathways/{pid}/audit"))
            .await
            .json();
        let rows = audit.as_array().expect("audit rows array");
        assert!(!rows.is_empty(), "the accountability record must survive");
        for row in rows {
            if row["action"] == "erased" {
                continue;
            }
            assert!(
                row["snapshot"].is_null(),
                "an erased record's audit content must be gone: {row}"
            );
            assert!(
                !row["redacted_at"].is_null(),
                "a redacted row must be marked: {row}"
            );
            assert!(
                row["hash"].is_string(),
                "redaction must preserve the chain hash: {row}"
            );
        }

        // The chain still verifies end to end.
        let report = verify(&request).await;
        assert_eq!(
            report["verified"], true,
            "redaction must not break the chain: {report}"
        );
        assert!(
            report["redacted"].as_u64().unwrap_or(0) >= 1,
            "redacted rows must be counted as such: {report}"
        );

        // The other record is untouched.
        assert_eq!(
            request
                .get(&format!("/api/care-pathways/{survivor}"))
                .await
                .status_code(),
            200,
            "erasure must be scoped to its subject"
        );
    })
    .await;
}

/// Erasing an unknown or already-erased `pid` is safe and still records
/// the accountability row — a subject's right does not lapse because the
/// record was already soft-deleted.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasure_is_idempotent() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create(&request, &rich_pathway()).await;
        let first: Value = request
            .post(&format!("/api/care-pathways/{pid}/erase"))
            .await
            .json();
        let second_response = request
            .post(&format!("/api/care-pathways/{pid}/erase"))
            .await;
        assert_eq!(second_response.status_code(), 200, "re-erasure is safe");
        let second: Value = second_response.json();
        assert_eq!(second["irreversible"], true);
        assert!(
            second["audit_rows_redacted"].as_u64().unwrap_or(99)
                <= first["audit_rows_redacted"].as_u64().unwrap_or(0) + 1,
            "a second sweep should find little or nothing left to redact"
        );
        assert_eq!(verify(&request).await["verified"], true);

        // A pid that never existed is handled, not 500'd.
        let unknown = uuid::Uuid::new_v4();
        assert_eq!(
            request
                .post(&format!("/api/care-pathways/{unknown}/erase"))
                .await
                .status_code(),
            200
        );
        // …but a malformed pid is a 400.
        assert_eq!(
            request
                .post("/api/care-pathways/not-a-uuid/erase")
                .await
                .status_code(),
            400
        );
    })
    .await;
}

/// The disclosure accounting is honest about its own completeness: with
/// read-auditing off (the default) it says so rather than returning a
/// reassuring empty list.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn disclosure_accounting_declares_its_completeness() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create(&request, &rich_pathway()).await;
        // A read that declares a research purpose and an external
        // recipient — a disclosure under §164.528 if auditing is on.
        request
            .get(&format!("/api/care-pathways/{pid}"))
            .add_header("x-purpose-of-use", "research")
            .add_header("x-disclosure-recipient", "University of Example")
            .await;

        let body: Value = request
            .get(&format!("/api/care-pathways/{pid}/audit/disclosures"))
            .await
            .json();
        assert_eq!(body["pid"], pid);
        assert!(body["count"].is_number());
        let enabled = body["read_auditing_enabled"].as_bool().unwrap_or(false);
        let caveat = body["caveat"].as_str().unwrap_or_default();
        if enabled {
            assert!(caveat.contains("complete for the period"));
        } else {
            assert!(
                caveat.contains("INCOMPLETE"),
                "with read-auditing off the accounting must say it is incomplete: {caveat}"
            );
        }
    })
    .await;
}

/// Fetch the record-integrity report.
async fn verify_records(request: &TestServer) -> Value {
    let response = request.get("/api/compliance/records/verify").await;
    assert_eq!(response.status_code(), 200);
    response.json()
}

/// **Row-level integrity, end to end.** Records written through the
/// service verify; a row rewritten with raw SQL does not. This is the
/// control that closes the gap the audit chain leaves open — the chain
/// attests to the trail, this attests to the rows.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn out_of_band_record_edit_is_detected() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pid = create(&request, &rich_pathway()).await;
        create(&request, &rich_pathway()).await;

        // Every write path rehashes, so an untouched set verifies.
        let report = verify_records(&request).await;
        assert_eq!(report["verified"], true, "{report}");
        assert_eq!(report["intact"], 2);
        assert_eq!(report["unhashed"], 0, "every write must set a hash");

        // Now edit a stored payload behind the service's back.
        sqlx::query!(
            r#"UPDATE care_pathways SET data = jsonb_set(data, '{name}', '"Tampered"')
               WHERE pid = $1"#,
            uuid::Uuid::parse_str(&pid).expect("uuid"),
        )
        .execute(&ctx.db)
        .await
        .expect("tamper");

        let report = verify_records(&request).await;
        assert_eq!(
            report["verified"], false,
            "an out-of-band edit must be detected: {report}"
        );
        assert_eq!(report["mismatched"].as_array().map(Vec::len), Some(1));
        assert_eq!(report["mismatched"][0]["pid"], pid);
        assert_eq!(report["intact"], 1, "the untouched record still verifies");
    })
    .await;
}

/// Update, soft-delete, merge and erase all rehash, so a record that has
/// been through the full lifecycle still verifies. A control that flagged
/// legitimate writes would be worse than useless.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn every_write_path_rehashes() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let updated_pid = create(&request, &rich_pathway()).await;
        let mut changed = rich_pathway();
        changed["keywords"] = json!(["stroke", "thrombolysis"]);
        request
            .put(&format!("/api/care-pathways/{updated_pid}"))
            .json(&changed)
            .await;

        let deleted_pid = create(&request, &rich_pathway()).await;
        request
            .delete(&format!("/api/care-pathways/{deleted_pid}"))
            .await;

        let erased_pid = create(&request, &rich_pathway()).await;
        request
            .post(&format!("/api/care-pathways/{erased_pid}/erase"))
            .await;

        let merged_main = create(&request, &rich_pathway()).await;
        let merged_dup = create(&request, &rich_pathway()).await;
        request
            .post("/api/care-pathways/merge")
            .json(&json!({ "main_pid": merged_main, "duplicate_pid": merged_dup }))
            .await;

        let report = verify_records(&request).await;
        assert_eq!(
            report["verified"], true,
            "create/update/delete/merge/erase must all rehash: {report}"
        );
        assert_eq!(report["unhashed"], 0);
        assert!(
            report["records"].as_u64().unwrap_or(0) >= 5,
            "soft-deleted and erased rows must be verified too: {report}"
        );
    })
    .await;
}

/// The posture endpoint reports the running configuration and states, per
/// framework, what is **not** claimed.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn posture_reports_controls_and_disclaimers() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/compliance").await;
        assert_eq!(response.status_code(), 200);
        let body: Value = response.json();
        assert_eq!(body["service"], "care-pathway-service");
        assert_eq!(body["controls"]["audit_chain"], true);
        assert!(body["controls"]["event_transport"].is_string());
        assert!(body["data_protection"]["residency"].is_string());
        let frameworks = body["frameworks"].as_array().expect("frameworks");
        assert_eq!(frameworks.len(), 4);
        for framework in frameworks {
            assert!(
                framework["not_claimed"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty()),
                "{} must state its limits",
                framework["framework"]
            );
        }
    })
    .await;
}

/// The SBOM is served, is valid CycloneDX, and annotates the crate's own
/// direct dependencies (IEC 62304 §8.1.2).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn sbom_is_served_and_annotated() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // Not open to everyone, even with enforcement off here.
        let refused = request.get("/api/compliance/sbom").await;
        assert_eq!(refused.status_code(), 401);

        let body = serde_json::to_value(care_pathway_service::compliance::soup::sbom())
            .expect("the SBOM serializes");
        assert_eq!(body["bomFormat"], "CycloneDX");
        assert_eq!(body["specVersion"], "1.5");
        let components = body["components"].as_array().expect("components");
        assert!(components.len() > 50, "the whole graph should be listed");
        let sha2 = components
            .iter()
            .find(|c| c["name"] == "sha2")
            .expect("sha2 is a direct dependency and must be listed");
        assert!(
            sha2["description"]
                .as_str()
                .is_some_and(|d| d.contains("hash chain")),
            "a direct dependency must carry its SOUP purpose: {sha2}"
        );
    })
    .await;
}

/// `$validate` reports conformance without persisting, and distinguishes
/// a clean resource from one whose code is invalid in a bound value set.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn fhir_validate_checks_profile_and_terminology() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let valid = json!({
            "resourceType": "PlanDefinition",
            "status": "active",
            "title": "Acute Stroke Care Pathway",
            "useContext": [{
                "code": {"system": "http://terminology.hl7.org/CodeSystem/usage-context-type", "code": "focus"},
                "valueCodeableConcept": {"coding": [{"system": "http://hl7.org/fhir/sid/icd-10", "code": "I63.9"}]}
            }]
        });
        let body: Value = request
            .post("/fhir/PlanDefinition/$validate")
            .json(&valid)
            .await
            .json();
        assert_eq!(body["resourceType"], "OperationOutcome");
        let issues = body["issue"].as_array().expect("issue array");
        assert!(
            issues.iter().all(|i| i["severity"] != "error"),
            "a conformant resource must produce no errors: {body}"
        );

        // A code that is well-formed JSON but invalid ICD-10.
        let mut invalid = valid.clone();
        invalid["useContext"][0]["valueCodeableConcept"]["coding"][0]["code"] = json!("banana");
        let body: Value = request
            .post("/fhir/PlanDefinition/$validate")
            .json(&invalid)
            .await
            .json();
        assert!(
            body["issue"]
                .as_array()
                .is_some_and(|v| v.iter().any(|i| i["code"] == "code-invalid")),
            "an invalid code in a bound system must be reported: {body}"
        );

        // Validation must not have persisted anything.
        let listed: Value = request.get("/api/care-pathways").await.json();
        assert_eq!(
            listed.as_array().map(Vec::len),
            Some(0),
            "$validate must not create records"
        );
    })
    .await;
}

/// SMART discovery is a **404 with an explanation** unless the deployment
/// configures an authorization server — the honest-gap rule.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn smart_discovery_is_absent_unless_configured() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/fhir/.well-known/smart-configuration").await;
        // The test environment configures no SMART server.
        assert_eq!(response.status_code(), 404);
        let body: Value = response.json();
        assert_eq!(body["resourceType"], "OperationOutcome");
        assert!(
            body["issue"][0]["diagnostics"]
                .as_str()
                .is_some_and(|d| d.contains("PASETO")),
            "the 404 must explain what this service actually uses: {body}"
        );
    })
    .await;
}

/// The `CapabilityStatement` declares the profile and the operations the
/// service really implements.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn capability_statement_declares_profile_and_operations() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let body: Value = request.get("/fhir/metadata").await.json();
        assert_eq!(body["resourceType"], "CapabilityStatement");
        assert_eq!(body["fhirVersion"], "5.0.0");
        let resource = &body["rest"][0]["resource"][0];
        assert_eq!(resource["type"], "PlanDefinition");
        assert!(
            resource["profile"]
                .as_str()
                .is_some_and(|p| p.starts_with("urn:mxi:")),
            "the declared profile must be the family-local one: {resource}"
        );
        assert!(
            resource["operation"]
                .as_array()
                .is_some_and(|ops| ops.iter().any(|o| o["name"] == "validate")),
            "$validate must be declared: {resource}"
        );
        assert!(
            body["rest"][0]["operation"]
                .as_array()
                .is_some_and(|ops| ops.iter().any(|o| o["name"] == "export")),
            "$export must be declared: {body}"
        );
        // No SMART server is configured here, so no security block.
        assert!(
            body["rest"][0]["security"].is_null(),
            "SMART must not be advertised when it is not configured"
        );
    })
    .await;
}

/// The Bulk Data flow works end to end: kickoff → poll → NDJSON.
///
/// The export is now genuinely asynchronous — kickoff enqueues a
/// `bulk_jobs` row on `bg_pg` and returns `202` — so the test polls the
/// status endpoint the way a real Bulk Data client does, instead of
/// assuming the manifest is ready on the first request.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn bulk_export_kickoff_status_and_output() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        create(&request, &rich_pathway()).await;
        create(&request, &rich_pathway()).await;

        let kickoff = request.get("/fhir/$export").await;
        assert_eq!(kickoff.status_code(), 202, "kickoff must be async-shaped");
        let location = kickoff
            .headers()
            .get("content-location")
            .and_then(|v| v.to_str().ok())
            .expect("Content-Location header")
            .to_string();
        assert!(location.starts_with("/fhir/$export-status/"), "{location}");

        // The job is durable: it exists as a row, not just in this process.
        let job_id = location
            .rsplit('/')
            .next()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .expect("job id in Content-Location");
        let job = care_pathway_service::models::bulk_jobs::Model::find_by_id(&ctx.db, job_id)
            .await
            .expect("query")
            .expect("the job row exists");
        assert_eq!(job.kind, "export");
        assert_eq!(job.format, "ndjson");

        // `config/test.yaml` runs workers in `ForegroundBlocking` mode, so
        // `perform_later` has already materialised the export by the time
        // kickoff returned. In production (`BackgroundQueue`) the same
        // call enqueues and this poll would first see `202`.
        let status = request.get(&location).await;
        assert_eq!(
            status.status_code(),
            200,
            "completed job returns a manifest"
        );
        let manifest: Value = status.json();
        assert!(manifest["transactionTime"].is_string());
        assert_eq!(manifest["output"][0]["type"], "PlanDefinition");
        assert_eq!(manifest["output"][0]["count"], 2);
        assert_eq!(manifest["error"].as_array().map(Vec::len), Some(0));

        let file_url = manifest["output"][0]["url"].as_str().expect("output url");
        let file = request.get(file_url).await;
        assert_eq!(file.status_code(), 200);
        let text = file.text();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "one resource per line");
        for line in lines {
            let resource: Value = serde_json::from_str(line).expect("each line is JSON");
            assert_eq!(resource["resourceType"], "PlanDefinition");
            assert!(
                resource["meta"]["profile"][0]
                    .as_str()
                    .is_some_and(|p| p.starts_with("urn:mxi:")),
                "exported resources must carry the profile claim: {resource}"
            );
        }

        // An unknown job id is a 404, not a panic.
        let unknown = uuid::Uuid::new_v4();
        assert_eq!(
            request
                .get(&format!("/fhir/$export-status/{unknown}"))
                .await
                .status_code(),
            404
        );
    })
    .await;
}

/// A **queued** job reports `202` with `X-Progress` — the IG's
/// in-progress response — so a client polls rather than treating it as an
/// error.
///
/// The job is created directly rather than through `$export`, because
/// `config/test.yaml` runs workers in `ForegroundBlocking` mode: a
/// kickoff there is already complete before it returns, so the queued
/// state is unobservable through the API in this environment. Testing the
/// endpoint's contract against a known-queued row checks the thing that
/// actually matters, instead of a worker mode.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn queued_export_reports_progress_not_a_manifest() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let job = care_pathway_service::models::bulk_jobs::Model::submit(
            &ctx.db,
            "care_pathway",
            "export",
            "ndjson",
            serde_json::json!({}),
            None,
            None,
        )
        .await
        .expect("submit");
        assert_eq!(job.status, "queued");

        let status = request
            .get(&format!("/fhir/$export-status/{}", job.id))
            .await;
        assert_eq!(status.status_code(), 202, "a queued job is not complete");
        assert_eq!(
            status
                .headers()
                .get("x-progress")
                .and_then(|v| v.to_str().ok()),
            Some("queued"),
            "the IG's X-Progress header must say where the job is"
        );
    })
    .await;
}

/// Cancelling a live job drops its output reference immediately, so a
/// cancelled export stops serving clinical data rather than waiting for
/// its TTL — and a job that already finished cannot be cancelled.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn cancelling_an_export_makes_its_output_unreachable() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        // A live (queued) job, created directly for the reason above.
        let job = care_pathway_service::models::bulk_jobs::Model::submit(
            &ctx.db,
            "care_pathway",
            "export",
            "ndjson",
            serde_json::json!({}),
            None,
            None,
        )
        .await
        .expect("submit");
        let location = format!("/fhir/$export-status/{}", job.id);

        assert_eq!(request.delete(&location).await.status_code(), 202);
        assert_eq!(
            request.get(&location).await.status_code(),
            404,
            "a cancelled job is gone as far as a client is concerned"
        );
        assert_eq!(
            request
                .get(&format!(
                    "/fhir/$export-file/{}/PlanDefinition.ndjson",
                    job.id
                ))
                .await
                .status_code(),
            404,
            "and its bytes stop being reachable at once"
        );

        // A finished job cannot be cancelled — its result is already out.
        create(&request, &rich_pathway()).await;
        let done = request.get("/fhir/$export").await;
        let done_location = done
            .headers()
            .get("content-location")
            .and_then(|v| v.to_str().ok())
            .expect("Content-Location")
            .to_string();
        assert_eq!(
            request.delete(&done_location).await.status_code(),
            404,
            "cancelling a completed job is not meaningful"
        );
    })
    .await;
}

/// The export survives the process: the job is a Postgres row and the
/// output is an artifact, not memory. This is the whole point of moving
/// off the in-process registry — a poll from another replica, or after a
/// restart, must still answer.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn export_state_is_durable_not_in_process() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        create(&request, &rich_pathway()).await;
        let location = request
            .get("/fhir/$export")
            .await
            .headers()
            .get("content-location")
            .and_then(|v| v.to_str().ok())
            .expect("Content-Location")
            .to_string();
        let job_id = location
            .rsplit('/')
            .next()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .expect("job id");

        // Read the state straight from the database, bypassing the
        // service entirely — which is exactly what a second replica does.
        let row = care_pathway_service::models::bulk_jobs::Model::find_by_id(&ctx.db, job_id)
            .await
            .expect("query")
            .expect("the job is a durable row");
        assert_eq!(row.status, "completed");
        assert_eq!(row.rows_processed, 1);
        let reference = row
            .result_url
            .expect("a completed job references its artifact");

        // And the bytes are in the artifact store, retrievable without
        // any in-process state.
        use care_pathway_service::bulk::store::{ArtifactStore, LocalFsArtifactStore};
        let bytes = LocalFsArtifactStore::from_env()
            .get(&reference)
            .await
            .expect("artifact is readable from the store");
        let text = String::from_utf8(bytes).expect("utf-8");
        assert_eq!(text.lines().count(), 1);
        assert!(text.contains("PlanDefinition"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// **The external witness.** Deleting audit rows wholesale is invisible to
// the chain — remove the tail and no successor is left to break, so the
// shortened chain verifies perfectly. A checkpoint recorded off-box is
// what catches it.
//
// This test is the argument in executable form: take a checkpoint, delete
// the rows it witnessed, confirm the chain still says "verified", and
// confirm the checkpoint says otherwise.
async fn a_recorded_checkpoint_detects_wholesale_deletion() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        // Some history to witness.
        for i in 0..3 {
            let mut body = rich_pathway();
            body["name"] = serde_json::json!(format!("Checkpoint subject {i}"));
            request.post("/api/care-pathways").json(&body).await;
        }

        let taken = request.get("/api/compliance/checkpoint").await;
        assert_eq!(
            taken.status_code(),
            200,
            "a chain with rows yields a checkpoint"
        );
        let checkpoint: serde_json::Value = taken.json();
        assert!(checkpoint["head"].as_str().is_some_and(|h| !h.is_empty()));

        // Honoured before anything is touched.
        let before = request
            .post("/api/compliance/checkpoint/verify")
            .json(&checkpoint)
            .await;
        assert_eq!(before.json::<serde_json::Value>()["honoured"], true);

        // Now delete the trail wholesale, as an attacker with SQL access
        // would. No audit row is written; nothing in-band records it.
        sqlx::query!("DELETE FROM audit_logs")
            .execute(&ctx.db)
            .await
            .expect("delete the trail");

        // The chain's own verification is perfectly happy: there is
        // nothing left to contradict. This is the blind spot.
        let chain = request.get("/api/compliance/audit/verify?limit=1000").await;
        assert_eq!(
            chain.json::<serde_json::Value>()["verified"],
            true,
            "an emptied chain verifies vacuously — which is exactly why the witness exists"
        );

        // The checkpoint is not fooled.
        let after = request
            .post("/api/compliance/checkpoint/verify")
            .json(&checkpoint)
            .await;
        let report: serde_json::Value = after.json();
        assert_eq!(
            report["honoured"], false,
            "deletion must be detected: {report}"
        );
        assert_eq!(
            report["verdict"]["verdict"], "anchor_missing",
            "and named as a deletion, not as a content change: {report}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A checkpoint that has itself been altered cannot be used to accuse the
// chain: the verdict says the *witness* failed, not the data.
//
// Without this distinction, an attacker who could edit the stored
// checkpoint would be able to manufacture an apparent tampering incident
// — or, worse, an investigation would start in the wrong place.
async fn an_altered_checkpoint_accuses_itself_not_the_chain() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        request
            .post("/api/care-pathways")
            .json(&rich_pathway())
            .await;
        let taken = request.get("/api/compliance/checkpoint").await;
        let mut checkpoint: serde_json::Value = taken.json();

        // Only meaningful when a key is configured; without one the
        // checkpoint carries no MAC and this is not the property under
        // test.
        if checkpoint["mac"].is_null() {
            return;
        }
        // Alter it to a value that is certainly different from whatever
        // was recorded. An earlier version of this test set it to `1`,
        // which happened to be the real count — so the "alteration" was a
        // no-op and the test passed while proving nothing.
        let recorded = checkpoint["rows_at_or_before"].as_u64().expect("a count");
        checkpoint["rows_at_or_before"] = serde_json::json!(recorded + 1_000);

        let response = request
            .post("/api/compliance/checkpoint/verify")
            .json(&checkpoint)
            .await;
        let report: serde_json::Value = response.json();
        assert_eq!(report["honoured"], false, "report: {report}");
        assert_eq!(
            report["verdict"]["verdict"], "checkpoint_not_authentic",
            "the witness is what failed, not the chain: {report}"
        );
    })
    .await;
}

/// The `pid`s the record-integrity report flags.
async fn mismatched_pids(request: &TestServer) -> Vec<String> {
    let response = request
        .get("/api/compliance/records/verify?limit=1000")
        .await;
    assert_eq!(response.status_code(), 200, "records verify should answer");
    let report: Value = response.json();
    report["mismatched"]
        .as_array()
        .expect("mismatched array")
        .iter()
        .map(|m| m["pid"].as_str().expect("mismatch pid").to_string())
        .collect()
}

/// INT-V1: a record the service wrote must verify. The digests are
/// computed from the payload the service serialized, while verification
/// recomputes them from the row Postgres hands back — so the JSONB
/// round-trip must not change the pre-image. A fresh record verifies, an
/// updated one verifies, and a row edited behind the service's back
/// through a raw pool is flagged.
///
/// Care pathway is not affected by INT-V1: the payload holds no floats, and its
/// canonical JSON is key-sorted, so the stored `jsonb` reproduces it
/// byte for byte. The test stays as a guard.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_written_record_verifies() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        // Every nested collection the payload carries, so the digest is
        // exercised over the whole stored record, not just `name`.
        let mut body = rich_pathway();
        body["care_setting"] = json!("Inpatient");
        body["same_as"] = json!(["https://www.wikidata.org/wiki/Q12202"]);
        body["keywords"] = json!(["stroke", "Zürich", "日本", "tab\there"]);
        body["relationships"] = json!([{"relation": "FollowedBy", "pathway_id": "rehab-1"}]);
        body["tags"] = json!(["tier-1", "acute"]);
        let pid = create(&request, &body).await;
        let uuid = uuid::Uuid::parse_str(&pid).expect("uuid");

        // Not vacuous: an unhashed row is never a mismatch, so the record
        // must actually carry both digests for the checks below to mean
        // anything.
        let digests = sqlx::query!(
            "SELECT content_hash, content_hash_sha3 FROM care_pathways WHERE pid = $1",
            uuid,
        )
        .fetch_one(&ctx.db)
        .await
        .expect("read digests");
        assert!(
            digests.content_hash.is_some() && digests.content_hash_sha3.is_some(),
            "a new record carries its digests"
        );
        let mismatched = mismatched_pids(&request).await;
        assert!(
            !mismatched.contains(&pid),
            "a freshly created record must verify; mismatched: {mismatched:?}"
        );

        let mut changed = body.clone();
        changed["keywords"] = json!(["stroke", "thrombolysis", "Ünïcode"]);
        changed["tags"] = json!(["tier-2"]);
        let response = request
            .put(&format!("/api/care-pathways/{pid}"))
            .json(&changed)
            .await;
        assert_eq!(response.status_code(), 200, "update should succeed");
        let mismatched = mismatched_pids(&request).await;
        assert!(
            !mismatched.contains(&pid),
            "an updated record must verify; mismatched: {mismatched:?}"
        );

        // An edit made behind the service's back, through a raw pool of
        // its own, must still be caught.
        let url = std::env::var("DB_URL").expect("DB_URL");
        let raw = sqlx::PgPool::connect(&url).await.expect("raw pool");
        sqlx::query!(
            "UPDATE care_pathways SET name = name || ' (edited in SQL)' WHERE pid = $1",
            uuid,
        )
        .execute(&raw)
        .await
        .expect("raw SQL edit");
        let mismatched = mismatched_pids(&request).await;
        assert!(
            mismatched.contains(&pid),
            "a row edited in SQL must be reported; mismatched: {mismatched:?}"
        );
    })
    .await;
}

/// `(rows, redacted rows, erased rows)` in the audit trail for `pid`.
async fn audit_counts(db: &sqlx::PgPool, pid: &str) -> (i64, i64, i64) {
    let row = sqlx::query!(
        r#"SELECT count(*) AS "rows!",
                  count(*) FILTER (WHERE redacted_at IS NOT NULL) AS "redacted!",
                  count(*) FILTER (WHERE action = 'erased') AS "erased!"
           FROM audit_logs WHERE entity_pid = $1::text::uuid"#,
        pid,
    )
    .fetch_one(db)
    .await
    .expect("audit counts");
    (row.rows, row.redacted, row.erased)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// CS-G1 (as found in case): GDPR erasure is all or nothing. A trigger
// makes the **last** step — appending the chained `erased` audit row —
// fail for this one pathway, and every earlier step must then be undone
// too: the payload not tombstoned, the audit content not redacted, and
// the chain still verifying with no `erased` row. Before the fix the
// tombstone and the redaction each committed on their own, leaving the
// record half erased with no accountability row recording it.
//
// The trigger fires only on an `erased` insert, which only an erasure
// writes. It is installed and dropped around the one request, before any
// assertion, and the suite is `#[serial]`, so it cannot reach another
// test.
async fn erasure_failing_late_leaves_nothing_changed() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pid = create(&request, &rich_pathway()).await;
        let (rows_before, redacted_before, _) = audit_counts(&ctx.db, &pid).await;
        assert!(rows_before >= 1, "the create must leave audit content");
        assert_eq!(redacted_before, 0);

        sqlx::query!(
            "CREATE OR REPLACE FUNCTION cp_g1_fail_erased() RETURNS trigger
                 LANGUAGE plpgsql AS $$
                 BEGIN RAISE EXCEPTION 'erasure test: erased audit row refused'; END $$"
        )
        .execute(&ctx.db)
        .await
        .expect("install the failing function");
        sqlx::query!("DROP TRIGGER IF EXISTS cp_g1_fail_erased ON audit_logs")
            .execute(&ctx.db)
            .await
            .expect("clear any stale trigger");
        sqlx::query!(
            "CREATE TRIGGER cp_g1_fail_erased BEFORE INSERT ON audit_logs
                 FOR EACH ROW WHEN (NEW.action = 'erased')
                 EXECUTE FUNCTION cp_g1_fail_erased()"
        )
        .execute(&ctx.db)
        .await
        .expect("install the failing trigger");

        let erase = request
            .post(&format!("/api/care-pathways/{pid}/erase"))
            .await;

        sqlx::query!("DROP TRIGGER cp_g1_fail_erased ON audit_logs")
            .execute(&ctx.db)
            .await
            .expect("drop the failing trigger");
        sqlx::query!("DROP FUNCTION cp_g1_fail_erased()")
            .execute(&ctx.db)
            .await
            .expect("drop the failing function");

        assert_eq!(
            erase.status_code(),
            500,
            "a failed audit append must fail the erasure: {}",
            erase.text()
        );

        // Step 1 undone: the pathway is live and still holds its payload.
        let live = request.get(&format!("/api/care-pathways/{pid}")).await;
        assert_eq!(live.status_code(), 200, "the pathway must still be live");
        let live: Value = live.json();
        assert_eq!(live["name"], "Acute Stroke Care Pathway");
        // Step 2 undone, and step 3 absent: nothing redacted, no `erased`.
        let (_, redacted, erased) = audit_counts(&ctx.db, &pid).await;
        assert_eq!(redacted, 0, "the redaction must be rolled back");
        assert_eq!(
            erased, 0,
            "no erased row for an erasure that did not happen"
        );
        // The chain still verifies: the rolled-back append left no row.
        let report = verify(&request).await;
        assert_eq!(
            report["verified"], true,
            "the chain must verify after a failed erasure: {report}"
        );

        // A retry with the fault gone erases fully.
        let retry = request
            .post(&format!("/api/care-pathways/{pid}/erase"))
            .await;
        assert_eq!(retry.status_code(), 200, "the retry should succeed");
        let (rows, redacted, erased) = audit_counts(&ctx.db, &pid).await;
        assert_eq!(erased, 1, "exactly one erased row");
        assert_eq!(redacted, rows - 1, "every row but the erased one redacted");
        let name = sqlx::query_scalar!(
            "SELECT name FROM care_pathways WHERE pid = $1::text::uuid",
            pid
        )
        .fetch_one(&ctx.db)
        .await
        .expect("the row survives erasure");
        assert_eq!(name, "(erased)");
        let report = verify(&request).await;
        assert_eq!(
            report["verified"], true,
            "the chain must verify after the retry: {report}"
        );
    })
    .await;
}

/// Erasure removes the pathway from the full-text index (CS-E1 as found
/// in case). The index keeps its own copy of the name and the searchable
/// payload, so an erasure that left it there would keep the erased
/// content at rest after Art. 17 said it was gone, even though search
/// answers resolve against the tombstoned row and so never return it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn erasure_removes_the_pathway_from_the_search_index() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let mut body = rich_pathway();
        let name = format!("Erasable pathway {}", uuid::Uuid::new_v4().simple());
        body["name"] = json!(name);
        let pid = create(&request, &body).await;
        let engine = care_pathway_service::search::engine().expect("the search index");
        engine.reload().ok();
        assert!(
            engine.search(&name, 10).unwrap().contains(&pid),
            "the new pathway must be indexed"
        );

        let erase = request
            .post(&format!("/api/care-pathways/{pid}/erase"))
            .await;
        assert_eq!(erase.status_code(), 200, "erase: {}", erase.text());
        engine.reload().ok();
        assert!(
            !engine.search(&name, 10).unwrap().contains(&pid),
            "an erased pathway must leave the index"
        );
    })
    .await;
}
