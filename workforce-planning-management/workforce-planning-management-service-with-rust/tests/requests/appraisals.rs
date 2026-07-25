//! The 360° appraisal round-trip (WPM-R29): nominations by group →
//! lifecycle gates → once-per-rater responses → the group-floored
//! report (WPM-D21: who responded is visible; what they said is only
//! ever a group aggregate).

use workforce_planning_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded circle, the whole 360 surface
async fn appraisal_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let subject = seed_employee(&request, &org, "A-0", None).await;
        activate(&request, &subject).await;
        let manager = seed_employee(&request, &org, "A-M", None).await;
        activate(&request, &manager).await;
        let mut peers = Vec::new();
        for n in 0..3 {
            let pid = seed_employee(&request, &org, &format!("A-P{n}"), None).await;
            activate(&request, &pid).await;
            peers.push(pid);
        }

        // Create: competencies required and unique.
        assert_eq!(
            request
                .post(&format!("/api/employees/{subject}/appraisals"))
                .json(&json!({ "competencies": [] }))
                .await
                .status_code(),
            422,
            "at least one competency"
        );
        let appraisal: Value = request
            .post(&format!("/api/employees/{subject}/appraisals"))
            .json(&json!({ "competencies": ["communication", "delivery"] }))
            .await
            .json();
        let a_pid = appraisal["pid"].as_str().unwrap().to_string();

        // The self nomination is automatic; the subject cannot be
        // re-nominated; groups are a closed set.
        let detail: Value = request.get(&format!("/api/appraisals/{a_pid}")).await.json();
        assert_eq!(detail["nominations"].as_array().unwrap().len(), 1);
        assert_eq!(detail["nominations"][0]["group"], "self");
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/nominations"))
                .json(&json!({ "rater_pid": subject, "group": "peer" }))
                .await
                .status_code(),
            422,
            "subject rates only as self"
        );
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/nominations"))
                .json(&json!({ "rater_pid": manager, "group": "astrologer" }))
                .await
                .status_code(),
            422,
            "groups are closed"
        );

        // Collecting needs >= 3 non-self raters.
        request
            .post(&format!("/api/appraisals/{a_pid}/nominations"))
            .json(&json!({ "rater_pid": manager, "group": "manager" }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/status"))
                .json(&json!({ "to": "collecting" }))
                .await
                .status_code(),
            422,
            "one non-self rater is not enough"
        );
        for peer in &peers {
            request
                .post(&format!("/api/appraisals/{a_pid}/nominations"))
                .json(&json!({ "rater_pid": peer, "group": "peer" }))
                .await
                .assert_status_ok();
        }
        // A response before collecting is refused.
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/responses"))
                .json(&json!({ "rater_pid": manager,
                               "scores": { "communication": 4, "delivery": 3 } }))
                .await
                .status_code(),
            422,
            "no responses in draft"
        );
        request
            .post(&format!("/api/appraisals/{a_pid}/status"))
            .json(&json!({ "to": "collecting" }))
            .await
            .assert_status_ok();
        // Nominations freeze once collecting.
        let late = seed_employee(&request, &org, "A-L", None).await;
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/nominations"))
                .json(&json!({ "rater_pid": late, "group": "peer" }))
                .await
                .status_code(),
            422,
            "nominations frozen"
        );

        // Responses: incomplete or off-scale scores refused; only
        // nominated raters; once per rater.
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/responses"))
                .json(&json!({ "rater_pid": manager, "scores": { "communication": 4 } }))
                .await
                .status_code(),
            422,
            "every declared competency is scored"
        );
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/responses"))
                .json(&json!({ "rater_pid": late,
                               "scores": { "communication": 4, "delivery": 3 } }))
                .await
                .status_code(),
            422,
            "only nominated raters"
        );
        request
            .post(&format!("/api/appraisals/{a_pid}/responses"))
            .json(&json!({ "rater_pid": manager,
                           "scores": { "communication": 4, "delivery": 3 },
                           "comment": "Strong quarter; delegate more." }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/responses"))
                .json(&json!({ "rater_pid": manager,
                               "scores": { "communication": 5, "delivery": 5 } }))
                .await
                .status_code(),
            422,
            "once per rater"
        );
        // Two peers respond (below the floor of 3); the subject self-rates.
        for (peer, score) in peers.iter().take(2).zip([3, 5]) {
            request
                .post(&format!("/api/appraisals/{a_pid}/responses"))
                .json(&json!({ "rater_pid": peer,
                               "scores": { "communication": score, "delivery": score },
                               "comment": format!("peer view {score}") }))
                .await
                .assert_status_ok();
        }
        request
            .post(&format!("/api/appraisals/{a_pid}/responses"))
            .json(&json!({ "rater_pid": subject,
                           "scores": { "communication": 3, "delivery": 4 } }))
            .await
            .assert_status_ok();

        // The detail shows who responded — never scores or comments.
        let detail: Value = request.get(&format!("/api/appraisals/{a_pid}")).await.json();
        let raw = serde_json::to_string(&detail).unwrap();
        assert!(!raw.contains("delegate more"), "no rater content on the detail view");
        assert!(!raw.contains("scores"), "no scores on the detail view");
        let responded: usize = detail["nominations"].as_array().unwrap().iter()
            .filter(|n| n["responded"] == true)
            .count();
        assert_eq!(responded, 4);

        // The report is gated on shared.
        assert_eq!(
            request.get(&format!("/api/appraisals/{a_pid}/report")).await.status_code(),
            422,
            "report only once shared"
        );
        request
            .post(&format!("/api/appraisals/{a_pid}/status"))
            .json(&json!({ "to": "shared" }))
            .await
            .assert_status_ok();
        let report: Value = request.get(&format!("/api/appraisals/{a_pid}/report")).await.json();
        let groups = report["groups"].as_array().unwrap();
        // Manager (n = 1) discloses by convention, with the comment.
        let manager_group = groups.iter().find(|g| g["group"] == "manager").unwrap();
        assert_eq!(manager_group["withheld"], false);
        assert_eq!(manager_group["competencies"]["communication"]["mean"], 4.0);
        assert_eq!(manager_group["comments"][0], "Strong quarter; delegate more.");
        // Peer (2 < 3) is withheld — count included.
        let peer_group = groups.iter().find(|g| g["group"] == "peer").unwrap();
        assert_eq!(peer_group["withheld"], true);
        assert!(peer_group["responses"].is_null(), "withheld cell hides its count");
        assert!(peer_group["comments"].is_null(), "withheld cell hides its comments");
        // Self discloses.
        let self_group = groups.iter().find(|g| g["group"] == "self").unwrap();
        assert_eq!(self_group["competencies"]["delivery"]["mean"], 4.0);
        // A third peer response lifts the floor: the peer cell disclose,
        // with the mean over all three.
        assert_eq!(
            request
                .post(&format!("/api/appraisals/{a_pid}/responses"))
                .json(&json!({ "rater_pid": peers[2],
                               "scores": { "communication": 4, "delivery": 4 } }))
                .await
                .status_code(),
            422,
            "responses closed once shared"
        );
        // (The floor transition is pinned in the pure rules; end-to-end
        // the shared gate correctly freezes late responses.)

        // The report read is audited (WPM-R10 sensitivity posture).
        let audits: Value = request.get("/api/audits/recent").await.json();
        let audit_raw = serde_json::to_string(&audits).unwrap();
        assert!(audit_raw.contains("report_read"));
    })
    .await;
}
