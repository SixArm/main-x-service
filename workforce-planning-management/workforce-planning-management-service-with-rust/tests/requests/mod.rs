//! Request-level test suites: the hire journey + employee lifecycle
//! in [`hr`], the time / leave / shift flows in [`workforce`], the
//! payroll derivation in [`payroll`], the L&D surface in [`learning`],
//! the aptitude / personality / psychometric / selection tests in
//! [`assessments`], and the talent-strategy surface (development plans,
//! pipelines, early careers, workforce intelligence) in [`talent`].

mod appraisals;
mod assessments;
mod hr;
mod learning;
mod payroll;
mod privacy;
mod talent;
mod wellbeing;
mod workforce;

use serde_json::{Value, json};

/// A fresh synthetic `person:` URN.
pub fn a_person() -> String {
    format!("person:{}", uuid::Uuid::new_v4())
}

/// A fresh synthetic `organization:` URN.
pub fn an_org() -> String {
    format!("organization:{}", uuid::Uuid::new_v4())
}

/// A fresh synthetic `worker:` URN.
pub fn a_worker() -> String {
    format!("worker:{}", uuid::Uuid::new_v4())
}

/// Create one active employee (no onboarding gate: the status stays
/// `onboarding` unless a test activates it). Returns the pid.
pub async fn seed_employee(
    request: &axum_test::TestServer,
    org: &str,
    number: &str,
    salary_minor: Option<i64>,
) -> String {
    let body = json!({
        "person_ref": a_person(),
        "organization_ref": org,
        "employee_number": number,
        "display_name": format!("Test Employee {number}"),
        "employment_type": "permanent",
        "department": "engineering",
        "job_title": "Engineer",
        "salary_minor": salary_minor,
        "salary_currency": salary_minor.map(|_| "GBP"),
        "hired_on": "2026-01-05",
    });
    let created: Value = request.post("/api/employees").json(&body).await.json();
    created["pid"].as_str().expect("employee pid").to_string()
}

/// Activate an employee (no mandatory onboarding items exist in the
/// test flows unless a test adds them).
pub async fn activate(request: &axum_test::TestServer, pid: &str) {
    let response = request
        .post(&format!("/api/employees/{pid}/status"))
        .json(&json!({ "to": "active" }))
        .await;
    assert_eq!(response.status_code(), 200, "activate {pid}");
}
