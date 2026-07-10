//! Hand-written `OpenAPI` 3 description of the link-graph read API
//! (spec §9). Dependency-light (no `utoipa`); authored by hand so the doc
//! stays accurate to the enveloped wire format. The API is **read-only**;
//! every graph response carries an `as_of` freshness watermark.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Link Graph Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Read-model aggregator for cross-service entity linking. Read-only: edges are populated from the entity event streams (linked/unlinked), never written via this API. A ref is an EntityRef URN (e.g. person:0c4f…), URL-encoded in a path. Every graph response carries an `as_of` watermark. When LINK_GRAPH_REQUIRE_AUTH is on, requests need a Bearer token; high-sensitivity case↔person (subject_of) edges are concealed from callers without case-read authorisation."
        },
        "paths": paths(),
        "components": components(),
    })
}

/// The `paths` object: the four read endpoints (spec §9.1).
fn paths() -> Value {
    let ref_param = json!({
        "name": "ref", "in": "path", "required": true,
        "schema": { "type": "string" },
        "description": "EntityRef URN (e.g. person:0c4f1e2a-…), URL-encoded."
    });
    let kind_param = json!({
        "name": "kind", "in": "query", "required": false,
        "schema": { "type": "string", "enum": EDGE_KINDS },
        "description": "Filter by edge kind."
    });
    json!({
        "/api/neighbors/{ref}": { "get": {
            "summary": "Edges incident to a ref",
            "description": "Edges incident to {ref}, in the requested direction, up to `depth` hops (capped at 2).",
            "parameters": [
                ref_param,
                kind_param,
                { "name": "direction", "in": "query", "required": false,
                  "schema": { "type": "string", "enum": ["out", "in", "both"], "default": "both" } },
                { "name": "depth", "in": "query", "required": false,
                  "schema": { "type": "integer", "minimum": 1, "maximum": 2, "default": 1 } }
            ],
            "responses": {
                "200": { "description": "Incident edges", "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/NeighborsEnvelope" } } } },
                "400": { "description": "Malformed ref, unknown kind, or depth over the cap" },
                "401": { "description": "Missing/invalid token (enforcement on)" }
            }
        } },
        "/api/edges": { "get": {
            "summary": "Filtered edge list",
            "parameters": [
                { "name": "from", "in": "query", "required": false, "schema": { "type": "string" },
                  "description": "Filter by from_ref URN." },
                { "name": "to", "in": "query", "required": false, "schema": { "type": "string" },
                  "description": "Filter by to_ref URN." },
                kind_param,
                { "name": "status", "in": "query", "required": false,
                  "schema": { "type": "string", "enum": ["unverified", "verified", "dangling"] } }
            ],
            "responses": {
                "200": { "description": "Filtered edges", "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/EdgesEnvelope" } } } },
                "400": { "description": "Malformed filter value" },
                "401": { "description": "Missing/invalid token (enforcement on)" }
            }
        } },
        "/api/single-view/{ref}": { "get": {
            "summary": "Golden-record walk",
            "description": "same_identity unification (person ↔ worker) plus the affiliations incident to the unified identity.",
            "parameters": [ ref_param ],
            "responses": {
                "200": { "description": "Unified identity + affiliations", "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/SingleViewEnvelope" } } } },
                "400": { "description": "Malformed ref" },
                "401": { "description": "Missing/invalid token (enforcement on)" }
            }
        } },
        "/api/health/freshness": { "get": {
            "summary": "Per-topic consumer freshness",
            "description": "The last consumed occurred_at + lag-versus-now per entity topic (the eventual-consistency window, made queryable).",
            "responses": {
                "200": { "description": "Per-topic freshness", "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/FreshnessEnvelope" } } } }
            }
        } }
    })
}

/// The closed v1 edge-kind vocabulary, for the `kind` enum.
const EDGE_KINDS: [&str; 5] = [
    "same_identity",
    "works_at",
    "member_of",
    "employed_by",
    "subject_of",
];

