//! The wellbeing round-trip (WPM-R25): entitlement rules → cohort
//! prompts (department + age band) → acknowledgements → the one
//! multi-dose reminder → aggregate-only uptake.

use workforce_planning_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded cohort, the whole wellbeing surface
async fn wellbeing_round_trip() {
    request::<App, _, _>(|request, ctx| async move {
        let org = an_org();
        let engineer = seed_employee(&request, &org, "W-1", None).await;
        activate(&request, &engineer).await;
        let second = seed_employee(&request, &org, "W-2", None).await;
        activate(&request, &second).await;
        // Move the second employee out of the engineering cohort.
        request
            .put(&format!("/api/employees/{second}"))
            .json(&json!({ "department": "finance" }))
            .await
            .assert_status_ok();

        // ── Validation: bad age band, bad doses.
        assert_eq!(
            request
                .post("/api/wellbeing-entitlements")
                .json(&json!({ "name": "Bad", "description": "x", "min_age": 79, "max_age": 50 }))
                .await
                .status_code(),
            422,
            "inverted age band refused"
        );
        assert_eq!(
            request
                .post("/api/wellbeing-entitlements")
                .json(&json!({ "name": "Bad", "description": "x", "doses": 0 }))
                .await
                .status_code(),
            422,
            "zero doses refused"
        );

        // ── A department-scoped, two-dose rule (flu-style for frontline).
        let flu: Value = request
            .post("/api/wellbeing-entitlements")
            .json(&json!({
                "name": "Seasonal flu vaccination",
                "description": "Free NHS flu jab for frontline staff.",
                "info_url": "https://www.nhs.uk/vaccinations/flu-vaccine/",
                "departments": ["engineering"],
                "doses": 2,
            }))
            .await
            .json();
        let flu_pid = flu["pid"].as_str().unwrap().to_string();
        // ── An age-banded rule (shingles-style, 65+).
        let shingles: Value = request
            .post("/api/wellbeing-entitlements")
            .json(&json!({
                "name": "Shingles vaccination",
                "description": "Free NHS shingles vaccine from age 65.",
                "min_age": 65,
            }))
            .await
            .json();
        let shingles_pid = shingles["pid"].as_str().unwrap().to_string();

        // ── Prompts: the engineer sees flu (department match) but not
        // shingles — their birth date is unknown, and unknown age fails
        // an age-banded rule (WPM-D17 honesty).
        let prompts: Value = request
            .get(&format!("/api/employees/{engineer}/wellbeing-prompts"))
            .await
            .json();
        assert_eq!(prompts["age_known"], false);
        let names: Vec<&str> = prompts["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p["name"].as_str())
            .collect();
        assert!(names.contains(&"Seasonal flu vaccination"));
        assert!(!names.contains(&"Shingles vaccination"), "unknown age is not a match");
        // The finance employee is outside the flu cohort.
        let other: Value = request
            .get(&format!("/api/employees/{second}/wellbeing-prompts"))
            .await
            .json();
        assert!(other["prompts"].as_array().unwrap().is_empty());

        // ── Priming the birth date (the upstream person seam) makes the
        // age-banded rule match.
        let employee_row: Value =
            request.get(&format!("/api/employees/{engineer}")).await.json();
        let person_urn = employee_row["person_ref"].as_str().unwrap();
        workforce_planning_management_service::clients::prime_birth_date(
            person_urn,
            chrono::NaiveDate::from_ymd_opt(1958, 3, 14).unwrap(),
        );
        let prompts: Value = request
            .get(&format!("/api/employees/{engineer}/wellbeing-prompts"))
            .await
            .json();
        assert_eq!(prompts["age_known"], true);
        let names: Vec<&str> = prompts["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p["name"].as_str())
            .collect();
        assert!(names.contains(&"Shingles vaccination"), "68-year-old matches 65+");

        // ── Acknowledgements: an unknown response token is refused.
        assert_eq!(
            request
                .post(&format!("/api/employees/{engineer}/wellbeing-acknowledgements"))
                .json(&json!({ "entitlement_pid": flu_pid, "response": "maybe" }))
                .await
                .status_code(),
            422,
            "response is a closed vocabulary"
        );
        // Declining shingles removes it and never re-prompts.
        request
            .post(&format!("/api/employees/{engineer}/wellbeing-acknowledgements"))
            .json(&json!({ "entitlement_pid": shingles_pid, "response": "declined" }))
            .await
            .assert_status_ok();
        // Booking the two-dose flu course earns exactly one reminder.
        request
            .post(&format!("/api/employees/{engineer}/wellbeing-acknowledgements"))
            .json(&json!({ "entitlement_pid": flu_pid, "response": "booked" }))
            .await
            .assert_status_ok();
        let prompts: Value = request
            .get(&format!("/api/employees/{engineer}/wellbeing-prompts"))
            .await
            .json();
        let items = prompts["prompts"].as_array().unwrap();
        assert_eq!(items.len(), 1, "declined is gone; flu comes back once as a reminder");
        assert_eq!(items[0]["kind"], "reminder");
        assert_eq!(items[0]["name"], "Seasonal flu vaccination");
        // Serving it stamped it: the next fetch is quiet.
        let prompts: Value = request
            .get(&format!("/api/employees/{engineer}/wellbeing-prompts"))
            .await
            .json();
        assert!(prompts["prompts"].as_array().unwrap().is_empty(), "one reminder only");

        // ── Uptake: aggregate counts only — no individual appears.
        let uptake: Value = request.get("/api/wellbeing/uptake").await.json();
        let rows = uptake["entitlements"].as_array().unwrap();
        let flu_row = rows.iter().find(|r| r["name"] == "Seasonal flu vaccination").unwrap();
        assert_eq!(flu_row["by_response"]["booked"], 1);
        assert_eq!(flu_row["uptake_rate"]["numerator"], 1);
        assert_eq!(flu_row["uptake_rate"]["denominator"], 1);
        let shingles_row = rows.iter().find(|r| r["name"] == "Shingles vaccination").unwrap();
        assert_eq!(shingles_row["by_response"]["declined"], 1);
        assert_eq!(shingles_row["uptake_rate"]["value"], 0.0);
        let raw = serde_json::to_string(&uptake).unwrap();
        assert!(!raw.contains(&engineer), "no employee pid in the aggregate view");
        assert!(!raw.contains(person_urn), "no person URN in the aggregate view");

        // ── The acknowledgement is audited (who said what, never a
        // clinical fact).
        let _ = &ctx; // ctx unused beyond the harness contract
        let audits: Value = request.get("/api/audits/recent").await.json();
        let audit_raw = serde_json::to_string(&audits).unwrap();
        assert!(audit_raw.contains("entitlement_acknowledgement"));

        // ── Soft-closing a rule stops prompting the still-unacknowledged.
        request
            .delete(&format!("/api/wellbeing-entitlements/{shingles_pid}"))
            .await
            .assert_status_ok();
        let listed: Value = request.get("/api/wellbeing-entitlements").await.json();
        assert_eq!(listed.as_array().unwrap().len(), 1, "closed rule not listed");
    })
    .await;
}
