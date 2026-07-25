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

/// Benefits awareness (WPM-R26): a `benefit`-kind rule signposts a
/// linked plan, goes quiet on enrolment (derived, WPM-D18), the kind
/// gate holds, and `?kind=` filters.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn benefits_awareness_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee(&request, &org, "B-1", None).await;
        activate(&request, &employee).await;

        // A real benefit plan to signpost.
        let plan: Value = request
            .post("/api/benefit-plans")
            .json(&json!({
                "name": "Cycle to work", "kind": "wellness", "provider": "CycleCo",
                "employee_cost_minor": 0, "employer_cost_minor": 500,
                "currency": "GBP",
            }))
            .await
            .json();
        let plan_pid = plan["pid"].as_str().unwrap().to_string();

        // The kind gate: linking a plan from a health rule is refused,
        // as is an unknown kind or a dead plan.
        assert_eq!(
            request
                .post("/api/wellbeing-entitlements")
                .json(&json!({ "name": "Bad", "description": "x", "kind": "health",
                               "benefit_plan_pid": plan_pid }))
                .await
                .status_code(),
            422,
            "benefit_plan_pid requires kind benefit"
        );
        assert_eq!(
            request
                .post("/api/wellbeing-entitlements")
                .json(&json!({ "name": "Bad", "description": "x", "kind": "voucher" }))
                .await
                .status_code(),
            422,
            "kind is a closed vocabulary"
        );
        assert_eq!(
            request
                .post("/api/wellbeing-entitlements")
                .json(&json!({ "name": "Bad", "description": "x", "kind": "benefit",
                               "benefit_plan_pid": uuid::Uuid::new_v4() }))
                .await
                .status_code(),
            404,
            "a linked plan must exist"
        );

        // A plan-linked benefit rule, open to everyone.
        let rule: Value = request
            .post("/api/wellbeing-entitlements")
            .json(&json!({
                "name": "Cycle-to-work scheme",
                "description": "Save on a bike through salary sacrifice.",
                "kind": "benefit",
                "benefit_plan_pid": plan_pid,
            }))
            .await
            .json();
        let rule_pid = rule["pid"].as_str().unwrap().to_string();

        // The prompt carries the kind and the plan reference.
        let prompts: Value = request
            .get(&format!("/api/employees/{employee}/wellbeing-prompts"))
            .await
            .json();
        let items = prompts["prompts"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["entitlement_kind"], "benefit");
        assert_eq!(items[0]["benefit_plan_pid"], plan_pid.as_str());

        // Enrolling in the plan silences the prompt — derived, with no
        // acknowledgement ever written (WPM-D18).
        request
            .post(&format!("/api/employees/{employee}/benefit-enrollments"))
            .json(&json!({ "plan_pid": plan_pid, "starts_on": "2026-07-01" }))
            .await
            .assert_status_ok();
        let prompts: Value = request
            .get(&format!("/api/employees/{employee}/wellbeing-prompts"))
            .await
            .json();
        assert!(
            prompts["prompts"].as_array().unwrap().is_empty(),
            "enrolment quietens the plan-linked prompt"
        );

        // `?kind=` filters; the uptake row carries the kind.
        let benefit_rules: Value =
            request.get("/api/wellbeing-entitlements?kind=benefit").await.json();
        assert_eq!(benefit_rules.as_array().unwrap().len(), 1);
        let health_rules: Value =
            request.get("/api/wellbeing-entitlements?kind=health").await.json();
        assert!(health_rules.as_array().unwrap().is_empty());
        assert_eq!(
            request.get("/api/wellbeing-entitlements?kind=voucher").await.status_code(),
            422
        );
        let uptake: Value = request.get("/api/wellbeing/uptake").await.json();
        let row = uptake["entitlements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["entitlement_pid"] == rule_pid.as_str())
            .expect("uptake row");
        assert_eq!(row["kind"], "benefit");
        assert_eq!(row["uptake_rate"]["denominator"], 0);
        assert!(row["uptake_rate"]["value"].is_null(), "no acknowledgements ⇒ null, not 0");
        assert_eq!(
            row["enrolment_conversion"]["denominator"], 0,
            "plan-linked rule carries conversion terms"
        );
        assert!(row["enrolment_conversion"]["value"].is_null(), "null, not 0");

        // ── Enrolment conversion: of the acknowledgers, how many are
        // now live-enrolled in the linked plan. The enrolled employee
        // acknowledges `done`; a second employee dismisses and does
        // not enrol ⇒ 1/2. A health rule carries no conversion.
        let second = seed_employee(&request, &org, "B-2", None).await;
        activate(&request, &second).await;
        request
            .post(&format!("/api/employees/{employee}/wellbeing-acknowledgements"))
            .json(&json!({ "entitlement_pid": rule_pid, "response": "done" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/employees/{second}/wellbeing-acknowledgements"))
            .json(&json!({ "entitlement_pid": rule_pid, "response": "dismissed" }))
            .await
            .assert_status_ok();
        let health: Value = request
            .post("/api/wellbeing-entitlements")
            .json(&json!({ "name": "Flu", "description": "x" }))
            .await
            .json();
        let uptake: Value = request.get("/api/wellbeing/uptake").await.json();
        let rows = uptake["entitlements"].as_array().unwrap();
        let row = rows
            .iter()
            .find(|r| r["entitlement_pid"] == rule_pid.as_str())
            .expect("uptake row");
        assert_eq!(row["enrolment_conversion"]["numerator"], 1);
        assert_eq!(row["enrolment_conversion"]["denominator"], 2);
        assert_eq!(row["enrolment_conversion"]["value"], 0.5);
        let health_row = rows
            .iter()
            .find(|r| r["entitlement_pid"] == health["pid"])
            .expect("health row");
        assert!(
            health_row["enrolment_conversion"].is_null(),
            "no linked plan ⇒ no conversion"
        );
        let raw = serde_json::to_string(&uptake).unwrap();
        assert!(!raw.contains(&second), "still no employee pid in the aggregate view");
    })
    .await;
}
