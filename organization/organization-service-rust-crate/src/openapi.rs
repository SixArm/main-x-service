//! Hand-written OpenAPI 3 description of the organization REST API.
//!
//! The request/response `Organization` body is the
//! `organization_matcher::Organization` shape. That crate is
//! intentionally dependency-light (no `utoipa`), so the schema is
//! authored here by hand rather than derived — which also keeps the doc
//! accurate to the wire format.

use serde_json::{json, Value};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
// One contiguous `json!` literal: splitting it into helpers would
// scatter the document and hurt readability, so the length is allowed.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Organization Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of organization identities (schema.org/Organization): CRUD + matching. The request/response body for an organization is the organization-matcher Organization shape."
        },
        "paths": {
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
            },
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
            }
        },
        "components": {
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
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_wellformed() {
        let s = spec();
        assert_eq!(s["openapi"], "3.0.3");
        assert!(s["paths"]["/api/organizations"]["post"].is_object());
        assert!(s["components"]["schemas"]["Organization"]["properties"]["name"].is_object());
    }

    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/organizations/merge"]["post"].is_object());
        assert!(s["paths"]["/api/organizations/merges/recent"]["get"].is_object());
        assert!(s["components"]["schemas"]["MergeRequest"]["properties"]["main_pid"].is_object());
    }

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
