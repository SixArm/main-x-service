//! Workforce flows (WPM-R4–R6): time caps + overtime, the leave
//! balance journey (+ the two-approver race), and shift conflicts.

use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use workforce_planning_management_service::app::App;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Time: >24h/day refused; overtime derives beyond the FTE-scaled
// contracted day; approval flips status.
async fn time_caps_and_overtime() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee(&request, &org, "E-1001", None).await;
        activate(&request, &employee).await;
        // 20h recorded fine; +5h more the same day breaks the cap.
        let first: Value = request
            .post(&format!("/api/employees/{employee}/time-entries"))
            .json(&json!({ "worked_on": "2026-07-06", "minutes": 1200 }))
            .await
            .json();
        assert!(first["pid"].is_string());
        let over = request
            .post(&format!("/api/employees/{employee}/time-entries"))
            .json(&json!({ "worked_on": "2026-07-06", "minutes": 300 }))
            .await;
        assert_eq!(over.status_code(), 422, "day total over 24h refused");
        // Overtime: 1200 min regular vs 450 contracted ⇒ 750 overtime.
        let listed: Value = request
            .get(&format!("/api/employees/{employee}/time-entries"))
            .await
            .json();
        assert_eq!(listed["overtime"][0]["overtime_minutes"], 750);
        // Approve.
        let entry_pid = first["pid"].as_str().unwrap();
        request
            .post(&format!("/api/time-entries/{entry_pid}/approve"))
            .await
            .assert_status_ok();
        let again = request
            .post(&format!("/api/time-entries/{entry_pid}/approve"))
            .await;
        assert_eq!(again.status_code(), 422, "double approval refused");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Leave: annual over-balance 422; sick goes negative flagged;
// approval decrements the balance in-tx; the second approver loses;
// cancelling an approved request restores the balance.
async fn leave_balance_journey() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee(&request, &org, "E-1002", None).await;
        activate(&request, &employee).await;
        request
            .post(&format!("/api/employees/{employee}/leave-entitlements"))
            .json(&json!({ "kind": "annual", "year": 2026, "entitled_days": 5 }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/employees/{employee}/leave-entitlements"))
            .json(&json!({ "kind": "sick", "year": 2026, "entitled_days": 2 }))
            .await
            .assert_status_ok();
        // Annual over balance: 6 days vs 5 ⇒ 422.
        let over = request
            .post(&format!("/api/employees/{employee}/leave-requests"))
            .json(&json!({ "kind": "annual", "start_on": "2026-08-03", "end_on": "2026-08-08" }))
            .await;
        assert_eq!(over.status_code(), 422);
        // Annual within balance.
        let annual: Value = request
            .post(&format!("/api/employees/{employee}/leave-requests"))
            .json(&json!({ "kind": "annual", "start_on": "2026-08-03", "end_on": "2026-08-07" }))
            .await
            .json();
        assert_eq!(annual["days"], 5);
        assert_eq!(annual["negative_balance"], false);
        // Sick beyond balance: allowed but flagged.
        let sick: Value = request
            .post(&format!("/api/employees/{employee}/leave-requests"))
            .json(&json!({ "kind": "sick", "start_on": "2026-09-01", "end_on": "2026-09-04" }))
            .await
            .json();
        assert_eq!(sick["negative_balance"], true);
        // Approve the annual request; balance drops to 0.
        let annual_pid = annual["pid"].as_str().unwrap();
        request
            .post(&format!("/api/leave-requests/{annual_pid}/approve"))
            .await
            .assert_status_ok();
        let balances: Value = request
            .get(&format!("/api/employees/{employee}/leave-entitlements"))
            .await
            .json();
        let annual_balance = balances
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["kind"] == "annual")
            .unwrap();
        assert_eq!(annual_balance["used_days"], 5);
        // The second decision on the same request is refused (the
        // race's loser sees the decided status).
        let second = request
            .post(&format!("/api/leave-requests/{annual_pid}/reject"))
            .await;
        assert_eq!(second.status_code(), 422, "already decided");
        // Cancelling the approved request restores the balance.
        request
            .post(&format!("/api/leave-requests/{annual_pid}/cancel"))
            .await
            .assert_status_ok();
        let balances: Value = request
            .get(&format!("/api/employees/{employee}/leave-entitlements"))
            .await
            .json();
        let annual_balance = balances
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["kind"] == "annual")
            .unwrap();
        assert_eq!(
            annual_balance["used_days"], 0,
            "cancel restores the balance"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Shifts: double-booking refused; assignment over approved leave
// refused; back-to-back shifts are fine.
async fn shift_conflicts() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee(&request, &org, "E-1003", None).await;
        activate(&request, &employee).await;
        let early: Value = request
            .post("/api/shifts")
            .json(&json!({
                "department": "engineering",
                "starts_at": "2026-07-06T08:00:00Z", "ends_at": "2026-07-06T16:00:00Z",
            }))
            .await
            .json();
        let overlapping: Value = request
            .post("/api/shifts")
            .json(&json!({
                "department": "engineering",
                "starts_at": "2026-07-06T15:00:00Z", "ends_at": "2026-07-06T23:00:00Z",
            }))
            .await
            .json();
        let late: Value = request
            .post("/api/shifts")
            .json(&json!({
                "department": "engineering",
                "starts_at": "2026-07-06T16:00:00Z", "ends_at": "2026-07-06T23:00:00Z",
            }))
            .await
            .json();
        // Assign the early shift; the overlapping one is refused; the
        // back-to-back one is fine.
        request
            .post(&format!(
                "/api/shifts/{}/assignments",
                early["pid"].as_str().unwrap()
            ))
            .json(&json!({ "employee_pid": employee }))
            .await
            .assert_status_ok();
        let double = request
            .post(&format!(
                "/api/shifts/{}/assignments",
                overlapping["pid"].as_str().unwrap()
            ))
            .json(&json!({ "employee_pid": employee }))
            .await;
        assert_eq!(double.status_code(), 422, "double booking refused");
        request
            .post(&format!(
                "/api/shifts/{}/assignments",
                late["pid"].as_str().unwrap()
            ))
            .json(&json!({ "employee_pid": employee }))
            .await
            .assert_status_ok();
        // Approved leave blocks a same-day assignment.
        request
            .post(&format!("/api/employees/{employee}/leave-entitlements"))
            .json(&json!({ "kind": "annual", "year": 2026, "entitled_days": 10 }))
            .await
            .assert_status_ok();
        let leave: Value = request
            .post(&format!("/api/employees/{employee}/leave-requests"))
            .json(&json!({ "kind": "annual", "start_on": "2026-07-10", "end_on": "2026-07-10" }))
            .await
            .json();
        request
            .post(&format!(
                "/api/leave-requests/{}/approve",
                leave["pid"].as_str().unwrap()
            ))
            .await
            .assert_status_ok();
        let on_leave_shift: Value = request
            .post("/api/shifts")
            .json(&json!({
                "department": "engineering",
                "starts_at": "2026-07-10T08:00:00Z", "ends_at": "2026-07-10T16:00:00Z",
            }))
            .await
            .json();
        let conflicted = request
            .post(&format!(
                "/api/shifts/{}/assignments",
                on_leave_shift["pid"].as_str().unwrap()
            ))
            .json(&json!({ "employee_pid": employee }))
            .await;
        assert_eq!(
            conflicted.status_code(),
            422,
            "assignment over approved leave refused"
        );
    })
    .await;
}

