//! Board-read request tests (PF-T17): the whiteboard card shape, the
//! **ETag conditional GET** (`304 Not Modified` while nothing changed,
//! a fresh tag after a change), the at-a-glance arithmetic, and the
//! locate read-audit pin.

use loco_rs::testing::prelude::*;
use patient_flow_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_ward};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn whiteboard_cards_and_etag_cycle() {
    request::<App, _, _>(|request, _ctx| async move {
        let (ward_pid, bed_pids) = seed_ward(&request, "inpatient", None, 2).await;
        request
            .post("/api/stays")
            .json(&json!({
                "person_ref": a_person(), "display_name": "Board Patient",
                "source": "ed", "bed_pid": bed_pids[0], "edd": "2026-07-20",
            }))
            .await
            .assert_status_ok();

        // First read: 200 with an ETag and the expected card shape.
        let first = request.get(&format!("/api/whiteboard/{ward_pid}")).await;
        assert_eq!(first.status_code(), 200);
        let etag = first
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .expect("whiteboard carries an ETag")
            .to_string();
        let board: Value = first.json();
        assert_eq!(board["cards"].as_array().map(Vec::len), Some(2));
        let occupied = board["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["state"] == "occupied")
            .expect("one occupied card");
        assert_eq!(occupied["display_name"], "Board Patient");
        assert_eq!(occupied["edd"], "2026-07-20");

        // Same state + If-None-Match ⇒ 304, no body.
        let unchanged = request
            .get(&format!("/api/whiteboard/{ward_pid}"))
            .add_header("if-none-match", etag.clone())
            .await;
        assert_eq!(unchanged.status_code(), 304, "unchanged board is Not Modified");
        assert!(unchanged.text().is_empty(), "304 carries no body");

        // A state change invalidates the tag: the same conditional read
        // now returns 200 with a different ETag.
        request
            .post(&format!("/api/beds/{}/state", bed_pids[1]))
            .json(&json!({ "transition": "close", "reason": "staffing" }))
            .await
            .assert_status_ok();
        let changed = request
            .get(&format!("/api/whiteboard/{ward_pid}"))
            .add_header("if-none-match", etag.clone())
            .await;
        assert_eq!(changed.status_code(), 200, "changed board re-sends");
        let fresh = changed
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .expect("fresh ETag");
        assert_ne!(fresh, etag, "the tag changed with the content");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// At-a-glance counts the seeded topology and is itself conditional.
async fn at_a_glance_counts_and_is_conditional() {
    request::<App, _, _>(|request, _ctx| async move {
        let (ward_pid, bed_pids) = seed_ward(&request, "inpatient", None, 3).await;
        request
            .post("/api/stays")
            .json(&json!({
                "person_ref": a_person(), "display_name": "Glance Patient",
                "source": "ed", "bed_pid": bed_pids[0],
            }))
            .await
            .assert_status_ok();
        let first = request.get("/api/at-a-glance").await;
        assert_eq!(first.status_code(), 200);
        let etag = first
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .expect("at-a-glance carries an ETag")
            .to_string();
        let glance: Value = first.json();
        let row = glance["wards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["ward_pid"] == ward_pid.as_str())
            .expect("seeded ward row");
        assert_eq!(row["beds_total"], 3);
        assert_eq!(row["occupied"], 1);
        assert_eq!(row["available"], 2);
        let unchanged = request
            .get("/api/at-a-glance")
            .add_header("if-none-match", etag)
            .await;
        assert_eq!(unchanged.status_code(), 304);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Locate answers "where is patient X" and — because location is
// personal data — writes a `locate_read` audit row (spec `audit.md`).
async fn locate_finds_and_audits_the_read() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_, bed_pids) = seed_ward(&request, "inpatient", None, 1).await;
        let person = a_person();
        request
            .post("/api/stays")
            .json(&json!({
                "person_ref": person, "display_name": "Locate Patient",
                "source": "ed", "bed_pid": bed_pids[0],
            }))
            .await
            .assert_status_ok();
        let location: Value = request.get(&format!("/api/locate/{person}")).await.json();
        assert_eq!(location["display_name"], "Locate Patient");
        assert_eq!(location["status"], "admitted");
        assert_eq!(location["ward"]["code"], "TW");
        assert_eq!(location["bed"], "TW-1");
        let audits: Value = request.get("/api/audits/recent").await.json();
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["action"] == "locate_read"),
            "the locate read is audited"
        );
        // A malformed URN is 422, an unknown person 404.
        assert_eq!(request.get("/api/locate/not-a-urn").await.status_code(), 422);
        assert_eq!(
            request.get(&format!("/api/locate/{}", a_person())).await.status_code(),
            404
        );
    })
    .await;
}
