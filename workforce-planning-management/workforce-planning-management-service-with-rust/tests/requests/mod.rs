//! Request-level test suites: the hire journey + employee lifecycle
//! in [`hr`], the time / leave / shift flows in [`workforce`], the
//! payroll derivation in [`payroll`], the L&D surface in [`learning`],
//! the aptitude / personality / psychometric / selection tests in
//! [`assessments`], and the talent-strategy surface (development plans,
//! pipelines, early careers, workforce intelligence) in [`talent`].

mod adjustments;
mod appraisals;
mod assessments;
mod ergonomics;
mod hr;
mod learning;
mod payroll;
mod privacy;
mod talent;
mod wellbeing;
mod workforce;

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
/// `onboarding` unless a test activates it). Expands to a value
/// (`String`, the employee pid), await it like a function call.
///
/// A macro, not a function: the `request` handed to each test by
/// loco's `request()` helper is a `loco_rs`-internal-pinned
/// `axum_test::TestServer` — a type this crate cannot name directly
/// without pulling in that exact same `axum-test` version as a direct
/// dependency (defeating the point of tracking a newer one for the
/// dev-dependency's own sake). Expanding inline sidesteps naming the
/// type at all; the actual value's type is inferred at each call site.
macro_rules! seed_employee {
    ($request:expr, $org:expr, $number:expr, $salary_minor:expr) => {
        async {
            let request = $request;
            let org = $org;
            let number = $number;
            let salary_minor: Option<i64> = $salary_minor;
            let body = serde_json::json!({
                "person_ref": $crate::requests::a_person(),
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
            let created: serde_json::Value =
                request.post("/api/employees").json(&body).await.json();
            created["pid"].as_str().expect("employee pid").to_string()
        }
    };
}
pub(crate) use seed_employee;

/// Activate an employee (no mandatory onboarding items exist in the
/// test flows unless a test adds them). Same macro rationale as
/// [`seed_employee`].
macro_rules! activate {
    ($request:expr, $pid:expr) => {
        async {
            let request = $request;
            let pid = $pid;
            let response = request
                .post(&format!("/api/employees/{pid}/status"))
                .json(&serde_json::json!({ "to": "active" }))
                .await;
            assert_eq!(response.status_code(), 200, "activate {pid}");
        }
    };
}
pub(crate) use activate;
