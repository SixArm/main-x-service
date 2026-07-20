//! Hand-written `OpenAPI` 3 description of the portfolio REST API.
//!
//! The request/response `WorkItem` body is the `project_portfolio_management_matcher::WorkItem`
//! shape. That crate is intentionally dependency-light (no `utoipa`), so
//! the schema is authored here by hand rather than derived — which also
//! keeps the doc accurate to the wire format. The four collections
//! (portfolios / projects / products / programs) share one templated path
//! set under `/api/{collection}`.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Portfolio Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of work-item identities across four collections (portfolios / projects / products / programs): CRUD + within-collection matching. The request/response body is the project-portfolio-management-matcher WorkItem shape. Validation failures (blank name, blank goal title, malformed identifier, malformed portfolio_ref / in_language) return 422. Matching never crosses collections (the matcher's kind gate)."
        },
        "paths": paths(),
        "components": components(),
    })
}

/// The `paths` object, composed from the CRUD/matching paths and the
/// auxiliary (auth/audit/events/metrics) paths.
fn paths() -> Value {
    let mut paths = crud_paths();
    merge_object(&mut paths, aux_paths());
    merge_object(&mut paths, insight_paths());
    paths
}

/// The executive-insight read paths (CEO / CFO / CTO derived views;
/// all ETag-conditional GETs with an `as_of` stamp).
fn insight_paths() -> Value {
    let get = |tag: &str, summary: &str| {
        json!({ "get": { "tags": [tag], "summary": summary,
            "responses": { "200": { "description": "Derived view (ETag-conditional; carries as_of)" },
                           "304": { "description": "Not modified" } } } })
    };
    json!({
        "/api/executive/health": get("executive", "CEO portfolio-health briefing: per-portfolio RAG rollup, overdue milestones, escalated risks, exposure, overrun currencies, staleness"),
        "/api/executive/decisions": get("executive", "Decision log: gate reviews, scenario commits, decided proposals, merges (newest first; ?limit=)"),
        "/api/executive/benefits": get("executive", "Benefits realization per portfolio: per-currency target vs realized (ratio only with a positive target) + non-financial status counts"),
        "/api/financials/variance": get("financials", "Budget variance (minor units, per currency, never merged) by collection, portfolio, and category"),
        "/api/financials/exposure": get("financials", "Per-currency estate exposure: planned / actual / remaining totals; deliberately no FX conversion"),
        "/api/technology/dependency-risk": get("technology", "Dependency lens: top fan-out items, cross-portfolio edges, edges with a RAG-red predecessor"),
        "/api/technology/radar": get("technology", "Technology radar from tech:<name>[:<ring>] tags; majority ring vote, ties break cautious"),
        "/api/executive/alignment": get("executive", "Strategic-alignment coverage: per-collection aligned/unaligned counts, unaligned spend per currency, ranked unaligned items"),
        "/api/technology/debt": get("technology", "Technical-debt register: risks categorised tech_debt, exposure-sorted, with status counts"),
        "/api/technology/flow": get("technology", "Delivery-flow metrics: milestone throughput per month + median lead days (?months=, cap 24)"),
        "/api/scenarios/compare": get("executive", "Side-by-side scenario comparison (?a=&b=): live evaluations + per-currency planned deltas, exposure/alignment deltas"),
        "/api/board/pack": get("oversight", "Board pack (?from=&to=): window decisions, benefits realized, milestones completed, tranches released + as-of-now health"),
        "/api/board/investments": get("oversight", "Money-moving decisions: scenario commits, tranche releases, approved proposals (newest first, cap 100)"),
        "/api/board/trends": get("oversight", "Stored estate-snapshot series (oldest first; no interpolated history)"),
        "/api/auditor/trail": get("oversight", "Audit explorer (?actor=&action=&entity=&from=&to=&limit=): filtered rows + integrity stats"),
        "/api/auditor/findings": get("oversight", "Segregation-of-duties + hygiene findings over recorded audit actors"),
        "/api/auditor/evidence-pack": get("oversight", "Period evidence bundle (?from=&to=&format=json|csv): audit rows + decisions"),
        "/api/compliance/register": get("oversight", "Compliance risk register (category=compliance, exposure-sorted)"),
        "/api/compliance/findings": get("oversight", "Conformance findings (?review_days=): overdue-unreviewed items, ownerless escalations, overdue reviews, approver-less approvals"),
        "/api/risk/heatmap": get("oversight", "CRO heatmap: probability x impact cells, top risks, posture, concentration, hygiene, declared appetite + breaches"),
        "/api/security/register": get("oversight", "Security risk register + the no-security-risk-at-late-stage heuristic"),
        "/api/regulator/extract": get("oversight", "Deliberately coarse per-portfolio aggregates; ABAC mask obligation withholds names"),
        "/api/{collection}/{pid}/tasks": {
            "parameters": [collection_param(), { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "get": { "tags": ["engineering"], "summary": "The item's live tasks + per-status board counts", "responses": { "200": { "description": "Tasks + counts" } } },
            "post": { "tags": ["engineering"], "summary": "Create a task (default status todo; done on create stamps done_at)",
                "responses": { "200": { "description": "The task" }, "422": { "description": "Validation failure" } } }
        },
        "/api/{collection}/{pid}/tasks/{t_pid}": {
            "parameters": [collection_param(),
                { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } },
                { "name": "t_pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "put": { "tags": ["engineering"], "summary": "Update task fields (status changes go through PATCH)", "responses": { "200": { "description": "The task" }, "422": { "description": "Validation failure" } } },
            "patch": { "tags": ["engineering"], "summary": "Board move: {status}; stamps status_changed_at, first done stamps done_at", "responses": { "200": { "description": "The task" }, "422": { "description": "Unknown status" } } },
            "delete": { "tags": ["engineering"], "summary": "Soft-delete the task", "responses": { "200": { "description": "Deleted" } } }
        },
        "/api/{collection}/{pid}/sprints": {
            "parameters": [collection_param(), { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "get": { "tags": ["engineering"], "summary": "The item's sprints", "responses": { "200": { "description": "Sprints" } } },
            "post": { "tags": ["engineering"], "summary": "Create a time-boxed sprint", "responses": { "200": { "description": "The sprint" }, "422": { "description": "ends_on before starts_on" } } }
        },
        "/api/{collection}/{pid}/burndown": get("engineering", "Honest sprint burndown (?sprint=): remaining per day from real done_at stamps; no ideal line"),
        "/api/{collection}/{pid}/standup": get("engineering", "Last-24h digest: tasks created/moved, current blockers (audit-derived)"),
        "/api/{collection}/{pid}/velocity": get("engineering", "Per-sprint completed counts + story-point sums (points are team-local; item-own trend only)"),
        "/api/{collection}/{pid}/sprints/{s_pid}/notes": {
            "get": { "tags": ["engineering"], "summary": "The sprint's retro/feedback notes", "responses": { "200": { "description": "Notes" } } },
            "post": { "tags": ["engineering"], "summary": "Add a note (went_well/improve/action/feedback)", "responses": { "200": { "description": "The note" }, "422": { "description": "Unknown category / blank body" } } }
        },
        "/api/{collection}/{pid}/sprints/{s_pid}/notes/{n_pid}/convert": { "post": { "tags": ["engineering"],
            "summary": "Convert an action/feedback note into a task (once)",
            "responses": { "200": { "description": "The note + created task" }, "422": { "description": "Not convertible / already converted" } } } },
        "/api/devops/events": { "post": { "tags": ["devops"],
            "summary": "Ingest a deploy/incident/recovery event (deploy needs environment; recovery references its incident)",
            "responses": { "200": { "description": "The stored event" }, "422": { "description": "Validation failure" } } } },
        "/api/devops/metrics": get("devops", "DORA-style metrics from ingested events only (?months=): deploys, incidents, MTTR over linked pairs, declared-cause change failure"),
        "/api/devops/releases": get("devops", "Release register: ingested deploy events, newest first (cap 200)"),
        "/api/engineering/blocked": get("engineering", "Estate blocked-work aging (days since entering blocked)"),
        "/api/engineering/moscow": get("engineering", "MoSCoW scope cut from moscow:<band> tags (untagged counted, never guessed)"),
        "/api/engineering/delivery-links": get("engineering", "External delivery-tracker identifiers per item + the untracked list"),
        "/api/engineering/milestone-calendar": get("engineering", "Estate milestone calendar (?kind=milestone|demo|release|checkpoint)"),
        "/api/board/snapshots": { "post": { "tags": ["oversight"],
            "summary": "Capture one estate snapshot now (portfolio counts, open exposure, per-currency money)",
            "responses": { "200": { "description": "The stored snapshot row" } } } },
    })
}

/// Shallow-merge the top-level keys of `src` into `dst`.
fn merge_object(dst: &mut Value, src: Value) {
    if let (Some(dst), Value::Object(src)) = (dst.as_object_mut(), src) {
        for (k, v) in src {
            dst.insert(k, v);
        }
    }
}

/// The `collection` path parameter shared by every templated path.
fn collection_param() -> Value {
    json!({
        "name": "collection", "in": "path", "required": true,
        "schema": { "type": "string", "enum": ["portfolios", "projects", "products", "programs"] }
    })
}

/// The CRUD + matching + merge paths (templated over `{collection}`).
fn crud_paths() -> Value {
    json!({
            "/api/{collection}": {
                "parameters": [collection_param()],
                "get": {
                    "tags": ["work-items"],
                    "summary": "List active work items in the collection (cap 100; ?portfolio= rolls up children)",
                    "responses": { "200": { "description": "List of references",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/WorkItemRef" } } } } } }
                },
                "post": {
                    "tags": ["work-items"],
                    "summary": "Create a work item in the collection",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } },
                    "responses": {
                        "200": { "description": "Created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItemRef" } } } },
                        "422": { "description": "Validation failure: blank name, kind not matching the collection, or a malformed goal/identifier/portfolio_ref/in_language" }
                    }
                }
            },
            "/api/{collection}/search": {
                "parameters": [collection_param()],
                "get": {
                    "tags": ["work-items"],
                    "summary": "Case-insensitive name search within the collection (Postgres ILIKE, cap 50)",
                    "parameters": [{ "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }],
                    "responses": {
                        "200": { "description": "Matches", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/WorkItemRef" } } } } },
                        "400": { "description": "Missing or blank `q`" }
                    }
                }
            },
            "/api/{collection}/match": {
                "parameters": [collection_param()],
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list (no persistence; cross-kind candidates score 0.0)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/{collection}/check-duplicates": {
                "parameters": [collection_param()],
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored work items in the collection",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/{collection}/merge": {
                "parameters": [collection_param()],
                "post": {
                    "tags": ["matching"],
                    "summary": "Merge a confirmed duplicate into a surviving work item (same collection)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MergeRequest" } } } },
                    "responses": {
                        "200": { "description": "The survivor's merged payload + the merged pids" },
                        "404": { "description": "main_pid or duplicate_pid not found" },
                        "422": { "description": "main_pid and duplicate_pid are equal" }
                    }
                }
            },
            "/api/{collection}/merges/recent": {
                "parameters": [collection_param()],
                "get": { "tags": ["matching"], "summary": "Recent merge-history records", "responses": { "200": { "description": "Merge records" } } }
            }
    })
}

/// The auth / audit / events / single-record / metrics paths.
fn aux_paths() -> Value {
    json!({
            "/api/{collection}/whoami": {
                "parameters": [collection_param()],
                "get": {
                    "tags": ["auth"],
                    "summary": "Echo the verified claims of the bearer token",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Verified token claims" },
                        "401": { "description": "Missing or invalid bearer token" }
                    }
                }
            },
            "/api/{collection}/audit/recent": {
                "parameters": [collection_param()],
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/{collection}/events/recent": {
                "parameters": [collection_param()],
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/{collection}/{pid}": {
                "parameters": [collection_param(), { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["work-items"], "summary": "Fetch the stored work item",
                    "responses": { "200": { "description": "WorkItem", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["work-items"], "summary": "Replace a work item's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItemRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure" } } },
                "delete": { "tags": ["work-items"], "summary": "Soft-delete a work item", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/{collection}/{pid}/audit": {
                "parameters": [collection_param(), { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["audit"], "summary": "Audit trail for one work item", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/metrics.prom": {
                "get": {
                    "tags": ["observability"],
                    "summary": "Prometheus metrics (text-exposition format)",
                    "description": "Process-wide metric registry rendered as text/plain; version=0.0.4. Mounted at the root (not under /api) and public even under blanket auth enforcement. Configure your scraper with metrics_path: /metrics.prom.",
                    "responses": { "200": { "description": "Prometheus text exposition",
                        "content": { "text/plain": { "schema": { "type": "string" } } } } }
                }
            }
    })
}

/// The `components` object of the `OpenAPI` document.
fn components() -> Value {
    json!({
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "PASETO",
                    "description": "Short-lived PASETO v4.public token from the authentication-service, verified offline against its published Ed25519 key." }
            },
            "schemas": {
                "WorkItemRef": { "type": "object", "required": ["pid", "name"], "properties": {
                    "pid": { "type": "string", "format": "uuid" }, "name": { "type": "string" } } },
                "ScoredRef": { "type": "object", "properties": {
                    "pid": { "type": "string" }, "name": { "type": "string" },
                    "score": { "type": "number", "format": "double" }, "confidence": { "type": "string" },
                    "is_match": { "type": "boolean" } } },
                "MatchRequest": { "type": "object", "required": ["query", "candidates"], "properties": {
                    "query": { "$ref": "#/components/schemas/WorkItem" },
                    "candidates": { "type": "array", "items": { "$ref": "#/components/schemas/WorkItem" } } } },
                "MergeRequest": { "type": "object", "required": ["main_pid", "duplicate_pid"], "properties": {
                    "main_pid": { "type": "string", "format": "uuid" },
                    "duplicate_pid": { "type": "string", "format": "uuid" },
                    "reason": { "type": "string", "nullable": true } } },
                "WorkItemIdentifier": { "type": "object", "required": ["scheme", "value"], "properties": {
                    "scheme": { "description": "Uri | Uuid | JiraProjectKey | AsanaGid | TrelloBoardId | MsProjectId | GitHubProjectId | LinearId | Code | LocalId | {Custom: string}" },
                    "value": { "type": "string", "description": "Must be non-blank." } } },
                "WorkItem": { "type": "object", "required": ["kind", "name"], "properties": {
                    "kind": { "type": "string", "enum": ["Portfolio", "Project", "Product", "Program"], "description": "The collection; must match the path collection" },
                    "name": { "type": "string" },
                    "alternate_names": { "type": "array", "items": { "type": "string" } },
                    "code": { "type": "string", "nullable": true, "description": "Owner-scoped code, e.g. PROJ-2026" },
                    "owner_org_id": { "type": "string", "nullable": true, "description": "EntityRef organization:<id>" },
                    "owner_org_name": { "type": "string", "nullable": true },
                    "lead_ref": { "type": "string", "nullable": true, "description": "EntityRef person:<id> | worker:<id>" },
                    "portfolio_ref": { "type": "string", "nullable": true, "description": "Parent portfolio pid (child kinds; UUID)" },
                    "status": { "type": "string", "nullable": true, "description": "Proposed | Active | OnHold | Completed | Cancelled | {Custom: string}" },
                    "goals": { "type": "array", "items": { "type": "object", "properties": { "title": { "type": "string" }, "description": { "type": "string", "nullable": true }, "target_date": { "type": "string", "nullable": true }, "status": { "type": "string", "nullable": true } } } },
                    "start_date": { "type": "string", "nullable": true, "description": "ISO-8601 YYYY / YYYY-MM / YYYY-MM-DD" },
                    "target_date": { "type": "string", "nullable": true },
                    "keywords": { "type": "array", "items": { "type": "string" } },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "identifiers": { "type": "array", "items": { "$ref": "#/components/schemas/WorkItemIdentifier" } },
                    "same_as": { "type": "array", "items": { "type": "string" } },
                    "in_language": { "type": "string", "nullable": true, "description": "BCP-47 language tag" },
                    "relationships": { "type": "array", "items": { "type": "object", "properties": { "relation": { "type": "string" }, "work_item_id": { "type": "string" } } } } } }
            }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the document's basic shape and the core schemas.
    #[test]
    fn spec_is_wellformed() {
        let s = spec();
        assert_eq!(s["openapi"], "3.0.3");
        assert!(s["paths"]["/api/{collection}"]["post"].is_object());
        assert!(s["paths"]["/api/{collection}/check-duplicates"]["post"].is_object());
        assert!(s["components"]["schemas"]["WorkItem"]["properties"]["name"].is_object());
        assert!(s["components"]["schemas"]["WorkItem"]["properties"]["kind"].is_object());
        assert!(
            s["components"]["schemas"]["WorkItemIdentifier"]["properties"]["value"].is_object()
        );
    }

    /// Pins that the seven core CRUD + matching operations are documented.
    #[test]
    fn spec_documents_core_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/{collection}"]["get"].is_object());
        assert!(paths["/api/{collection}"]["post"].is_object());
        assert!(paths["/api/{collection}/match"]["post"].is_object());
        assert!(paths["/api/{collection}/check-duplicates"]["post"].is_object());
        assert!(paths["/api/{collection}/{pid}"]["get"].is_object());
        assert!(paths["/api/{collection}/{pid}"]["put"].is_object());
        assert!(paths["/api/{collection}/{pid}"]["delete"].is_object());
    }

    /// Pins that the audit + event-stream endpoints are documented.
    #[test]
    fn spec_documents_audit_and_event_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/{collection}/audit/recent"]["get"].is_object());
        assert!(paths["/api/{collection}/events/recent"]["get"].is_object());
        assert!(paths["/api/{collection}/{pid}/audit"]["get"].is_object());
    }

    /// Pins that the name-search endpoint is documented with its `q` param.
    #[test]
    fn spec_documents_search_endpoint() {
        let s = spec();
        let op = &s["paths"]["/api/{collection}/search"]["get"];
        assert!(op.is_object());
        assert_eq!(op["parameters"][0]["name"], "q");
    }

    /// Pins the merge endpoints + `MergeRequest` schema.
    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/{collection}/merge"]["post"].is_object());
        assert!(s["paths"]["/api/{collection}/merges/recent"]["get"].is_object());
        assert!(s["components"]["schemas"]["MergeRequest"]["properties"]["main_pid"].is_object());
    }

    /// Pins the Prometheus `/metrics.prom` endpoint under the root path.
    #[test]
    fn spec_documents_metrics_endpoint() {
        let s = spec();
        let op = &s["paths"]["/metrics.prom"]["get"];
        assert!(op.is_object());
        assert!(op["responses"]["200"]["content"]["text/plain"].is_object());
    }

    /// Pins that `/whoami` carries a bearer security requirement.
    #[test]
    fn spec_documents_whoami_with_bearer_security() {
        let s = spec();
        assert!(s["paths"]["/api/{collection}/whoami"]["get"]["security"][0]["bearer"].is_array());
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
