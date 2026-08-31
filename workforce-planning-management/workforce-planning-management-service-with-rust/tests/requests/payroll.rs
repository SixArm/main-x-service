//! Payroll derivation (WPM-R13, WPM-R14): draft → calculate →
//! approve → paid with reconciled payslips, the approved-run
//! immutability, and the benchmark comparison flags.

use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use workforce_planning_management_service::app::App;
use workforce_planning_management_service::rules::payroll as rules;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Two salaried employees (one with a pension enrolment and approved
// overtime) get reconciled payslips; approve freezes the run.
async fn payroll_run_derives_reconciled_payslips() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let plain = seed_employee!(&request, &org, "E-2001", Some(3_600_000)).await;
        let enrolled = seed_employee!(&request, &org, "E-2002", Some(4_800_000)).await;
        activate!(&request, &plain).await;
        activate!(&request, &enrolled).await;
        // A pension plan + enrolment for the second employee.
        let plan: Value = request
            .post("/api/benefit-plans")
            .json(&json!({
                "name": "Pension 5%", "kind": "pension", "provider": "Demo Provider",
                "employee_cost_minor": 15_000, "employer_cost_minor": 30_000,
                "currency": "GBP",
            }))
            .await
            .json();
        request
            .post(&format!("/api/employees/{enrolled}/benefit-enrollments"))
            .json(&json!({ "plan_pid": plan["pid"], "starts_on": "2026-01-01" }))
            .await
            .assert_status_ok();
        // Approved overtime inside the period: a full extra contracted
        // day (450 regular + 450 more = 450 overtime minutes).
        let entry: Value = request
            .post(&format!("/api/employees/{enrolled}/time-entries"))
            .json(&json!({ "worked_on": "2026-07-06", "minutes": 900 }))
            .await
            .json();
        request
            .post(&format!(
                "/api/time-entries/{}/approve",
                entry["pid"].as_str().unwrap()
            ))
            .await
            .assert_status_ok();
        // Unapproved time must NOT count: another 900-minute day left
        // in `recorded`.
        request
            .post(&format!("/api/employees/{enrolled}/time-entries"))
            .json(&json!({ "worked_on": "2026-07-07", "minutes": 900 }))
            .await
            .assert_status_ok();
        // Draft run over July.
        let run: Value = request
            .post("/api/payroll-runs")
            .json(&json!({
                "organization_ref": org,
                "period_start": "2026-07-01", "period_end": "2026-07-31",
            }))
            .await
            .json();
        let run_pid = run["pid"].as_str().unwrap().to_string();
        // Approve from draft is an illegal transition.
        let premature = request
            .post(&format!("/api/payroll-runs/{run_pid}/approve"))
            .await;
        assert_eq!(premature.status_code(), 422);
        // Calculate.
        let calculated: Value = request
            .post(&format!("/api/payroll-runs/{run_pid}/calculate"))
            .await
            .json();
        assert_eq!(calculated["payslips"], 2);
        let payslips: Value = request
            .get(&format!("/api/payroll-runs/{run_pid}/payslips"))
            .await
            .json();
        let payslips = payslips.as_array().unwrap();
        assert_eq!(payslips.len(), 2);
        for slip in payslips {
            let gross = slip["gross_minor"].as_i64().unwrap();
            let net = slip["net_minor"].as_i64().unwrap();
            let deductions: i64 = slip["deductions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["amount_minor"].as_i64().unwrap())
                .sum();
            assert_eq!(net, gross - deductions, "payslip reconciles");
        }
        // The enrolled employee's slip: base 400000 + overtime
        // (450 min × 400000 / 9750) + pension deduction present.
        let enrolled_slip = payslips
            .iter()
            .find(|s| s["employee_pid"].as_str() == Some(enrolled.as_str()))
            .unwrap();
        let expected_base = 400_000;
        let expected_overtime = rules::overtime_pay_minor(expected_base, 450).unwrap();
        assert_eq!(
            enrolled_slip["gross_minor"].as_i64().unwrap(),
            expected_base + expected_overtime,
            "only approved overtime feeds the gross"
        );
        assert!(
            enrolled_slip["deductions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["label"] == "Pension 5%"),
            "benefit employee-cost is a deduction line"
        );
        // Approve → immutable (no recalculate, no reopen); then paid.
        request
            .post(&format!("/api/payroll-runs/{run_pid}/approve"))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/payroll-runs/{run_pid}/calculate"))
                .await
                .status_code(),
            422,
            "approved runs are immutable"
        );
        assert_eq!(
            request
                .post(&format!("/api/payroll-runs/{run_pid}/reopen"))
                .await
                .status_code(),
            422
        );
        request
            .post(&format!("/api/payroll-runs/{run_pid}/pay"))
            .await
            .assert_status_ok();
        // Self-service payslips list for the employee.
        let mine: Value = request
            .get(&format!("/api/employees/{enrolled}/payslips"))
            .await
            .json();
        assert_eq!(mine.as_array().unwrap().len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Benchmarks: band validation, and the comparison flags employees
// below/within/above without echoing salary amounts.
async fn benchmark_comparison_flags() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let low = seed_employee!(&request, &org, "E-3001", Some(2_000_000)).await;
        let high = seed_employee!(&request, &org, "E-3002", Some(9_900_000)).await;
        let bad_band = request
            .post("/api/benchmarks")
            .json(&json!({
                "job_title": "Engineer", "currency": "GBP",
                "min_minor": 500, "median_minor": 400, "max_minor": 600,
                "source": "survey", "as_of": "2026-04-01",
            }))
            .await;
        assert_eq!(bad_band.status_code(), 422, "min<=median<=max enforced");
        request
            .post("/api/benchmarks")
            .json(&json!({
                "job_title": "Engineer", "currency": "GBP",
                "min_minor": 3_000_000, "median_minor": 3_800_000, "max_minor": 4_600_000,
                "source": "survey", "as_of": "2026-04-01",
            }))
            .await
            .assert_status_ok();
        let comparison: Value = request
            .get(&format!("/api/benchmarks/comparison?organization={org}"))
            .await
            .json();
        let rows = comparison["rows"].as_array().unwrap();
        let flag_of = |pid: &str| {
            rows.iter()
                .find(|r| r["employee_pid"].as_str() == Some(pid))
                .and_then(|r| r["flag"].as_str().map(ToString::to_string))
        };
        assert_eq!(flag_of(&low).as_deref(), Some("below_min"));
        assert_eq!(flag_of(&high).as_deref(), Some("above_max"));
        // No salary amounts in the comparison payload.
        assert!(
            !comparison.to_string().contains("salary_minor"),
            "comparison echoes flags, not amounts"
        );
    })
    .await;
}
