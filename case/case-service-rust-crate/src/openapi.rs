//! Hand-written `OpenAPI` 3 description of the case REST API.
//!
//! The request/response `Case` body is the `case_matcher::Case` shape.
//! That crate is intentionally dependency-light (no `utoipa`), so the
//! schema is authored here by hand rather than derived — which also keeps
//! the doc accurate to the wire format.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
// One contiguous `json!` literal: splitting it into helpers would
// scatter the document and hurt readability, so the length is allowed.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Case Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of governmental case identities: CRUD + matching. The request/response body for a case is the case-matcher Case shape. Validation failures (blank title, malformed opened_date, blank identifier value, blank subjects/keywords) return 422."
        },
        "paths": {
            "/api/cases": {
                "get": {
                    "tags": ["cases"],
                    "summary": "List active cases (capped at 100)",
                    "responses": { "200": { "description": "List of references",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/CaseRef" } } } } } }
                },
                "post": {
                    "tags": ["cases"],
                    "summary": "Create a case",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Case" } } } },
                    "responses": {
                        "200": { "description": "Created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CaseRef" } } } },
                        "422": { "description": "Validation failure: blank title, malformed opened_date, or blank identifier/subject/keyword" }
                    }
                }
            },
            "/api/cases/search": {
                "get": {
                    "tags": ["cases"],
                    "summary": "Case-insensitive title search (Postgres ILIKE, cap 50)",
                    "parameters": [{ "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }],
                    "responses": {
                        "200": { "description": "Matches", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/CaseRef" } } } } },
                        "400": { "description": "Missing or blank `q`" }
                    }
                }
            },
            "/api/cases/match": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list (no persistence)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/cases/check-duplicates": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored cases",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Case" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/cases/merge": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Merge a confirmed duplicate into a surviving case",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MergeRequest" } } } },
                    "responses": {
                        "200": { "description": "The survivor's merged payload + the merged pids" },
                        "404": { "description": "main_pid or duplicate_pid not found" },
                        "422": { "description": "main_pid and duplicate_pid are equal" }
                    }
                }
            },
            "/api/cases/merges/recent": {
                "get": { "tags": ["matching"], "summary": "Recent merge-history records", "responses": { "200": { "description": "Merge records" } } }
            },
            "/api/cases/whoami": {
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
            "/api/cases/audit/recent": {
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries across all cases", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/cases/events/recent": {
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/cases/{pid}": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["cases"], "summary": "Fetch the stored case",
                    "responses": { "200": { "description": "Case", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Case" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["cases"], "summary": "Replace a case's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Case" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CaseRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure" } } },
                "delete": { "tags": ["cases"], "summary": "Soft-delete a case", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/cases/{pid}/audit": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["audit"], "summary": "Audit trail for one case", "responses": { "200": { "description": "Audit entries" } } }
            }
        },
        "components": {
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT",
                    "description": "RS256 access token from the authentication-service, verified offline against its JWKS." }
            },
            "schemas": {
                "CaseRef": { "type": "object", "required": ["pid", "title"], "properties": {
                    "pid": { "type": "string", "format": "uuid" }, "title": { "type": "string" } } },
                "ScoredRef": { "type": "object", "properties": {
                    "pid": { "type": "string" }, "title": { "type": "string" },
                    "score": { "type": "number", "format": "double" }, "confidence": { "type": "string" },
                    "is_match": { "type": "boolean" } } },
                "MatchRequest": { "type": "object", "required": ["query", "candidates"], "properties": {
                    "query": { "$ref": "#/components/schemas/Case" },
                    "candidates": { "type": "array", "items": { "$ref": "#/components/schemas/Case" } } } },
                "MergeRequest": { "type": "object", "required": ["main_pid", "duplicate_pid"], "properties": {
                    "main_pid": { "type": "string", "format": "uuid" },
                    "duplicate_pid": { "type": "string", "format": "uuid" },
                    "reason": { "type": "string", "nullable": true } } },
                "CaseIdentifier": { "type": "object", "required": ["scheme", "value"], "properties": {
                    "scheme": { "description": "Docket | ExternalCaseId | Uri | Uuid | AgencyCaseNumber | LocalId | {Custom: string}" },
                    "value": { "type": "string", "description": "Must be non-blank." } } },
                "Case": { "type": "object", "required": ["title"], "properties": {
                    "title": { "type": "string" },
                    "alternate_titles": { "type": "array", "items": { "type": "string" } },
                    "case_number": { "type": "string", "nullable": true, "description": "Agency-scoped local case number" },
                    "agency_id": { "type": "string", "nullable": true },
                    "agency_name": { "type": "string", "nullable": true },
                    "case_type": { "type": "string", "nullable": true, "description": "Benefit | Legal | SocialServices | Healthcare | Housing | Immigration | Licensing | Complaint | Appeal | Investigation | Tax | Employment | {Custom: string}" },
                    "status": { "type": "string", "nullable": true, "description": "Open | InProgress | Pending | OnHold | Closed | Resolved | Rejected | Withdrawn | {Custom: string}" },
                    "priority": { "type": "string", "nullable": true, "description": "Low | Normal | High | Urgent" },
                    "opened_date": { "type": "string", "nullable": true, "description": "ISO-8601 YYYY or YYYY-MM-DD" },
                    "subjects": { "type": "array", "items": { "type": "string" }, "description": "Opaque involved-party identifiers; each must be non-blank" },
                    "keywords": { "type": "array", "items": { "type": "string" }, "description": "Each must be non-blank" },
                    "identifiers": { "type": "array", "items": { "$ref": "#/components/schemas/CaseIdentifier" } },
                    "same_as": { "type": "array", "items": { "type": "string" } },
                    "in_language": { "type": "array", "items": { "type": "string" }, "description": "BCP-47 language codes" } } }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the document's basic shape: `OpenAPI` version 3.0.3 and the
    /// presence of the core create path and `Case`/`CaseIdentifier` schemas.
    #[test]
    fn spec_is_wellformed() {
        let s = spec();
        assert_eq!(s["openapi"], "3.0.3");
        assert!(s["paths"]["/api/cases"]["post"].is_object());
        assert!(s["paths"]["/api/cases/check-duplicates"]["post"].is_object());
        assert!(s["components"]["schemas"]["Case"]["properties"]["title"].is_object());
        assert!(s["components"]["schemas"]["CaseIdentifier"]["properties"]["value"].is_object());
    }

    /// Pins that the seven core CRUD + matching operations are all
    /// documented (keeps the hand-written spec from drifting from routes).
    #[test]
    fn spec_documents_core_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        // The seven core CRUD + matching operations.
        assert!(paths["/api/cases"]["get"].is_object());
        assert!(paths["/api/cases"]["post"].is_object());
        assert!(paths["/api/cases/match"]["post"].is_object());
        assert!(paths["/api/cases/check-duplicates"]["post"].is_object());
        assert!(paths["/api/cases/{pid}"]["get"].is_object());
        assert!(paths["/api/cases/{pid}"]["put"].is_object());
        assert!(paths["/api/cases/{pid}"]["delete"].is_object());
    }

    /// Pins that the audit + event-stream endpoints are documented.
    #[test]
    fn spec_documents_audit_and_event_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/cases/audit/recent"]["get"].is_object());
        assert!(paths["/api/cases/events/recent"]["get"].is_object());
        assert!(paths["/api/cases/{pid}/audit"]["get"].is_object());
    }

    /// Pins that the title-search endpoint is documented with its `q`
    /// query parameter.
    #[test]
    fn spec_documents_search_endpoint() {
        let s = spec();
        let op = &s["paths"]["/api/cases/search"]["get"];
        assert!(op.is_object());
        assert_eq!(op["parameters"][0]["name"], "q");
    }

    /// Pins that both merge endpoints and the `MergeRequest` schema are
    /// documented.
    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/cases/merge"]["post"].is_object());
        assert!(s["paths"]["/api/cases/merges/recent"]["get"].is_object());
        assert!(s["components"]["schemas"]["MergeRequest"]["properties"]["main_pid"].is_object());
    }

    /// Pins that `/whoami` carries a bearer security requirement and that
    /// the `bearer` security scheme is declared.
    #[test]
    fn spec_documents_whoami_with_bearer_security() {
        let s = spec();
        assert!(s["paths"]["/api/cases/whoami"]["get"]["security"][0]["bearer"].is_array());
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
