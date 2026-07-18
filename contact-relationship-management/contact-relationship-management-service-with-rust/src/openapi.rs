//! Hand-written `OpenAPI` 3 description of the CRM REST API.
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
            "title": "Contact Relationship Management Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "CRM across four modules: sales automation (contacts/accounts over person:/organization: URNs, rule-scored leads, deal pipelines with Kanban, stage-weighted forecasting), consent-first marketing automation (segments, campaigns, nurture), support (tickets + SLA + knowledge base), analytics (derived, per-currency, honest ratios). Money is minor units + ISO-4217. Validation failures return 422. API version is negotiated with the Accepts-version header (1.0)."
        },
        "paths": {
            "/api/contacts": {
                "post": { "tags": ["relationships"], "summary": "Create a contact (person: URN wrapper; consent starts never)", "responses": created },
                "get": { "tags": ["relationships"], "summary": "List contacts", "responses": ok("Contacts") }
            },
            "/api/contacts/{pid}": { "get": { "tags": ["relationships"], "summary": "Contact + merged timeline (activities, deals, tickets)", "responses": ok("ContactDetail") } },
            "/api/contacts/{pid}/repoint": { "post": { "tags": ["relationships"], "summary": "Repoint the wrapper after an upstream merge (reasoned, audited)", "responses": ok("Contact") } },
            "/api/accounts": {
                "post": { "tags": ["relationships"], "summary": "Create an account (organization: URN wrapper)", "responses": created },
                "get": { "tags": ["relationships"], "summary": "List accounts", "responses": ok("Accounts") }
            },
            "/api/accounts/{pid}": { "get": { "tags": ["relationships"], "summary": "Account + contacts + deals", "responses": ok("AccountDetail") } },
            "/api/accounts/{pid}/clv": { "get": { "tags": ["analytics"], "summary": "CLV per currency over won deals", "responses": ok("Clv") } },
            "/api/activities": {
                "post": { "tags": ["relationships"], "summary": "Log an interaction (stamps ticket first-response for the assignee)", "responses": created },
                "get": { "tags": ["relationships"], "summary": "Activity feed (?subject_kind=&subject_pid=)", "responses": ok("Activities") }
            },
            "/api/activities/{pid}/done": { "put": { "tags": ["relationships"], "summary": "Tick a task activity", "responses": ok("Activity") } },
            "/api/leads": {
                "post": { "tags": ["sales"], "summary": "Capture a lead (scored immediately, breakdown returned)", "responses": created },
                "get": { "tags": ["sales"], "summary": "The queue, score-sorted", "responses": ok("Leads") }
            },
            "/api/leads/{pid}": { "get": { "tags": ["sales"], "summary": "Lead + live score breakdown", "responses": ok("LeadDetail") } },
            "/api/leads/{pid}/status": { "post": { "tags": ["sales"], "summary": "Lead transition; converted creates/links the contact (+ optional deal) in one transaction", "responses": transition } },
            "/api/pipelines": {
                "post": { "tags": ["sales"], "summary": "Create a pipeline with ordered stages (probabilities + terminal flags)", "responses": created },
                "get": { "tags": ["sales"], "summary": "Pipelines with stages", "responses": ok("Pipelines") }
            },
            "/api/deals": {
                "post": { "tags": ["sales"], "summary": "Open a deal in the pipeline's first stage (minor units)", "responses": created },
                "get": { "tags": ["sales"], "summary": "Deals (?pipeline=)", "responses": ok("Deals") }
            },
            "/api/deals/{pid}/stage": { "post": { "tags": ["sales"], "summary": "Kanban stage move (pipeline-membership checked; terminal closes; lost needs a reason; row-locked)", "responses": transition } },
            "/api/deals/{pid}/reopen": { "post": { "tags": ["sales"], "summary": "Reasoned reopen of a closed deal", "responses": transition } },
            "/api/forecast": { "get": { "tags": ["sales"], "summary": "Live stage-weighted forecast per currency (derived, never typed)", "responses": ok("Forecast") } },
            "/api/forecast/snapshot": { "post": { "tags": ["sales"], "summary": "Freeze the current roll-up", "responses": ok("Snapshot") } },
            "/api/contacts/{pid}/consent": {
                "post": { "tags": ["marketing"], "summary": "Record consent (withdrawal exits nurture + blocks sends)", "responses": ok("Recorded") },
                "get": { "tags": ["marketing"], "summary": "The append-only history (audited read)", "responses": ok("ConsentEvents") }
            },
            "/api/segments": {
                "post": { "tags": ["marketing"], "summary": "Create a segment (consent ANDed structurally)", "responses": created },
                "get": { "tags": ["marketing"], "summary": "List segments", "responses": ok("Segments") }
            },
            "/api/segments/{pid}/preview": { "get": { "tags": ["marketing"], "summary": "Count + sample before scheduling", "responses": ok("Preview") } },
            "/api/campaigns": {
                "post": { "tags": ["marketing"], "summary": "Create a campaign (email kind; cost in minor units)", "responses": created },
                "get": { "tags": ["marketing"], "summary": "List campaigns", "responses": ok("Campaigns") }
            },
            "/api/campaigns/{pid}/status": { "post": { "tags": ["marketing"], "summary": "Campaign transition (draft→scheduled→running→completed|cancelled)", "responses": transition } },
            "/api/campaigns/{pid}/run": { "post": { "tags": ["marketing"], "summary": "Simulated send (demo mode): consent re-checked at send time; touch activities; deterministic counters", "responses": transition } },
            "/api/campaigns/{pid}/funnel": { "get": { "tags": ["marketing"], "summary": "Funnel + ROI (null on zero cost; per-currency honesty)", "responses": ok("Funnel") } },
            "/api/nurture-sequences": {
                "post": { "tags": ["marketing"], "summary": "Create a drip sequence (ordered delayed steps)", "responses": created },
                "get": { "tags": ["marketing"], "summary": "Sequences + steps + active enrolments", "responses": ok("Sequences") }
            },
            "/api/nurture-sequences/{pid}/enrollments": { "post": { "tags": ["marketing"], "summary": "Enrol a consented contact", "responses": created } },
            "/api/nurture/advance": { "post": { "tags": ["marketing"], "summary": "The idempotent advance sweep (send due steps; complete; exit unconsented)", "responses": ok("Sweep result") } },
            "/api/sla-policies": {
                "post": { "tags": ["support"], "summary": "Set a priority's SLA targets", "responses": created },
                "get": { "tags": ["support"], "summary": "List policies", "responses": ok("Policies") }
            },
            "/api/tickets": {
                "post": { "tags": ["support"], "summary": "Open a ticket (deadlines derive from the priority's policy)", "responses": created },
                "get": { "tags": ["support"], "summary": "The queue with live breach flags (?status=)", "responses": ok("Tickets") }
            },
            "/api/tickets/{pid}": { "get": { "tags": ["support"], "summary": "Ticket + activities", "responses": ok("TicketDetail") } },
            "/api/tickets/{pid}/status": { "post": { "tags": ["support"], "summary": "Ticket transition (resolved stamps resolved_at; reopen clears it)", "responses": transition } },
            "/api/tickets/{pid}/priority": { "put": { "tags": ["support"], "summary": "Audited priority change (re-derives the deadlines)", "responses": ok("Ticket") } },
            "/api/sla/sweep": { "post": { "tags": ["support"], "summary": "Persist breach facts + emit sla_breached once per breach", "responses": ok("Sweep result") } },
            "/api/articles": {
                "post": { "tags": ["support"], "summary": "Draft a KB article", "responses": created },
                "get": { "tags": ["support"], "summary": "Search articles (?q=)", "responses": ok("Articles") }
            },
            "/api/articles/{pid}": { "put": { "tags": ["support"], "summary": "Edit (published edits bump the version)", "responses": ok("Article") } },
            "/api/articles/{pid}/status": { "post": { "tags": ["support"], "summary": "Publish / archive", "responses": transition } },
            "/api/dashboards/sales": { "get": { "tags": ["analytics"], "summary": "Win rate + pipeline by stage (per currency; ETag conditional)", "responses": ok("SalesDashboard") } },
            "/api/dashboards/sla": { "get": { "tags": ["analytics"], "summary": "Open tickets by priority × breach (ETag conditional)", "responses": ok("SlaDashboard") } },
            "/api/dashboards/activity": { "get": { "tags": ["analytics"], "summary": "Activity counts by kind (?days=)", "responses": ok("ActivityDashboard") } },
            "/api/audits/recent": { "get": { "tags": ["audit"], "summary": "Recent audit entries", "responses": ok("Audit entries") } },
            "/api/audits": { "get": { "tags": ["audit"], "summary": "Owner-scoped trail (?owner=&since=)", "responses": ok("Audit entries") } },
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
            "/api/contacts",
            "/api/leads/{pid}/status",
            "/api/deals/{pid}/stage",
            "/api/campaigns/{pid}/run",
            "/api/nurture/advance",
            "/api/tickets/{pid}/priority",
            "/api/dashboards/sales",
            "/api/accounts/{pid}/clv",
        ] {
            assert!(paths.contains_key(p), "missing {p}");
        }
    }
}
