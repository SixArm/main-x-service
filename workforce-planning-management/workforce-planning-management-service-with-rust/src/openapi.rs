//! Hand-written `OpenAPI` 3 description of the HCM REST API.
//!
//! Summary-level by design: every path and verb is present with its
//! request/response essentials; the full field-by-field shapes live in
//! the spec (`../spec/domain-model.md`).

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
#[allow(clippy::too_many_lines)] // one literal document
pub fn spec() -> Value {
    let ok = |desc: &str| json!({ "200": { "description": desc } });
    let created = json!({
        "200": { "description": "Created: {pid}" },
        "422": { "description": "Validation failure" }
    });
    let transition = json!({
        "200": { "description": "The updated record" },
        "422": { "description": "Illegal transition (names the current state)" }
    });
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Human Capital Management Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "HCM across the employee lifecycle: requisitions/ATS, onboarding, employee records + org chart, time & attendance, leave, shifts, benefits, reviews, training, succession, payroll runs with derived payslips, benchmarking. Identities are EntityRef URNs (person:/worker:/organization:/course:). Money is minor units + ISO-4217. Validation failures return 422. API version is negotiated with the Accepts-version header (1.0)."
        },
        "paths": {
            "/api/employees": {
                "post": { "tags": ["hr-core"], "summary": "Create an employee (onboarding status; person/organization URNs; salary minor units)", "responses": created },
                "get": { "tags": ["hr-core"], "summary": "List employees (?department=&status=; per-row mask obligation)", "responses": ok("Employees") }
            },
            "/api/employees/{pid}": {
                "get": { "tags": ["hr-core"], "summary": "Fetch an employee (salary read audited; mask honoured)", "responses": ok("Employee") },
                "put": { "tags": ["hr-core"], "summary": "Update employment facts (manager change is cycle-checked)", "responses": ok("Updated employee") },
                "delete": { "tags": ["hr-core"], "summary": "Soft-delete an employee", "responses": ok("Deleted") }
            },
            "/api/employees/{pid}/status": { "post": { "tags": ["hr-core"], "summary": "Employee lifecycle transition (activation gated on mandatory onboarding items)", "responses": transition } },
            "/api/org-chart": { "get": { "tags": ["hr-core"], "summary": "The manager forest for one organization (?organization=)", "responses": ok("OrgNodes") } },
            "/api/benefit-plans": {
                "post": { "tags": ["benefits"], "summary": "Create a benefit plan (minor-unit costs)", "responses": created },
                "get": { "tags": ["benefits"], "summary": "List plans", "responses": ok("Plans") }
            },
            "/api/employees/{pid}/benefit-enrollments": {
                "post": { "tags": ["benefits"], "summary": "Enrol (double-enrolment refused)", "responses": created },
                "get": { "tags": ["benefits"], "summary": "List enrolments", "responses": ok("Enrollments") }
            },
            "/api/benefit-enrollments/{pid}": { "delete": { "tags": ["benefits"], "summary": "Unenrol (soft delete)", "responses": ok("Deleted") } },
            "/api/requisitions": {
                "post": { "tags": ["acquisition"], "summary": "Open a requisition (draft; headcount; salary band)", "responses": created },
                "get": { "tags": ["acquisition"], "summary": "List requisitions (?status=)", "responses": ok("Requisitions") }
            },
            "/api/requisitions/{pid}": { "get": { "tags": ["acquisition"], "summary": "Fetch a requisition", "responses": ok("Requisition") } },
            "/api/requisitions/{pid}/status": { "post": { "tags": ["acquisition"], "summary": "Requisition transition (filled requires hired ≥ headcount)", "responses": transition } },
            "/api/requisitions/{pid}/applications": {
                "post": { "tags": ["acquisition"], "summary": "Apply a candidate (consent-checked)", "responses": created },
                "get": { "tags": ["acquisition"], "summary": "List applications", "responses": ok("Applications") }
            },
            "/api/candidates": {
                "post": { "tags": ["acquisition"], "summary": "Add a candidate (consent_until bounds the pool)", "responses": created },
                "get": { "tags": ["acquisition"], "summary": "The pool (consent-expired excluded; ?expired=1 = purge queue)", "responses": ok("Candidates") }
            },
            "/api/applications/{pid}/stage": { "post": { "tags": ["acquisition"], "summary": "Stage transition; hired creates the employee in the same transaction", "responses": transition } },
            "/api/applications/{pid}/interviews": {
                "post": { "tags": ["acquisition"], "summary": "Schedule an interview (worker: interviewer URN)", "responses": created },
                "get": { "tags": ["acquisition"], "summary": "List interviews", "responses": ok("Interviews") }
            },
            "/api/interviews/{pid}": { "put": { "tags": ["acquisition"], "summary": "Record the outcome (pending|advance|reject)", "responses": ok("Interview") } },
            "/api/employees/{pid}/onboarding": {
                "post": { "tags": ["acquisition"], "summary": "Add checklist items", "responses": created },
                "get": { "tags": ["acquisition"], "summary": "The checklist", "responses": ok("Items") }
            },
            "/api/onboarding-items/{pid}/complete": { "post": { "tags": ["acquisition"], "summary": "Complete an item", "responses": ok("Item") } },
            "/api/onboarding-items/{pid}/waive": { "post": { "tags": ["acquisition"], "summary": "Waive an item (reason required, audited)", "responses": ok("Item") } },
            "/api/employees/{pid}/time-entries": {
                "post": { "tags": ["workforce"], "summary": "Record time (minutes; day capped at 24h)", "responses": created },
                "get": { "tags": ["workforce"], "summary": "Entries + derived per-day overtime (?from=&to=)", "responses": ok("Entries") }
            },
            "/api/time-entries/{pid}/approve": { "post": { "tags": ["workforce"], "summary": "Approve (only approved time feeds payroll)", "responses": ok("Entry") } },
            "/api/employees/{pid}/leave-entitlements": {
                "post": { "tags": ["workforce"], "summary": "Grant an entitlement (kind/year/days)", "responses": created },
                "get": { "tags": ["workforce"], "summary": "Balances", "responses": ok("Entitlements") }
            },
            "/api/employees/{pid}/leave-requests": {
                "post": { "tags": ["workforce"], "summary": "Request leave (annual over-balance 422; sick flags negative)", "responses": created },
                "get": { "tags": ["workforce"], "summary": "List requests", "responses": ok("Requests") }
            },
            "/api/leave-requests/{pid}/approve": { "post": { "tags": ["workforce"], "summary": "Approve (locks the row; decrements the balance in-tx)", "responses": transition } },
            "/api/leave-requests/{pid}/reject": { "post": { "tags": ["workforce"], "summary": "Reject", "responses": transition } },
            "/api/leave-requests/{pid}/cancel": { "post": { "tags": ["workforce"], "summary": "Cancel (approved-cancel restores the balance)", "responses": transition } },
            "/api/shifts": {
                "post": { "tags": ["workforce"], "summary": "Plan a shift", "responses": created },
                "get": { "tags": ["workforce"], "summary": "The rota (?department=&date=; assignments attached)", "responses": ok("Shifts") }
            },
            "/api/shifts/{pid}/assignments": { "post": { "tags": ["workforce"], "summary": "Assign (double-booking and leave-conflict refused)", "responses": created } },
            "/api/shift-assignments/{pid}": { "delete": { "tags": ["workforce"], "summary": "Unassign (soft delete)", "responses": ok("Deleted") } },
            "/api/review-cycles": {
                "post": { "tags": ["development"], "summary": "Open a review cycle", "responses": created },
                "get": { "tags": ["development"], "summary": "List cycles", "responses": ok("Cycles") }
            },
            "/api/review-cycles/{pid}/reviews": { "post": { "tags": ["development"], "summary": "Open a draft review", "responses": created } },
            "/api/employees/{pid}/reviews": { "get": { "tags": ["development"], "summary": "An employee's reviews (unshared content redacted; shared reads audited)", "responses": ok("Reviews") } },
            "/api/reviews/{pid}": {
                "get": { "tags": ["development"], "summary": "Review detail + goals + feedback (content read audited)", "responses": ok("ReviewDetail") },
                "put": { "tags": ["development"], "summary": "Author edits (drafts only; rating 1-5)", "responses": ok("Review") }
            },
            "/api/reviews/{pid}/status": { "post": { "tags": ["development"], "summary": "Review transition (draft→submitted→calibrated→shared)", "responses": transition } },
            "/api/reviews/{pid}/goals": { "post": { "tags": ["development"], "summary": "Add a weighted goal", "responses": created } },
            "/api/goals/{pid}": { "put": { "tags": ["development"], "summary": "Set goal status (open|met|missed)", "responses": ok("Goal") } },
            "/api/reviews/{pid}/feedback": { "post": { "tags": ["development"], "summary": "Add feedback", "responses": created } },
            "/api/employees/{pid}/training-enrollments": {
                "post": { "tags": ["development"], "summary": "Enrol against a course:/courseinstance: URN", "responses": created },
                "get": { "tags": ["development"], "summary": "List enrolments", "responses": ok("Enrollments") }
            },
            "/api/training-enrollments/{pid}": { "put": { "tags": ["development"], "summary": "Progress (completed stamps completion + certificate expiry)", "responses": ok("Enrollment") } },
            "/api/training/expiring": { "get": { "tags": ["development"], "summary": "Certificates expiring (?within_days=, default 90)", "responses": ok("Expiring") } },
            "/api/succession-plans": {
                "post": { "tags": ["development"], "summary": "Create a plan (criticality 1-5)", "responses": created },
                "get": { "tags": ["development"], "summary": "Plans + ranked candidates (read audited)", "responses": ok("Plans") }
            },
            "/api/succession-plans/gaps": { "get": { "tags": ["development"], "summary": "Criticality ≥ 4 roles with no ready_now candidate", "responses": ok("Gaps") } },
            "/api/succession-plans/{pid}/candidates": { "post": { "tags": ["development"], "summary": "Add a ranked candidate (ready_now|ready_1y|ready_2y)", "responses": created } },
            "/api/payroll-runs": {
                "post": { "tags": ["payroll"], "summary": "Open a draft run (organization URN + period)", "responses": created },
                "get": { "tags": ["payroll"], "summary": "List runs", "responses": ok("Runs") }
            },
            "/api/payroll-runs/{pid}": { "get": { "tags": ["payroll"], "summary": "Fetch a run", "responses": ok("Run") } },
            "/api/payroll-runs/{pid}/calculate": { "post": { "tags": ["payroll"], "summary": "Derive payslips (salary × FTE + approved overtime − deductions; stub tax; net invariant enforced)", "responses": transition } },
            "/api/payroll-runs/{pid}/approve": { "post": { "tags": ["payroll"], "summary": "Approve (immutable thereafter)", "responses": transition } },
            "/api/payroll-runs/{pid}/pay": { "post": { "tags": ["payroll"], "summary": "Mark paid", "responses": transition } },
            "/api/payroll-runs/{pid}/reopen": { "post": { "tags": ["payroll"], "summary": "Reopen calculated → draft", "responses": transition } },
            "/api/payroll-runs/{pid}/payslips": { "get": { "tags": ["payroll"], "summary": "The run's payslips (audited; mask honoured)", "responses": ok("Payslips") } },
            "/api/employees/{pid}/payslips": { "get": { "tags": ["payroll"], "summary": "One employee's payslips (self-service; audited; mask honoured)", "responses": ok("Payslips") } },
            "/api/benchmarks": {
                "post": { "tags": ["compensation"], "summary": "Record a market band (min ≤ median ≤ max)", "responses": created },
                "get": { "tags": ["compensation"], "summary": "List benchmarks", "responses": ok("Benchmarks") }
            },
            "/api/benchmarks/comparison": { "get": { "tags": ["compensation"], "summary": "Employees vs bands: below_min|within|above_max flags only (?organization=; audited)", "responses": ok("Comparison") } },
            "/api/audits/recent": { "get": { "tags": ["audit"], "summary": "Recent audit entries", "responses": ok("Audit entries") } },
            "/api/audits": { "get": { "tags": ["audit"], "summary": "Department-scoped trail (?department=&since=)", "responses": ok("Audit entries") } },
            "/api/audits/{entity_pid}": { "get": { "tags": ["audit"], "summary": "One record's audit trail", "responses": ok("Audit entries") } },
            "/api/events/recent": { "get": { "tags": ["events"], "summary": "Recent events (memory ring or outbox)", "responses": ok("Events") } },
            "/metrics.prom": { "get": { "tags": ["ops"], "summary": "Prometheus metrics (public)", "responses": ok("Exposition text") } }
        }
    })
}

#[cfg(test)]
mod tests {
    /// The document parses, declares `OpenAPI` 3, and covers the mounted
    /// API surface (spot-checked against the route table).
    #[test]
    fn spec_shape() {
        let doc = super::spec();
        assert_eq!(doc["openapi"], "3.0.3");
        let paths = doc["paths"].as_object().unwrap();
        for p in [
            "/api/employees",
            "/api/employees/{pid}/status",
            "/api/applications/{pid}/stage",
            "/api/leave-requests/{pid}/approve",
            "/api/payroll-runs/{pid}/calculate",
            "/api/benchmarks/comparison",
            "/api/succession-plans/gaps",
        ] {
            assert!(paths.contains_key(p), "missing {p}");
        }
    }
}