/// Working-time guardrails (WPM-R27): the 17-week 48-hour average over
/// recorded minutes and the 11-hour rest gap flag — advisory only, and
/// scoped by the department filter.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn working_time_guardrails() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let heavy = seed_employee(&request, &org, "WT-1", None).await;
        activate(&request, &heavy).await;
        let light = seed_employee(&request, &org, "WT-2", None).await;
        activate(&request, &light).await;

        // 35 recorded 24-hour days inside the 17-week window ending
        // 2026-07-10 — 50 400 min, over the 48 960-min ceiling. Unapproved
        // on purpose: a safety signal must not wait for approval.
        let mut day = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        for _ in 0..35 {
            request
                .post(&format!("/api/employees/{heavy}/time-entries"))
                .json(&json!({ "worked_on": day, "minutes": 1440 }))
                .await
                .assert_status_ok();
            day += chrono::Duration::days(1);
        }
        // The light employee records one modest day.
        request
            .post(&format!("/api/employees/{light}/time-entries"))
            .json(&json!({ "worked_on": "2026-07-01", "minutes": 480 }))
            .await
            .assert_status_ok();

        // A 10-hour turnaround: 14:00-22:00 then 08:00-16:00 next day.
        for (starts, ends) in [
            ("2026-07-05T14:00:00Z", "2026-07-05T22:00:00Z"),
            ("2026-07-06T08:00:00Z", "2026-07-06T16:00:00Z"),
        ] {
            let shift: Value = request
                .post("/api/shifts")
                .json(&json!({ "department": "engineering", "starts_at": starts,
                               "ends_at": ends, "required_headcount": 1 }))
                .await
                .json();
            let shift_pid = shift["pid"].as_str().unwrap();
            request
                .post(&format!("/api/shifts/{shift_pid}/assignments"))
                .json(&json!({ "employee_pid": heavy }))
                .await
                .assert_status_ok();
        }

        let signals: Value = request
            .get("/api/workforce/working-time?as_of=2026-07-10")
            .await
            .json();
        assert!(signals["employees_checked"].as_u64().unwrap() >= 2);
        let flagged = signals["flagged"].as_array().unwrap();
        let row = flagged
            .iter()
            .find(|f| f["employee_pid"] == heavy.as_str())
            .expect("heavy worker flagged");
        assert_eq!(row["over_48h"], true);
        assert_eq!(row["average_weekly"]["numerator_minutes"], 50_400);
        assert_eq!(row["average_weekly"]["denominator_weeks"], 17);
        assert_eq!(row["rest_breaches"].as_array().unwrap().len(), 1);
        assert_eq!(row["rest_breaches"][0]["gap_minutes"], 600);
        assert!(
            !flagged.iter().any(|f| f["employee_pid"] == light.as_str()),
            "a modest week is not flagged"
        );
        // The department filter scopes the check.
        let scoped: Value = request
            .get("/api/workforce/working-time?department=finance&as_of=2026-07-10")
            .await
            .json();
        assert_eq!(scoped["employees_checked"], 0);
        assert!(scoped["flagged"].as_array().unwrap().is_empty());
    })
    .await;
}
