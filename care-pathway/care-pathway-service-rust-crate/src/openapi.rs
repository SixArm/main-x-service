//! Hand-written `OpenAPI` 3 description of the care-pathway REST API.
//!
//! The request/response `CarePathway` body is the
//! `care_pathway_matcher::CarePathway` shape. That crate is intentionally
//! dependency-light (no `utoipa`), so the schema is authored here by hand
//! rather than derived — which also keeps the doc accurate to the wire
//! format.

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
            "title": "Care Pathway Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of clinical care-pathway identities: CRUD + matching. The request/response body for a pathway is the care-pathway-matcher CarePathway shape. Validation failures (blank name, malformed condition_codes) return 422."
        },
        "paths": {
            "/api/care-pathways": {
                "get": {
                    "tags": ["care-pathways"],
                    "summary": "List active care pathways (capped at 100)",
                    "responses": { "200": { "description": "List of references",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/PathwayRef" } } } } } }
                },
                "post": {
                    "tags": ["care-pathways"],
                    "summary": "Create a care pathway",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } },
                    "responses": {
                        "200": { "description": "Created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PathwayRef" } } } },
                        "422": { "description": "Validation failure: blank name or malformed condition_codes" }
                    }
                }
            },
            "/api/care-pathways/match": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list (no persistence)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/care-pathways/check-duplicates": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored pathways",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/care-pathways/whoami": {
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
            "/api/care-pathways/audit/recent": {
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries across all pathways", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/care-pathways/events/recent": {
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/care-pathways/{pid}": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["care-pathways"], "summary": "Fetch the stored care pathway",
                    "responses": { "200": { "description": "CarePathway", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["care-pathways"], "summary": "Replace a care pathway's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PathwayRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure: blank name or malformed condition_codes" } } },
                "delete": { "tags": ["care-pathways"], "summary": "Soft-delete a care pathway", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/care-pathways/{pid}/audit": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["audit"], "summary": "Audit trail for one care pathway", "responses": { "200": { "description": "Audit entries" } } }
            }
        },
        "components": {
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT",
                    "description": "RS256 access token from the authentication-service, verified offline against its JWKS." }
            },
            "schemas": {
                "PathwayRef": { "type": "object", "required": ["pid", "name"], "properties": {
                    "pid": { "type": "string", "format": "uuid" }, "name": { "type": "string" } } },
                "ScoredRef": { "type": "object", "properties": {
                    "pid": { "type": "string" }, "name": { "type": "string" },
                    "score": { "type": "number", "format": "double" }, "confidence": { "type": "string" },
                    "is_match": { "type": "boolean" } } },
                "MatchRequest": { "type": "object", "required": ["query", "candidates"], "properties": {
                    "query": { "$ref": "#/components/schemas/CarePathway" },
                    "candidates": { "type": "array", "items": { "$ref": "#/components/schemas/CarePathway" } } } },
                "ConditionCode": { "type": "object", "required": ["system", "code"], "properties": {
                    "system": { "description": "Icd10 | Icd11 | Snomed | {Custom: string}" },
                    "code": { "type": "string", "description": "Format-validated by system: ICD-10 (e.g. I63.9), ICD-11 stem (e.g. 1A00), SNOMED CT SCTID (6-18 digits, Verhoeff check digit); Custom must be non-blank." } } },
                "PathwayIdentifier": { "type": "object", "required": ["scheme", "value"], "properties": {
                    "scheme": { "description": "Doi | Wikidata | GuidelineId | Uri | Uuid | {Custom: string}" },
                    "value": { "type": "string" } } },
                "CarePathway": { "type": "object", "required": ["name"], "properties": {
                    "name": { "type": "string" },
                    "alternate_names": { "type": "array", "items": { "type": "string" } },
                    "pathway_code": { "type": "string", "nullable": true, "description": "Provider-scoped local code" },
                    "provider_id": { "type": "string", "nullable": true },
                    "provider_name": { "type": "string", "nullable": true },
                    "care_setting": { "type": "string", "nullable": true, "description": "Inpatient | Outpatient | Emergency | Community | …" },
                    "condition_codes": { "type": "array", "items": { "$ref": "#/components/schemas/ConditionCode" } },
                    "interventions": { "type": "array", "items": { "type": "string" } },
                    "keywords": { "type": "array", "items": { "type": "string" } },
                    "identifiers": { "type": "array", "items": { "$ref": "#/components/schemas/PathwayIdentifier" } },
                    "same_as": { "type": "array", "items": { "type": "string" } },
                    "in_language": { "type": "array", "items": { "type": "string" }, "description": "BCP-47 language codes" } } }
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
        assert!(s["paths"]["/api/care-pathways"]["post"].is_object());
        assert!(s["paths"]["/api/care-pathways/check-duplicates"]["post"].is_object());
        assert!(s["components"]["schemas"]["CarePathway"]["properties"]["name"].is_object());
        assert!(s["components"]["schemas"]["ConditionCode"]["properties"]["code"].is_object());
    }

    #[test]
    fn spec_documents_core_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        // The seven core CRUD + matching operations.
        assert!(paths["/api/care-pathways"]["get"].is_object());
        assert!(paths["/api/care-pathways"]["post"].is_object());
        assert!(paths["/api/care-pathways/match"]["post"].is_object());
        assert!(paths["/api/care-pathways/check-duplicates"]["post"].is_object());
        assert!(paths["/api/care-pathways/{pid}"]["get"].is_object());
        assert!(paths["/api/care-pathways/{pid}"]["put"].is_object());
        assert!(paths["/api/care-pathways/{pid}"]["delete"].is_object());
    }

    #[test]
    fn spec_documents_audit_and_event_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/care-pathways/audit/recent"]["get"].is_object());
        assert!(paths["/api/care-pathways/events/recent"]["get"].is_object());
        assert!(paths["/api/care-pathways/{pid}/audit"]["get"].is_object());
    }

    #[test]
    fn spec_documents_whoami_with_bearer_security() {
        let s = spec();
        assert!(s["paths"]["/api/care-pathways/whoami"]["get"]["security"][0]["bearer"].is_array());
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
