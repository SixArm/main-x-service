//! Hand-written `OpenAPI` 3 description of the portfolio REST API.
//!
//! The request/response `WorkItem` body is the `portfolio_matcher::WorkItem`
//! shape. That crate is intentionally dependency-light (no `utoipa`), so
//! the schema is authored here by hand rather than derived — which also
//! keeps the doc accurate to the wire format. The four collections
//! (portfolios / projects / products / programs) share one templated path
//! set under `/api/v1/{collection}`.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Portfolio Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of work-item identities across four collections (portfolios / projects / products / programs): CRUD + within-collection matching. The request/response body is the portfolio-matcher WorkItem shape. Validation failures (blank name, blank goal title, malformed identifier, malformed portfolio_ref / in_language) return 422. Matching never crosses collections (the matcher's kind gate)."
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
    paths
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
            "/api/v1/{collection}": {
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
            "/api/v1/{collection}/search": {
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
            "/api/v1/{collection}/match": {
                "parameters": [collection_param()],
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list (no persistence; cross-kind candidates score 0.0)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/v1/{collection}/check-duplicates": {
                "parameters": [collection_param()],
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored work items in the collection",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/v1/{collection}/merge": {
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
            "/api/v1/{collection}/merges/recent": {
                "parameters": [collection_param()],
                "get": { "tags": ["matching"], "summary": "Recent merge-history records", "responses": { "200": { "description": "Merge records" } } }
            }
    })
}

/// The auth / audit / events / single-record / metrics paths.
fn aux_paths() -> Value {
    json!({
            "/api/v1/{collection}/whoami": {
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
            "/api/v1/{collection}/audit/recent": {
                "parameters": [collection_param()],
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/v1/{collection}/events/recent": {
                "parameters": [collection_param()],
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/v1/{collection}/{pid}": {
                "parameters": [collection_param(), { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["work-items"], "summary": "Fetch the stored work item",
                    "responses": { "200": { "description": "WorkItem", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["work-items"], "summary": "Replace a work item's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItem" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/WorkItemRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure" } } },
                "delete": { "tags": ["work-items"], "summary": "Soft-delete a work item", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/v1/{collection}/{pid}/audit": {
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
        assert!(s["paths"]["/api/v1/{collection}"]["post"].is_object());
        assert!(s["paths"]["/api/v1/{collection}/check-duplicates"]["post"].is_object());
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
        assert!(paths["/api/v1/{collection}"]["get"].is_object());
        assert!(paths["/api/v1/{collection}"]["post"].is_object());
        assert!(paths["/api/v1/{collection}/match"]["post"].is_object());
        assert!(paths["/api/v1/{collection}/check-duplicates"]["post"].is_object());
        assert!(paths["/api/v1/{collection}/{pid}"]["get"].is_object());
        assert!(paths["/api/v1/{collection}/{pid}"]["put"].is_object());
        assert!(paths["/api/v1/{collection}/{pid}"]["delete"].is_object());
    }

    /// Pins that the audit + event-stream endpoints are documented.
    #[test]
    fn spec_documents_audit_and_event_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/v1/{collection}/audit/recent"]["get"].is_object());
        assert!(paths["/api/v1/{collection}/events/recent"]["get"].is_object());
        assert!(paths["/api/v1/{collection}/{pid}/audit"]["get"].is_object());
    }

    /// Pins that the name-search endpoint is documented with its `q` param.
    #[test]
    fn spec_documents_search_endpoint() {
        let s = spec();
        let op = &s["paths"]["/api/v1/{collection}/search"]["get"];
        assert!(op.is_object());
        assert_eq!(op["parameters"][0]["name"], "q");
    }

    /// Pins the merge endpoints + `MergeRequest` schema.
    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/v1/{collection}/merge"]["post"].is_object());
        assert!(s["paths"]["/api/v1/{collection}/merges/recent"]["get"].is_object());
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
        assert!(
            s["paths"]["/api/v1/{collection}/whoami"]["get"]["security"][0]["bearer"].is_array()
        );
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
