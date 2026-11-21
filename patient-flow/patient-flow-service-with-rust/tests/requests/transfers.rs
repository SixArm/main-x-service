//! `POST /api/stays/{pid}/transfer`: a patient moves to another bed in one
//! transaction (the old bed vacated, the new one occupied, the stay
//! relocated, the move recorded), and a transfer that names no known
//! reason, overrides a rule without saying why, goes nowhere, targets an
//! ineligible bed, or moves a discharged or unknown stay is refused.

use axa_test::prelude::*;
use patient_flow_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_ward};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL via DB_URL; run with `cargo test -- --ignored`"]
async fn a_transfer_moves_the_patient_and_refuses_bad_moves() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_ward, beds) = seed_ward!(&request, "inpatient", Some("respiratory"), 3).await;
        let stay: Value = request
            .post("/api/stays")
            .json(&json!({
                "person_ref": a_person(), "display_name": "Transfer Patient",
                "source": "ed", "bed_pid": beds[0],
            }))
            .await
            .json();
        let stay_pid = stay["pid"].as_str().expect("stay pid").to_string();
        let uri = format!("/api/stays/{stay_pid}/transfer");
        let transfer = |to: &str, extra: Value| {
            let mut body = json!({ "to_bed_pid": to, "reason": "capacity" });
            if let Value::Object(m) = extra {
                for (k, v) in m {
                    body[k] = v;
                }
            }
            body
        };

        for (why, body) in [
            (
                "an unknown reason",
                transfer(&beds[1], json!({ "reason": "whim" })),
            ),
            (
                "an override with no reason",
                transfer(&beds[1], json!({ "override_sex": true })),
            ),
            ("the current bed", transfer(&beds[0], json!({}))),
        ] {
            let r = request.post(&uri).json(&body).await;
            assert_eq!(r.status_code(), 422, "{why}: {}", r.text());
        }

        let moved = request
            .post(&uri)
            .json(&transfer(&beds[1], json!({})))
            .await;
        assert_eq!(moved.status_code(), 200, "{}", moved.text());
        let moved: Value = moved.json();
        assert_eq!(moved["bed_pid"], beds[1], "the stay follows the patient");
        let old: Value = request.get(&format!("/api/beds/{}", beds[0])).await.json();
        let new: Value = request.get(&format!("/api/beds/{}", beds[1])).await.json();
        assert_eq!(new["state"], "occupied", "{new}");
        assert_ne!(old["state"], "occupied", "the old bed is vacated: {old}");

        // The vacated bed awaits cleaning, so it cannot take the patient back.
        let back = request
            .post(&uri)
            .json(&transfer(&beds[0], json!({})))
            .await;
        assert_eq!(back.status_code(), 422, "{}", back.text());

        let unknown = format!("/api/stays/{}/transfer", uuid::Uuid::new_v4());
        let r = request
            .post(&unknown)
            .json(&transfer(&beds[2], json!({})))
            .await;
        assert_eq!(r.status_code(), 404, "{}", r.text());
    })
    .await;
}
