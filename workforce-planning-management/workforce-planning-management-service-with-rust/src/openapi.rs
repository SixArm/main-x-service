//! Hand-written `OpenAPI` 3 description of the WPM REST API.
//!
//! Summary-level by design: every path and verb is present with its
//! request/response essentials; the full field-by-field shapes live in
//! the spec (`../spec/domain-model.md`).

use serde_json::{json, Value};

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
            "title": "Workforce Planning Management Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "WPM across the employee lifecycle: requisitions/ATS, onboarding, employee records + org chart, time & attendance, leave, shifts, benefits, reviews, training, succession, payroll runs with derived payslips, benchmarking. Identities are EntityRef URNs (person:/worker:/organization:/course:). Money is minor units + ISO-4217. Validation failures return 422. API version is negotiated with the Accepts-version header (1.0)."
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
            "/api/workforce/working-time": { "get": { "tags": ["workforce"], "summary": "Advisory working-time guardrails (?department=&as_of=): 17-week 48h average over recorded minutes + 11h rest-gap breaches across recent and planned assignments; flags only, nothing refused (WPM-D19)", "responses": ok("Signals") } },
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
            "/api/succession-plans/{pid}": { "put": { "tags": ["talent"], "summary": "Restate criticality / risk_of_loss / expected vacancy / incumbent", "responses": ok("Plan") } },
            "/api/succession-candidates/{pid}": { "put": { "tags": ["talent"], "summary": "Move a successor's readiness or rank (readiness may go down)", "responses": ok("Candidate") } },
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
            "/api/assessment-instruments": {
                "post": { "tags": ["assessments"], "summary": "Add a test to the catalog (category + the scales it reports; a scale must suit the category)", "responses": created },
                "get": { "tags": ["assessments"], "summary": "The catalog (?category=aptitude|personality|psychometric|selection)", "responses": ok("Instruments") }
            },
            "/api/assessments": { "post": { "tags": ["assessments"], "summary": "Schedule a sitting for a candidate or employee (application-linked sittings must match the candidate)", "responses": created } },
            "/api/assessments/{pid}": {
                "get": { "tags": ["assessments"], "summary": "One sitting + its per-scale results (sensitive: mask honoured, unmasked reads audited)", "responses": ok("Assessment") },
                "delete": { "tags": ["assessments"], "summary": "Withdraw a sitting (soft delete)", "responses": ok("Deleted") }
            },
            "/api/assessments/{pid}/status": { "post": { "tags": ["assessments"], "summary": "Lifecycle move; completing requires ≥1 result and derives expires_on from the instrument validity", "responses": transition } },
            "/api/assessments/{pid}/results": { "post": { "tags": ["assessments"], "summary": "Record a scale result (upsert; percentile 0–100, band derived; scale must suit the category)", "responses": created } },
            "/api/assessments/analytics": { "get": { "tags": ["assessments"], "summary": "Aggregate sittings by category/status + band distribution (no individual scores)", "responses": ok("Analytics") } },
            "/api/applications/{pid}/assessments": { "get": { "tags": ["assessments"], "summary": "The hiring view: one application's sittings, outstanding count, results (mask honoured)", "responses": ok("Assessments") } },
            "/api/candidates/{pid}/assessment-profile": { "get": { "tags": ["assessments"], "summary": "A candidate's profile: current reading per scale, gaps, selection suitability", "responses": ok("Profile") } },
            "/api/employees/{pid}/assessment-profile": { "get": { "tags": ["assessments"], "summary": "An employee's profile (authorized + masked at the employee level)", "responses": ok("Profile") } },
            "/api/employees/{pid}/development-plans": {
                "post": { "tags": ["talent"], "summary": "Open an upskill or reskill plan with its skill steps (reskill must name a target role; upskill must not)", "responses": created },
                "get": { "tags": ["talent"], "summary": "The employee's plans with declared AND verified progress", "responses": ok("Plans") }
            },
            "/api/development-plans/{pid}/status": { "post": { "tags": ["talent"], "summary": "draft→active→completed (completing needs every item resolved)", "responses": transition } },
            "/api/development-plan-items/{pid}": { "put": { "tags": ["talent"], "summary": "Move one step's status (claiming achievement does not change declared proficiency)", "responses": ok("Item") } },
            "/api/talent-pipelines": {
                "post": { "tags": ["talent"], "summary": "Open a pipeline (succession|hiring|early_careers|internal_mobility)", "responses": created },
                "get": { "tags": ["talent"], "summary": "Pipelines with health (live pool excludes placed/exited)", "responses": ok("Pipelines") }
            },
            "/api/talent-pipelines/{pid}": { "get": { "tags": ["talent"], "summary": "One pipeline, its health, and its members", "responses": ok("Pipeline") } },
            "/api/talent-pipelines/{pid}/members": { "post": { "tags": ["talent"], "summary": "Add a candidate or employee at the identified stage (one row per subject)", "responses": created } },
            "/api/pipeline-members/{pid}/stage": { "post": { "tags": ["talent"], "summary": "Stage move; ready may regress to developing", "responses": transition } },
            "/api/early-career-programs": {
                "post": { "tags": ["early-careers"], "summary": "Add an apprenticeship / internship / graduate scheme (an apprenticeship must declare its off-the-job hours)", "responses": created },
                "get": { "tags": ["early-careers"], "summary": "The catalog with placement counts + conversion rate (?kind=)", "responses": ok("Programs") }
            },
            "/api/early-career-programs/{pid}/placements": { "post": { "tags": ["early-careers"], "summary": "Place someone on the programme (offered)", "responses": created } },
            "/api/program-placements/{pid}/hours": { "post": { "tags": ["early-careers"], "summary": "Log off-the-job training hours (active placements only; checked add)", "responses": ok("Placement") } },
            "/api/program-placements/{pid}/status": { "post": { "tags": ["early-careers"], "summary": "offered→active→completed; completing an apprenticeship requires its off-the-job hours", "responses": transition } },
            "/api/employees/{pid}/placements": { "get": { "tags": ["early-careers"], "summary": "One person's placements with hours against the requirement", "responses": ok("Placements") } },
            "/api/workforce-intelligence/overview": { "get": { "tags": ["intelligence"], "summary": "Headcount, FTE, tenure buckets, spans of control (?as_of=)", "responses": ok("Overview") } },
            "/api/workforce-intelligence/capability": { "get": { "tags": ["intelligence"], "summary": "Declared skill coverage + gaps, plans in flight, assessment coverage", "responses": ok("Capability") } },
            "/api/workforce-intelligence/succession": { "get": { "tags": ["intelligence"], "summary": "Bench strength + single points of failure (criticality × risk of loss)", "responses": ok("Succession") } },
            "/api/workforce-intelligence/pipelines": { "get": { "tags": ["intelligence"], "summary": "Pipeline funnel + early-career conversion rates", "responses": ok("Pipelines") } },
            "/api/wellbeing-entitlements": {
                "post": { "tags": ["wellbeing"], "summary": "Add an entitlement rule (kind health|benefit; non-clinical predicates only: age band, departments, job titles; a benefit rule may link a benefit plan; WPM-D17/D18)", "responses": created },
                "get": { "tags": ["wellbeing"], "summary": "The configured entitlement rules (?kind=health|benefit)", "responses": ok("Entitlements") }
            },
            "/api/wellbeing-entitlements/{pid}": {
                "put": { "tags": ["wellbeing"], "summary": "Restate a rule (cohorts change year to year; acknowledgements untouched)", "responses": ok("Entitlement") },
                "delete": { "tags": ["wellbeing"], "summary": "Soft-close a rule (history kept)", "responses": ok("Deleted") }
            },
            "/api/employees/{pid}/wellbeing-prompts": { "get": { "tags": ["wellbeing"], "summary": "The employee's live prompts (self-service, employee-owned; one reminder max per multi-dose course; unknown age fails an age-banded rule; a plan-linked rule is quiet once enrolled)", "responses": ok("Prompts") } },
            "/api/employees/{pid}/wellbeing-acknowledgements": { "post": { "tags": ["wellbeing"], "summary": "Acknowledge a prompt (booked|done|declined|dismissed; a workflow fact, never a vaccination status; audited)", "responses": created } },
            "/api/wellbeing/uptake": { "get": { "tags": ["wellbeing"], "summary": "HR aggregate uptake: counts by response + rate with its terms; no individual appears", "responses": ok("Uptake") } },
            "/api/pulse-surveys": {
                "post": { "tags": ["wellbeing"], "summary": "Open an anonymous pulse survey (name, question, window)", "responses": created },
                "get": { "tags": ["wellbeing"], "summary": "The surveys with their open state", "responses": ok("Surveys") }
            },
            "/api/pulse-surveys/{pid}/responses": { "post": { "tags": ["wellbeing"], "summary": "Submit one anonymous 1-5 score (stored row has no author; actor-less audit; no handle returned; WPM-D20)", "responses": created } },
            "/api/pulse-surveys/{pid}/results": { "get": { "tags": ["wellbeing"], "summary": "K-floored aggregate (k=5): per-department + overall cells, suppressed below the floor (count withheld); counts are responses, not respondents", "responses": ok("Results") } },
            "/api/employees/{pid}/appraisals": {
                "post": { "tags": ["appraisals"], "summary": "Open a draft 360 for the subject (declared competencies; self nomination automatic)", "responses": created },
                "get": { "tags": ["appraisals"], "summary": "The subject's appraisals with nomination/response counts (never content)", "responses": ok("Appraisals") }
            },
            "/api/employees/{pid}/appraisal-requests": { "get": { "tags": ["appraisals"], "summary": "The rater's own pending 360 requests (collecting, nominated, not yet responded; $sub-owned)", "responses": ok("Requests") } },
            "/api/appraisals/{pid}": { "get": { "tags": ["appraisals"], "summary": "Detail: nominations with responded flags -- who responded, never what (WPM-D21)", "responses": ok("Appraisal") } },
            "/api/appraisals/{pid}/nominations": { "post": { "tags": ["appraisals"], "summary": "Invite a rater (draft only; group manager|peer|report; one per rater; max 12)", "responses": created } },
            "/api/appraisals/{pid}/status": { "post": { "tags": ["appraisals"], "summary": "draft -> collecting (needs >= 3 non-self raters) -> shared (stamps shared_on)", "responses": transition } },
            "/api/appraisals/{pid}/responses": { "post": { "tags": ["appraisals"], "summary": "One rater's response: collecting only, nominated only, once per rater, every declared competency scored 1-5 ($sub-owned)", "responses": created } },
            "/api/appraisals/{pid}/report": { "get": { "tags": ["appraisals"], "summary": "Group-floored report (shared only; reads audited): group x competency count+mean, pooled comments; peer/report cells under 3 responses withheld, count included", "responses": ok("Report") } },
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
            "/api/assessments/{pid}/results",
            "/api/employees/{pid}/assessment-profile",
            "/api/employees/{pid}/development-plans",
            "/api/talent-pipelines/{pid}/members",
            "/api/early-career-programs/{pid}/placements",
            "/api/program-placements/{pid}/status",
            "/api/workforce-intelligence/succession",
            "/api/wellbeing-entitlements",
            "/api/employees/{pid}/wellbeing-prompts",
            "/api/employees/{pid}/wellbeing-acknowledgements",
            "/api/wellbeing/uptake",
            "/api/workforce/working-time",
            "/api/pulse-surveys",
            "/api/pulse-surveys/{pid}/results",
            "/api/employees/{pid}/appraisals",
            "/api/appraisals/{pid}/responses",
            "/api/appraisals/{pid}/report",
        ] {
            assert!(paths.contains_key(p), "missing {p}");
        }
    }
}
