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
//! DATABASE_URL=postgres://loco:loco@localhost:5432/care_pathway_service_test \
//!   cargo test -- --ignored
//! ```

use axum_test::TestServer;
use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
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
    request::<App, _, _>(|request, ctx| async move {
        create(&request, &rich_pathway()).await;
        assert_eq!(verify(&request).await["verified"], true);

        // Rewrite a snapshot directly in the database, exactly as an
        // attacker (or a careless operator) with SQL access would.
        use sea_orm::ConnectionTrait as _;
        ctx.db
            .execute_unprepared(
                r#"UPDATE audit_logs SET snapshot = jsonb_set(snapshot, '{name}', '"Tampered"')
                   WHERE snapshot IS NOT NULL AND redacted_at IS NULL"#,
            )
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

/// The posture endpoint reports the running configuration and states, per
/// framework, what is **not** claimed.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn posture_reports_controls_and_disclaimers() {
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
    request::<App, _, _>(|request, _ctx| async move {
        let body: Value = request.get("/api/compliance/sbom").await.json();
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

/// The Bulk Data flow works end to end: kickoff → status manifest →
/// NDJSON output.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn bulk_export_kickoff_status_and_output() {
    request::<App, _, _>(|request, _ctx| async move {
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

        let status = request.get(&location).await;
        assert_eq!(status.status_code(), 200);
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

        // Cancelling releases the payload and makes the job unreachable.
        assert_eq!(request.delete(&location).await.status_code(), 202);
        assert_eq!(request.get(&location).await.status_code(), 404);

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
