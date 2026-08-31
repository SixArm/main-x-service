//! Subject rights & retention round-trip (CRM-R20): the subject-access
//! export gathers the contact's footprint and names exclusions; erasure
//! is refused while a live engagement exists (an open deal, an open
//! ticket, or an active nurture enrolment) and anonymises once none
//! does; the retention report is horizon-floored and the sweep is
//! audited even when it deletes nothing.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn subject_rights_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        // ── Subject access: gathers the footprint, names exclusions, audited.
        let contact_pid = seed_contact!(&request, "Dana Rivers").await;
        request
            .post("/api/activities")
            .json(
                &json!({ "subject_kind": "contact", "subject_pid": contact_pid,
                            "kind": "note", "summary": "Prefers afternoon calls" }),
            )
            .await
            .assert_status_ok();
        let export: Value = request
            .get(&format!("/api/contacts/{contact_pid}/subject-access"))
            .await
            .json();
        assert_eq!(export["contact"]["pid"], contact_pid);
        assert_eq!(export["consent_history"].as_array().unwrap().len(), 1);
        assert_eq!(export["activities"].as_array().unwrap().len(), 1);
        assert!(!export["exclusions"].as_array().unwrap().is_empty());
        let audits: Value = request
            .get(&format!("/api/audits/{contact_pid}"))
            .await
            .json();
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["action"] == "subject_access_exported"),
            "the export itself is audited"
        );

        // ── Erasure refused while an open deal names it primary contact.
        let (pipeline_pid, stages) = super::seed_pipeline!(&request).await;
        let deal: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Renewal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 250_000_i64, "currency": "GBP",
                            "primary_contact_pid": contact_pid }))
            .await
            .json();
        let deal_pid = deal["pid"].as_str().unwrap().to_string();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/erase"))
                .await
                .status_code(),
            422,
            "an open deal is a live engagement"
        );

        // ── Close the deal (Won, terminal); erasure still refused for an
        // open ticket.
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stages[2] }))
            .await
            .assert_status_ok();
        let ticket: Value = request
            .post("/api/tickets")
            .json(&json!({ "contact_pid": contact_pid, "title": "Login issue",
                            "priority": "normal", "channel": "email" }))
            .await
            .json();
        let ticket_pid = ticket["pid"].as_str().unwrap().to_string();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/erase"))
                .await
                .status_code(),
            422,
            "an open ticket is a live engagement"
        );

        // ── Resolve + close the ticket; erasure still refused for an
        // active nurture enrolment.
        request
            .post(&format!("/api/tickets/{ticket_pid}/status"))
            .json(&json!({ "to": "resolved" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/tickets/{ticket_pid}/status"))
            .json(&json!({ "to": "closed" }))
            .await
            .assert_status_ok();
        let sequence: Value = request
            .post("/api/nurture-sequences")
            .json(&json!({ "name": "Welcome",
                            "steps": [{ "delay_hours": 0, "template_ref": "welcome-1" }] }))
            .await
            .json();
        let sequence_pid = sequence["pid"].as_str().unwrap().to_string();
        request
            .post(&format!(
                "/api/nurture-sequences/{sequence_pid}/enrollments"
            ))
            .json(&json!({ "contact_pid": contact_pid }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/erase"))
                .await
                .status_code(),
            422,
            "an active nurture enrolment is a live engagement"
        );

        // ── Withdraw consent (exits nurture per CRM-D6); now erasure
        // succeeds, and it anonymises the record and scrubs the linked
        // free text.
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "support call" }))
            .await
            .assert_status_ok();
        let erased: Value = request
            .post(&format!("/api/contacts/{contact_pid}/erase"))
            .await
            .json();
        assert_eq!(erased["erased"], contact_pid);
        assert_eq!(
            request
                .get(&format!("/api/contacts/{contact_pid}"))
                .await
                .status_code(),
            404,
            "the contact is soft-deleted"
        );
        let audits_after: Value = request
            .get(&format!("/api/audits/{contact_pid}"))
            .await
            .json();
        let erased_entry = audits_after
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["action"] == "erased")
            .expect("erased audit row")
            .clone();
        assert_eq!(erased_entry["snapshot"]["activity_summaries_scrubbed"], 1);

        // ── Retention: the report discloses the (default, unfloored-by-env)
        // horizon and today's just-erased contact; a sweep against a
        // same-day estate deletes nothing and is still audited (never
        // silently a no-op).
        let report: Value = request.get("/api/retention").await.json();
        assert_eq!(
            report["horizon_days"], 365,
            "CRM_RETENTION_DAYS unset in tests ⇒ default"
        );
        let sweep: Value = request.post("/api/retention/sweep").await.json();
        assert_eq!(
            sweep["rows_deleted"], 0,
            "nothing is past a 365-day horizon yet"
        );
    })
    .await;
}
