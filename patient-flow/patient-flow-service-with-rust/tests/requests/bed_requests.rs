//! The bed-request demand board and cancellation: open requests list in
//! priority order (emergency, urgent, routine) with their live eligible-bed
//! count; cancelling an allocated request releases its reserved bed; a
//! request already cancelled, or unknown, cannot be cancelled again.

use axa_test::prelude::*;
use patient_flow_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_ward};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL via DB_URL; run with `cargo test -- --ignored`"]
async fn demand_board_order_and_cancellation() {
    request::<App, _, _>(|request, _ctx| async move {
        let (ward, beds) = seed_ward!(&request, "inpatient", Some("respiratory"), 2).await;
        let mut pids = Vec::new();
        for priority in ["routine", "emergency", "urgent"] {
            let created: Value = request
                .post("/api/bed-requests")
                .json(&json!({
                    "person_ref": a_person(), "origin": "ed", "priority": priority,
                    "target_ward_pid": ward,
                }))
                .await
                .json();
            pids.push(created["pid"].as_str().expect("pid").to_string());
        }
        let (routine, emergency, urgent) = (&pids[0], &pids[1], &pids[2]);

        let board: Value = request.get("/api/bed-requests").await.json();
        let order: Vec<&str> = board
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v["pid"].as_str())
            .filter(|p| pids.iter().any(|x| x == p))
            .collect();
        assert_eq!(
            order,
            vec![emergency.as_str(), urgent.as_str(), routine.as_str()],
            "{board}"
        );
        let ours = board
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["pid"] == *urgent)
            .unwrap();
        assert!(ours["eligible_beds"].as_u64().unwrap() >= 2, "{ours}");

        // Allocate, then cancel: the reserved bed is released.
        request
            .post(&format!("/api/bed-requests/{urgent}/allocate"))
            .json(&json!({ "bed_pid": beds[0] }))
            .await
            .assert_status_ok();
        let bed: Value = request.get(&format!("/api/beds/{}", beds[0])).await.json();
        assert_eq!(bed["state"], "reserved");
        let cancelled = request
            .post(&format!("/api/bed-requests/{urgent}/cancel"))
            .await;
        assert_eq!(cancelled.status_code(), 200, "{}", cancelled.text());
        assert_eq!(cancelled.json::<Value>()["status"], "cancelled");
        let bed: Value = request.get(&format!("/api/beds/{}", beds[0])).await.json();
        assert_ne!(
            bed["state"], "reserved",
            "the reservation is released: {bed}"
        );

        let again = request
            .post(&format!("/api/bed-requests/{urgent}/cancel"))
            .await;
        assert_eq!(again.status_code(), 422, "{}", again.text());
        let unknown = request
            .post(&format!(
                "/api/bed-requests/{}/cancel",
                uuid::Uuid::new_v4()
            ))
            .await;
        assert_eq!(unknown.status_code(), 404, "{}", unknown.text());

        // An open request with no allocation cancels too.
        let r = request
            .post(&format!("/api/bed-requests/{routine}/cancel"))
            .await;
        assert_eq!(r.status_code(), 200, "{}", r.text());

        let cancelled: Value = request
            .get("/api/bed-requests?status=cancelled")
            .await
            .json();
        let row = cancelled
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["pid"] == *urgent)
            .expect("listed");
        assert_eq!(row["eligible_beds"], 0, "only open requests are counted");
    })
    .await;
}
