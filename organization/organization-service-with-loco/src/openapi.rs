//! Hand-written OpenAPI 3 description of the organization REST API.
//!
//! The request/response `Organization` body is the
//! `organization_matcher::Organization` shape. That crate is
//! intentionally dependency-light (no `utoipa`), so the schema is
//! authored here by hand rather than derived — which also keeps the doc
//! accurate to the wire format.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Organization Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of organization identities (schema.org/Organization): CRUD + matching. The request/response body for an organization is the organization-matcher Organization shape."
        },
        "paths": paths(),
        "components": components(),
    })
}

/// The `paths` object of the `OpenAPI` document, composed from the
/// CRUD/matching paths and the auxiliary (auth/audit/events/metrics)
/// paths.
fn paths() -> Value {
    let mut paths = crud_paths();
    merge_object(&mut paths, aux_paths());
    paths
}

/// Shallow-merge the top-level keys of `src` into `dst`. Both are JSON
/// objects; this keeps the composed document byte-identical to the
/// single literal it was split from.
fn merge_object(dst: &mut Value, src: Value) {
    if let (Some(dst), Value::Object(src)) = (dst.as_object_mut(), src) {
        for (k, v) in src {
            dst.insert(k, v);
        }
    }
}

/// The CRUD + matching + merge paths.
#[allow(clippy::too_many_lines)] // linear JSON path table
fn crud_paths() -> Value {
    json!({
            "/api/organizations": {
                "get": {
                    "tags": ["organizations"],
                    "summary": "List active organizations (capped at 100)",
                    "responses": { "200": { "description": "List of references",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/OrgRef" } } } } } }
                },
                "post": {
                    "tags": ["organizations"],
                    "summary": "Create an organization",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Organization" } } } },
                    "responses": {
                        "200": { "description": "Created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/OrgRef" } } } },
                        "422": { "description": "Validation failure: name is required" }
                    }
                }
            },
            "/api/organizations/search": {
                "get": {
                    "tags": ["search"],
                    "summary": "Case-insensitive name search",
                    "parameters": [{ "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }],
                    "responses": { "200": { "description": "Matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/OrgRef" } } } } } }
                }
            },
            "/api/organizations/match": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/organizations/check-duplicates": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored organizations",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Organization" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/organizations/deduplicate": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Batch-scan stored organizations pairwise and persist likely duplicates in the review queue",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": {
                        "type": "object", "properties": { "threshold": { "type": "number", "format": "double", "nullable": true } } } } } },
                    "responses": { "200": { "description": "Scan report over the STORED review rows (stable ids)",
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/BatchDeduplicationResponse" } } } } }
                }
            },
            "/api/organizations/review-queue": {
                "get": {
                    "tags": ["matching"],
                    "summary": "List the stored deduplication review queue (newest first)",
                    "parameters": [
                        { "name": "status", "in": "query", "required": false, "schema": { "type": "string", "enum": ["pending", "confirmed", "rejected", "automerged"] } },
                        { "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "minimum": 1, "maximum": 500 } }
                    ],
                    "responses": {
                        "200": { "description": "The stored review items", "content": { "application/json": { "schema": {
                            "type": "object", "properties": {
                                "items": { "type": "array", "items": { "$ref": "#/components/schemas/ReviewQueueItem" } },
                                "total": { "type": "integer" } } } } } },
                        "422": { "description": "Unknown status token" }
                    }
                }
            },
            "/api/organizations/review-queue/{id}/decision": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Decide a pending review item (confirmed / rejected; first writer wins)",
                    "parameters": [ { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } } ],
                    "requestBody": { "required": true, "content": { "application/json": { "schema": {
                        "type": "object", "required": ["status"], "properties": { "status": { "type": "string", "enum": ["confirmed", "rejected"] } } } } } },
                    "responses": {
                        "200": { "description": "The decided item", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ReviewQueueItem" } } } },
                        "404": { "description": "No review item with that id" },
                        "422": { "description": "Item already decided" }
                    }
                }
            },
            "/api/organizations/merge": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Merge a confirmed duplicate into a surviving organization",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MergeRequest" } } } },
                    "responses": {
                        "200": { "description": "The survivor's merged payload + the merged pids" },
                        "404": { "description": "main_pid or duplicate_pid not found" },
                        "422": { "description": "main_pid and duplicate_pid are equal" }
                    }
                }
            },
            "/api/organizations/merges/recent": {
                "get": { "tags": ["matching"], "summary": "Recent merge-history records", "responses": { "200": { "description": "Merge records" } } }
            }
    })
}

