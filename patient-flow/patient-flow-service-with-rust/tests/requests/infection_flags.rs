//! Infection control across a stay: an uncleared transmissible flag keeps
//! the patient out of an ordinary bed on transfer; once cleared, the move
//! is allowed. A flag clears once, only through its own stay, and the
//! clear is in the flag's audit trail and the ward's handover.

use axa_test::prelude::*;
use patient_flow_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_ward};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL via DB_URL; run with `cargo test -- --ignored`"]
async fn an_open_flag_constrains_transfer_until_cleared() {
    request::<App, _, _>(|request, _ctx| async move {
        let since = chrono::Utc::now() - chrono::Duration::seconds(5);
        let (ward, beds) = seed_ward!(&request, "inpatient", Some("respiratory"), 2).await;
        let admit = |bed: &str| {
            json!({ "person_ref": a_person(), "display_name": "Flagged Patient", "source": "ed", "bed_pid": bed })
        };
        let stay: Value = request.post("/api/stays").json(&admit(&beds[0])).await.json();
        let stay_pid = stay["pid"].as_str().expect("stay pid").to_string();
        let flag: Value = request
            .post(&format!("/api/stays/{stay_pid}/infection-flags"))
            .json(&json!({ "precaution": "droplet", "organism": "influenza", "status": "confirmed" }))
            .await
            .json();
        let flag_pid = flag["pid"].as_str().expect("flag pid").to_string();

        // The destination is an ordinary bed: refused while the flag is open.
        let to = json!({ "to_bed_pid": beds[1], "reason": "clinical" });
        let refused = request.post(&format!("/api/stays/{stay_pid}/transfer")).json(&to).await;
        assert_eq!(refused.status_code(), 422, "{}", refused.text());

        // A flag clears only through its own stay, once.
        let other: Value = request.post("/api/stays").json(&admit(&beds[1])).await.json();
        let other_pid = other["pid"].as_str().unwrap();
        let wrong = request
            .post(&format!("/api/stays/{other_pid}/infection-flags/{flag_pid}/clear"))
            .await;
        assert_eq!(wrong.status_code(), 404, "through another stay");
        let clear = format!("/api/stays/{stay_pid}/infection-flags/{flag_pid}/clear");
        assert_eq!(request.post(&clear).await.status_code(), 200);
        assert_eq!(request.post(&clear).await.status_code(), 422, "already cleared");
        let unknown = format!("/api/stays/{stay_pid}/infection-flags/{}/clear", uuid::Uuid::new_v4());
        assert_eq!(request.post(&unknown).await.status_code(), 404);

        // Cleared, the same move to an ordinary bed now succeeds (a fresh
        // ward, since the first one's beds are both occupied).
        let (_w2, beds2) = seed_ward!(&request, "inpatient", Some("respiratory"), 1).await;
        let to = json!({ "to_bed_pid": beds2[0], "reason": "clinical" });
        let moved = request.post(&format!("/api/stays/{stay_pid}/transfer")).json(&to).await;
        assert_eq!(moved.status_code(), 200, "{}", moved.text());

        // The clear is audited, and in the ward's handover.
        let trail: Value = request.get(&format!("/api/audits/{flag_pid}")).await.json();
        assert!(trail.to_string().contains("infection_cleared"), "{trail}");
        let handover = request
            .get(&format!("/api/audits?ward={ward}&since={}", since.to_rfc3339().replace('+', "%2B")))
            .await;
        assert_eq!(handover.status_code(), 200, "{}", handover.text());
        assert!(handover.text().contains("infection_cleared"), "{}", handover.text());
        for uri in ["/api/audits/recent", "/api/events/recent"] {
            assert_eq!(request.get(uri).await.status_code(), 200, "{uri}");
        }
    })
    .await;
}
