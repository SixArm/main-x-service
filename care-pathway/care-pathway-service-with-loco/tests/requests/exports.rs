//! The bulk `event_log` / `journey_features` export codecs (spec
//! `13-tasks.md` T-14a), end to end over HTTP: seed a pathway + one
//! enrolled instance with a segment, a team member, and an event; pull
//! both exports in both formats; and pin the one invariant that matters
//! most — neither export ever carries the instance's `subject_ref` or a
//! person URN, even though the seeded instance has both.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn event_log_and_journey_features_exports_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": format!("Export pathway {}", uuid::Uuid::new_v4()),
                "care_setting": "Outpatient",
                "condition_codes": [{"system": "Icd10", "code": "M54"}],
            }))
            .await;
        created.assert_status_ok();
        let template: Value = created.json();
        let pathway = template["pid"].as_str().expect("pathway pid").to_string();

        let subject_ref = format!("person:{}", uuid::Uuid::new_v4());
        let enrolled = request
            .post(&format!("/api/care-pathways/{pathway}/instances"))
            .json(&json!({ "subject_ref": subject_ref }))
            .await;
        enrolled.assert_status_ok();
        let instance: Value = enrolled.json();
        let instance_pid = instance["pid"].as_str().expect("instance pid").to_string();

        let actor_ref = format!("worker:{}", uuid::Uuid::new_v4());
        request
            .post(&format!("/api/instances/{instance_pid}/team"))
            .json(&json!({ "member_ref": actor_ref, "role": "nurse" }))
            .await
            .assert_status_ok();

        request
            .post(&format!("/api/instances/{instance_pid}/segments"))
            .json(&json!({
                "label": "triage assessment", "stage": "triage",
                "category": "value_adding",
                "started_at": "2026-01-01T00:00:00Z",
                "ended_at": "2026-01-02T00:00:00Z",
                "actor_ref": actor_ref,
                "location_ref": "place:11111111-1111-1111-1111-111111111111",
            }))
            .await
            .assert_status_ok();

        request
            .post(&format!("/api/instances/{instance_pid}/events"))
            .json(&json!({ "kind": "review", "note": "on track" }))
            .await
            .assert_status_ok();

        for (path, expected_activity_prefix) in [
            ("export/event-log", "stage:triage"),
            ("export/journey-features", ""),
        ] {
            for format in ["jsonl", "csv"] {
                let response = request
                    .get(&format!(
                        "/api/care-pathways/{pathway}/{path}?format={format}"
                    ))
                    .await;
                assert_eq!(
                    response.status_code(),
                    200,
                    "{path}?format={format}: {}",
                    response.text()
                );
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();
                if format == "csv" {
                    assert_eq!(content_type, "text/csv");
                } else {
                    assert_eq!(content_type, "application/x-ndjson");
                }
                let body = response.text();

                // The one invariant that matters most: never the subject
                // reference, never a bare person URN, and — for the
                // event log — never the raw actor URN either (only its
                // resolved team role).
                assert!(
                    !body.contains(&subject_ref),
                    "{path}?format={format} leaked subject_ref: {body}"
                );
                assert!(
                    !body.contains("person:"),
                    "{path}?format={format} leaked a person URN: {body}"
                );
                assert!(
                    !body.contains(&actor_ref),
                    "{path}?format={format} leaked the raw actor URN: {body}"
                );

                if path == "export/event-log" {
                    assert!(
                        body.contains(expected_activity_prefix),
                        "{path}?format={format} missing {expected_activity_prefix}: {body}"
                    );
                    assert!(
                        body.contains("nurse"),
                        "{path}?format={format} should resolve the actor to its team role: {body}"
                    );
                    assert!(
                        body.contains(&instance_pid),
                        "{path}?format={format} should carry the instance pid as case_id: {body}"
                    );
                } else {
                    assert!(
                        body.contains(&instance_pid),
                        "{path}?format={format} should carry the instance pid as case_id: {body}"
                    );
                }
            }
        }

        // An unrecognised format is a 422, not a silent fallback.
        let bad_format = request
            .get(&format!(
                "/api/care-pathways/{pathway}/export/event-log?format=parquet"
            ))
            .await;
        assert_eq!(bad_format.status_code(), 422);

        // An unknown pathway is a 404.
        let missing = request
            .get("/api/care-pathways/00000000-0000-0000-0000-000000000000/export/event-log")
            .await;
        assert_eq!(missing.status_code(), 404);
    })
    .await;
}