/// The `components/schemas` object.
fn components() -> Value {
    json!({ "schemas": {
        "Edge": { "type": "object",
            "description": "One stored graph edge (a derived read-model row).",
            "properties": {
                "edge_id": { "type": "string", "format": "uuid" },
                "from_ref": { "type": "string", "description": "Canonical from-endpoint URN." },
                "to_ref": { "type": "string" },
                "kind": { "type": "string", "enum": EDGE_KINDS },
                "directed": { "type": "boolean", "description": "false for symmetric kinds (same_identity)." },
                "role": { "type": "string", "nullable": true },
                "confidence": { "type": "number", "format": "double", "nullable": true },
                "provenance": { "type": "string", "enum": ["operator", "import", "matcher_suggested"] },
                "valid_from": { "type": "string", "format": "date", "nullable": true },
                "valid_to": { "type": "string", "format": "date", "nullable": true },
                "status": { "type": "string", "enum": ["unverified", "verified", "dangling"],
                    "description": "Integrity lifecycle from endpoint presence." },
                "observed_at": { "type": "string", "format": "date-time" },
                "source_event_id": { "type": "string", "format": "uuid" }
            }
        },
        "Affiliation": { "type": "object",
            "properties": {
                "from": { "type": "string" }, "to": { "type": "string" },
                "kind": { "type": "string", "enum": EDGE_KINDS }
            }
        },
        "TopicFreshness": { "type": "object",
            "properties": {
                "entity": { "type": "string" },
                "last_occurred_at": { "type": "string", "format": "date-time" },
                "lag_seconds": { "type": "integer", "format": "int64" }
            }
        },
        "NeighborsEnvelope": envelope(&json!({
            "type": "object", "properties": {
                "ref": { "type": "string" },
                "edges": { "type": "array", "items": { "$ref": "#/components/schemas/Edge" } },
                "as_of": { "type": "string", "format": "date-time", "nullable": true }
            }
        })),
        "EdgesEnvelope": envelope(&json!({
            "type": "object", "properties": {
                "edges": { "type": "array", "items": { "$ref": "#/components/schemas/Edge" } },
                "as_of": { "type": "string", "format": "date-time", "nullable": true }
            }
        })),
        "SingleViewEnvelope": envelope(&json!({
            "type": "object", "properties": {
                "identity_refs": { "type": "array", "items": { "type": "string" } },
                "affiliations": { "type": "array", "items": { "$ref": "#/components/schemas/Affiliation" } },
                "as_of": { "type": "string", "format": "date-time", "nullable": true }
            }
        })),
        "FreshnessEnvelope": envelope(&json!({
            "type": "object", "properties": {
                "topics": { "type": "array", "items": { "$ref": "#/components/schemas/TopicFreshness" } },
                "as_of": { "type": "string", "format": "date-time", "nullable": true }
            }
        }))
    } })
}

/// Wrap a `data` schema in the family's `{ success, data, error }`
/// response envelope.
fn envelope(data: &Value) -> Value {
    json!({
        "type": "object",
        "required": ["success"],
        "properties": {
            "success": { "type": "boolean" },
            "data": data,
            "error": { "type": "string", "nullable": true }
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
        assert!(s["info"]["title"].is_string());
        assert!(s["components"]["schemas"]["Edge"].is_object());
    }

    #[test]
    fn spec_documents_the_four_read_endpoints() {
        let s = spec();
        let p = &s["paths"];
        assert!(p["/api/neighbors/{ref}"]["get"].is_object());
        assert!(p["/api/edges"]["get"].is_object());
        assert!(p["/api/single-view/{ref}"]["get"].is_object());
        assert!(p["/api/health/freshness"]["get"].is_object());
    }

    #[test]
    fn every_edge_returning_response_is_enveloped_with_as_of() {
        let s = spec();
        for env in [
            "NeighborsEnvelope",
            "EdgesEnvelope",
            "SingleViewEnvelope",
            "FreshnessEnvelope",
        ] {
            let schema = &s["components"]["schemas"][env];
            assert_eq!(schema["properties"]["success"]["type"], "boolean");
            assert!(
                schema["properties"]["data"]["properties"]["as_of"].is_object(),
                "{env} data carries as_of"
            );
        }
    }
}