/// The auth / audit / events / single-record / metrics paths.
fn aux_paths() -> Value {
    json!({
            "/api/organizations/whoami": {
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
            "/api/organizations/audit/recent": {
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/organizations/events/recent": {
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/organizations/{pid}": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["organizations"], "summary": "Fetch an organization",
                    "responses": { "200": { "description": "Organization", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Organization" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["organizations"], "summary": "Replace an organization's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Organization" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/OrgRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure: name is required" } } },
                "delete": { "tags": ["organizations"], "summary": "Soft-delete an organization", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/organizations/{pid}/audit": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["audit"], "summary": "Audit trail for one organization", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/metrics.prom": {
                "get": {
                    "tags": ["observability"],
                    "summary": "Prometheus metrics (text-exposition format)",
                    "description": "Process-wide metric registry in Prometheus text-exposition format (`text/plain; version=0.0.4`). Mounted at the application root and public even under blanket auth enforcement, so scraping needs no bearer token. Configure your scraper with metrics_path: /metrics.prom.",
                    "responses": { "200": { "description": "Prometheus metrics",
                        "content": { "text/plain": { "schema": { "type": "string" } } } } }
                }
            }
    })
}

/// The `components` object of the `OpenAPI` document.
fn components() -> Value {
    json!({
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT",
                    "description": "RS256 access token from the authentication-service, verified offline against its JWKS." }
            },
            "schemas": {
                "OrgRef": { "type": "object", "required": ["pid", "name"], "properties": {
                    "pid": { "type": "string", "format": "uuid" }, "name": { "type": "string" } } },
                "ScoredRef": { "type": "object", "properties": {
                    "pid": { "type": "string" }, "name": { "type": "string" },
                    "score": { "type": "number", "format": "double" }, "confidence": { "type": "string" },
                    "is_match": { "type": "boolean" } } },
                "ReviewQueueItem": { "type": "object", "properties": {
                    "id": { "type": "string", "format": "uuid" },
                    "organization_id_a": { "type": "string", "format": "uuid" },
                    "organization_id_b": { "type": "string", "format": "uuid" },
                    "match_score": { "type": "number", "format": "double" },
                    "match_quality": { "type": "string" },
                    "detection_method": { "type": "string" },
                    "status": { "type": "string", "enum": ["pending", "confirmed", "rejected", "automerged"] },
                    "reviewed_by": { "type": "string", "nullable": true },
                    "created_at": { "type": "string", "format": "date-time" },
                    "reviewed_at": { "type": "string", "format": "date-time", "nullable": true } } },
                "BatchDeduplicationResponse": { "type": "object", "properties": {
                    "organizations_scanned": { "type": "integer" },
                    "duplicates_found": { "type": "integer" },
                    "auto_merged": { "type": "integer" },
                    "queued_for_review": { "type": "integer" },
                    "review_items": { "type": "array", "items": { "$ref": "#/components/schemas/ReviewQueueItem" } } } },
                "MergeRequest": { "type": "object", "required": ["main_pid", "duplicate_pid"], "properties": {
                    "main_pid": { "type": "string", "format": "uuid" },
                    "duplicate_pid": { "type": "string", "format": "uuid" },
                    "reason": { "type": "string", "nullable": true } } },
                "MatchRequest": { "type": "object", "required": ["query", "candidates"], "properties": {
                    "query": { "$ref": "#/components/schemas/Organization" },
                    "candidates": { "type": "array", "items": { "$ref": "#/components/schemas/Organization" } } } },
                "OrgIdentifier": { "type": "object", "required": ["scheme", "value"], "properties": {
                    "scheme": { "description": "Lei | Duns | Iso6523 | Gln | Wikidata | Ror | Isni | Vat | TaxId | Naics | IsicV4 | Sic | {Custom: string}" },
                    "value": { "type": "string" } } },
                "PostalAddress": { "type": "object", "properties": {
                    "street_address": { "type": "string", "nullable": true },
                    "locality": { "type": "string", "nullable": true },
                    "region": { "type": "string", "nullable": true },
                    "postal_code": { "type": "string", "nullable": true },
                    "country": { "type": "string", "nullable": true } } },
                "Organization": { "type": "object", "required": ["name"], "properties": {
                    "name": { "type": "string" },
                    "legal_name": { "type": "string", "nullable": true },
                    "alternate_names": { "type": "array", "items": { "type": "string" } },
                    "identifiers": { "type": "array", "items": { "$ref": "#/components/schemas/OrgIdentifier" } },
                    "url": { "type": "string", "nullable": true },
                    "same_as": { "type": "array", "items": { "type": "string" } },
                    "address": { "allOf": [{ "$ref": "#/components/schemas/PostalAddress" }], "nullable": true },
                    "jurisdiction": { "type": "string", "nullable": true, "description": "ISO 3166 country" },
                    "founding_date": { "type": "string", "nullable": true },
                    "telephone": { "type": "string", "nullable": true },
                    "email": { "type": "string", "nullable": true },
                    "keywords": { "type": "array", "items": { "type": "string" } } } }
            }
    })
}

/// Pins on the hand-written spec: that it is well-formed and that the
/// load-bearing endpoints/schemas/security are present, so edits to the
/// `json!` literal can't silently drop them.
#[cfg(test)]
mod tests {
    use super::*;

    /// The document advertises `OpenAPI` 3.0.3 and includes the core
    /// create endpoint and the `Organization.name` schema property.
    #[test]
    fn spec_is_wellformed() {
        let s = spec();
        assert_eq!(s["openapi"], "3.0.3");
        assert!(s["paths"]["/api/organizations"]["post"].is_object());
        assert!(s["components"]["schemas"]["Organization"]["properties"]["name"].is_object());
    }

    /// The merge endpoints and the `MergeRequest` schema are documented.
    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/organizations/merge"]["post"].is_object());
        assert!(s["paths"]["/api/organizations/merges/recent"]["get"].is_object());
        assert!(s["components"]["schemas"]["MergeRequest"]["properties"]["main_pid"].is_object());
    }

    /// The Prometheus metrics endpoint is documented at the root path.
    #[test]
    fn spec_documents_metrics_endpoint() {
        let s = spec();
        assert!(s["paths"]["/metrics.prom"]["get"].is_object());
    }

    /// `whoami` carries the bearer security requirement and the matching
    /// `bearer` security scheme is declared.
    #[test]
    fn spec_documents_whoami_with_bearer_security() {
        let s = spec();
        assert!(s["paths"]["/api/organizations/whoami"]["get"]["security"][0]["bearer"].is_array());
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
