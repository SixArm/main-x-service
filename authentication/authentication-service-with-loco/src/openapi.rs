//! Hand-written `OpenAPI` 3 description of the authentication REST API.
//!
//! The family authors `OpenAPI` documents by hand (no `utoipa`), which
//! keeps the schema accurate to the wire format and the crate
//! dependency-light. This document covers the passwordless magic-link
//! surface (FR-1…FR-8): signup, magic-link issuance, redemption, `me`,
//! signout, and the public key set. The bearer `securityScheme` describes
//! the PASETO v4.public access token that `me` and `signout` require.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
///
/// Assembled from cohesive helpers (`paths`, `components`) so each part
/// stays readable; the produced JSON is identical to one contiguous
/// literal.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Authentication Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Central single sign-on provider for the Main X Index family. Passwordless email magic-link authentication; issues PASETO v4.public access tokens verifiable offline against the Ed25519 key set at /.well-known/paseto-keys. The unauthenticated issuance endpoints (signup, magic-link) always return 200 to avoid account enumeration, and are rate-limited per email (429 when exceeded). me and signout require a bearer token."
        },
        "paths": paths(),
        "components": components()
    })
}

/// The `paths` object of the `OpenAPI` document, assembled from the
/// per-area path groups (merged into a single object).
fn paths() -> Value {
    let mut paths = serde_json::Map::new();
    for group in [
        auth_paths(),
        account_paths(),
        admin_paths(),
        compliance_paths(),
        infra_paths(),
    ] {
        if let Value::Object(map) = group {
            paths.extend(map);
        }
    }
    Value::Object(paths)
}

/// Magic-link sign-up / sign-in and the system-wide audit trail.
fn auth_paths() -> Value {
    json!({
            "/api/auth/signup": {
                "post": {
                    "tags": ["auth"],
                    "summary": "Create a passwordless account and issue a magic link",
                    "description": "Always returns 200 regardless of whether the email already exists (anti-enumeration). Rate-limited per email; over the limit returns 429 without issuing a token or sending mail.",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SignupParams" } } } },
                    "responses": {
                        "200": { "description": "Magic link issued (or silently ignored to avoid enumeration)" },
                        "429": { "description": "Too many issuance requests for this email; try again later", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } }
                    }
                }
            },
            "/api/auth/magic-link": {
                "post": {
                    "tags": ["auth"],
                    "summary": "Request a magic link for an existing account (sign in)",
                    "description": "Always returns 200, even for unknown emails (anti-enumeration). Rate-limited per email; over the limit returns 429 without issuing a token or sending mail.",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MagicLinkParams" } } } },
                    "responses": {
                        "200": { "description": "Magic link issued (or silently ignored for unknown emails)" },
                        "429": { "description": "Too many issuance requests for this email; try again later", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } }
                    }
                }
            },
            "/api/auth/magic-link/{token}": {
                "get": {
                    "tags": ["auth"],
                    "summary": "Consume a magic link → PASETO access token + session",
                    "description": "Validates the unexpired, single-use token, marks the email verified, issues a PASETO v4.public access token, and records a revocable session.",
                    "parameters": [{ "name": "token", "in": "path", "required": true, "schema": { "type": "string" }, "description": "The opaque magic-link token." }],
                    "responses": {
                        "200": { "description": "Access token + user", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/LoginResponse" } } } },
                        "401": { "description": "Invalid, expired, or already-consumed magic link" }
                    }
                }
            },
            "/api/auth/me": {
                "get": {
                    "tags": ["auth"],
                    "summary": "Current authenticated user",
                    "description": "Returns the current user. Honors local revocation: a signed-out session is rejected even though the token signature is still valid.",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Current user", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CurrentResponse" } } } },
                        "401": { "description": "Missing/invalid bearer token, or the session was signed out" }
                    }
                }
            },
            "/api/auth/signout": {
                "post": {
                    "tags": ["auth"],
                    "summary": "Revoke the current session",
                    "description": "Marks the bearer token's session revoked. Peer services that cached the token keep honoring it until expiry (offline PASETO verification); TTLs are kept short to bound this window.",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Session revoked" },
                        "401": { "description": "Missing or invalid bearer token" }
                    }
                }
            },
            "/api/auth/audit/recent": {
                "get": {
                    "tags": ["audit"],
                    "summary": "Recent authentication events (system-wide audit trail)",
                    "description": "Newest-first authentication audit events (signup, magic-link request/redeem, signout, account_erased), capped at 100. Rows carry the event name, normalised email, subject pid, and an outcome marker — never tokens or secrets. Deliberately unauthenticated (mirrors the family /audit/recent pattern); the GDPR right-of-access requirement is met by the bearer-gated per-subject /api/auth/account/audit instead.",
                    "responses": {
                        "200": { "description": "Recent auth events", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/AuthEvent" } } } } }
                    }
                }
            }
    })
}

/// GDPR account endpoints: export, per-subject audit, and erasure.
fn account_paths() -> Value {
    json!({
            "/api/auth/account/export": {
                "get": {
                    "tags": ["account"],
                    "summary": "GDPR right of access — export the subject's account data",
                    "description": "Returns everything the service holds about the authenticated subject: their users row, their sessions (issuance/expiry/revocation timestamps + user agent), and their auth_events audit trail. Never includes tokens, key material, the password hash, or the api key. A GDPR-erased account is treated as gone (401).",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Account export", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/AccountExport" } } } },
                        "401": { "description": "Missing/invalid bearer token, or the account has been erased" }
                    }
                }
            },
            "/api/auth/account/audit": {
                "get": {
                    "tags": ["account"],
                    "summary": "GDPR right of access — the subject's own audit trail",
                    "description": "Returns only the authenticated subject's own auth_events rows (matched by pid or email), newest first — the bearer-gated, per-subject counterpart to the open system-wide /api/auth/audit/recent.",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "The subject's audit events", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/AccountAuditExport" } } } } },
                        "401": { "description": "Missing/invalid bearer token, or the account has been erased" }
                    }
                }
            },
            "/api/auth/account": {
                "delete": {
                    "tags": ["account"],
                    "summary": "GDPR right to erasure — delete the subject's account",
                    "description": "Soft-deletes + anonymises the account: stamps users.deleted_at, replaces email/name with a tombstone (so referential history and the audit trail keep their integrity), revokes all of the subject's sessions, and records an account_erased audit row. After erasure the bearer token still verifies cryptographically until exp, but /me and the export treat the subject as gone (401). Idempotent.",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Account erased (soft-delete + anonymise)" },
                        "401": { "description": "Missing/invalid bearer token, or the account is already erased" }
                    }
                }
            }
    })
}

/// Admin endpoints: ABAC attribute assignment over HTTP. Gated by an
/// `access=admin` bearer (403 otherwise); see
/// `agents/share/authorization-attributes.md` §6.
fn admin_paths() -> Value {
    json!({
            "/api/auth/admin/users/{pid}/attributes": {
                "get": {
                    "tags": ["admin"],
                    "summary": "Show a user's ABAC subject attributes",
                    "description": "Returns the user's current ABAC attribute map (the string→strings map minted into the PASETO attrs claim). Requires a bearer whose own attributes include access=admin.",
                    "security": [{ "bearer": [] }],
                    "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                    "responses": {
                        "200": { "description": "The user's attribute map", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/UserAttributes" } } } },
                        "401": { "description": "Missing/invalid bearer token" },
                        "403": { "description": "Valid token but the caller is not an admin (access=admin required)" },
                        "404": { "description": "No such (live) user" }
                    }
                },
                "put": {
                    "tags": ["admin"],
                    "summary": "Replace a user's ABAC subject attributes",
                    "description": "Replaces the user's entire ABAC attribute map. Keys and values must be short lowercase tokens; the reserved pseudo-attributes sub/email/entity are refused, and no key may map to an empty value list (send {} to clear). Requires access=admin. Writes an attributes_assigned auth_events audit row (actor = the admin's pid).",
                    "security": [{ "bearer": [] }],
                    "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ReplaceUserAttributes" } } } },
                    "responses": {
                        "200": { "description": "The updated attribute map", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/UserAttributes" } } } },
                        "401": { "description": "Missing/invalid bearer token" },
                        "403": { "description": "Valid token but the caller is not an admin (access=admin required)" },
                        "404": { "description": "No such (live) user" },
                        "422": { "description": "Invalid attribute key or value" }
                    }
                }
            }
    })
}

/// Keyed integrity verification over the `auth_events` audit trail.
/// Requires a bearer (any authenticated caller — not admin-gated, since
/// the report carries no PII); see `src/controllers/compliance.rs` for
/// the reasoning.
fn compliance_paths() -> Value {
    json!({
            "/api/compliance/audit/verify": {
                "get": {
                    "tags": ["compliance"],
                    "summary": "Recompute and verify auth_events integrity digests",
                    "description": "Recomputes each examined auth_events row's SHA-256, SHA-3, and (where a key is configured) HMAC digest, and reports any row whose recomputed value no longer matches what was stored. Examines up to `limit` rows (default 1000, capped at 10000), newest first. A verified:true result attests only that no examined row's content was altered — it does NOT attest that no row was deleted (see the caveat field in the response). Requires a valid bearer token (any authenticated caller); gated for cost, not disclosure — the handler recomputes real digests over real DB rows on every call, an unauthenticated CPU/DB denial-of-service surface even though the report itself carries no PII (row counts and row ids only).",
                    "security": [{ "bearer": [] }],
                    "parameters": [{ "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "minimum": 1, "maximum": 10000, "default": 1000 }, "description": "Rows to examine, newest first; clamped to [1, 10000]." }],
                    "responses": {
                        "200": { "description": "Integrity report", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/AuditIntegrityReport" } } } },
                        "401": { "description": "Missing or invalid bearer token" }
                    }
                }
            }
    })
}

/// Infrastructure endpoints: the public key set and Prometheus metrics.
fn infra_paths() -> Value {
    json!({
            "/.well-known/paseto-keys": {
                "get": {
                    "tags": ["paseto-keys"],
                    "summary": "Public keys for offline token verification",
                    "description": "The Ed25519 public key set (OKP/Ed25519 JWK form) peer services fetch once to verify PASETO v4.public tokens locally — no shared secret, no introspection round-trip. May publish MULTIPLE keys during a key rotation: the primary (active signing) key is first, followed by any additional verify-only keys whose recently-issued tokens are still within their lifetime. Peers select the verifying key by the token footer kid, so every published kid is trusted until it is retired.",
                    "responses": {
                        "200": { "description": "PASETO public key set", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PasetoKeys" } } } }
                    }
                }
            },
            "/metrics.prom": {
                "get": {
                    "tags": ["metrics"],
                    "summary": "Prometheus metrics (text exposition)",
                    "description": "Process-wide auth-specific counters in Prometheus text-exposition format (Content-Type text/plain; version=0.0.4). Mounted at the root for the conventional scrape path (metrics_path: /metrics.prom). Aggregate counts only — signups, magic links issued/redeemed, signouts, rate-limited rejections, and labelled HTTP request totals. Never carries a token, email, or pid.",
                    "responses": {
                        "200": { "description": "Prometheus text exposition", "content": { "text/plain": { "schema": { "type": "string" } } } }
                    }
                }
            }
    })
}

/// The `components` object of the `OpenAPI` document.
fn components() -> Value {
    json!({
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "PASETO",
                    "description": "PASETO v4.public access token issued by this service, verified offline against its published Ed25519 keys." }
            },
            "schemas": schemas()
    })
}

/// The `components.schemas` object, assembled from [`core_schemas`] and
/// [`compliance_schemas`] so each stays under the pedantic line budget.
fn schemas() -> Value {
    let mut schemas = serde_json::Map::new();
    for group in [core_schemas(), compliance_schemas()] {
        if let Value::Object(map) = group {
            schemas.extend(map);
        }
    }
    Value::Object(schemas)
}

/// The bulk of `components.schemas` — everything except the compliance
/// report (split into [`compliance_schemas`] to stay under the pedantic
/// line budget).
fn core_schemas() -> Value {
    json!({
                "SignupParams": { "type": "object", "required": ["email"], "properties": {
                    "email": { "type": "string", "format": "email", "description": "Email to register and the magic-link recipient." },
                    "name": { "type": "string", "nullable": true, "description": "Optional display name; defaults from the email local-part." },
                    "locale": { "type": "string", "nullable": true, "description": "Optional BCP-47 locale (e.g. 'en', 'cy') for the magic-link email language; unknown/absent falls back to English." } } },
                "MagicLinkParams": { "type": "object", "required": ["email"], "properties": {
                    "email": { "type": "string", "format": "email", "description": "Email of the existing account to sign in." },
                    "locale": { "type": "string", "nullable": true, "description": "Optional BCP-47 locale (e.g. 'en', 'cy') for the magic-link email language; unknown/absent falls back to English." } } },
                "LoginResponse": { "type": "object", "required": ["token", "pid", "name", "email", "is_verified"], "properties": {
                    "token": { "type": "string", "description": "PASETO v4.public access token (bearer)." },
                    "pid": { "type": "string", "format": "uuid", "description": "User public id." },
                    "name": { "type": "string" },
                    "email": { "type": "string", "format": "email" },
                    "is_verified": { "type": "boolean", "description": "Whether the email has been verified." } } },
                "CurrentResponse": { "type": "object", "required": ["pid", "name", "email"], "properties": {
                    "pid": { "type": "string", "format": "uuid" },
                    "name": { "type": "string" },
                    "email": { "type": "string", "format": "email" } } },
                "Claims": { "type": "object",
                    "description": "Decoded PASETO v4.public payload. sub carries the user pid; sid indexes the revocable sessions row.",
                    "required": ["sub", "email", "name", "iss", "aud", "exp", "iat", "sid"], "properties": {
                    "sub": { "type": "string", "format": "uuid", "description": "Subject — the user pid." },
                    "email": { "type": "string", "format": "email" },
                    "name": { "type": "string" },
                    "iss": { "type": "string", "description": "Issuer (default authentication-service)." },
                    "aud": { "type": "string", "description": "Audience (default main-x-service)." },
                    "exp": { "type": "integer", "format": "int64", "description": "Expiry (unix seconds)." },
                    "iat": { "type": "integer", "format": "int64", "description": "Issued-at (unix seconds)." },
                    "nbf": { "type": "integer", "format": "int64", "nullable": true, "description": "Not-before (unix seconds); omitted when absent." },
                    "sid": { "type": "string", "format": "uuid", "description": "Session id; also sessions.jid for revocation." },
                    "scope": { "type": "array", "items": { "type": "string" }, "description": "Granted scopes, if any. Deprecated for authorization — the ABAC guard decides from attrs." },
                    "roles": { "type": "array", "items": { "type": "string" }, "description": "Granted roles, if any. Deprecated for authorization — the ABAC guard decides from attrs." },
                    "attrs": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "string" } }, "description": "ABAC subject attributes (string→strings map, e.g. access: [\"write\"]), copied from the session at minting. Omitted when empty; absent on old tokens means an empty map. See agents/share/authorization-attributes.md." } } },
                "PasetoKey": { "type": "object", "required": ["kty", "crv", "use", "kid", "x"], "properties": {
                    "kty": { "type": "string", "example": "OKP" },
                    "crv": { "type": "string", "example": "Ed25519" },
                    "use": { "type": "string", "example": "sig" },
                    "kid": { "type": "string", "description": "base64url(SHA-256(public key bytes))." },
                    "x": { "type": "string", "description": "Ed25519 public key, 32 bytes (base64url)." } } },
                "PasetoKeys": { "type": "object", "required": ["keys"], "properties": {
                    "keys": { "type": "array", "description": "One or more Ed25519 signing keys. The primary (active signer) is first; additional entries are verify-only keys retained across a rotation until their last-issued tokens expire.", "items": { "$ref": "#/components/schemas/PasetoKey" } } } },
                "AuthEvent": { "type": "object",
                    "description": "One authentication audit-trail row. Never carries tokens or secrets.",
                    "required": ["id", "event", "created_at", "updated_at"], "properties": {
                    "id": { "type": "integer", "format": "int32", "description": "Row id (monotonic; newest = largest)." },
                    "event": { "type": "string", "description": "signup / magic_link_requested / magic_link_redeemed / signout / me." },
                    "email": { "type": "string", "format": "email", "nullable": true, "description": "Normalised subject email where applicable." },
                    "user_pid": { "type": "string", "format": "uuid", "nullable": true, "description": "Subject user pid when known." },
                    "detail": { "type": "string", "nullable": true, "description": "Outcome marker, e.g. rate_limited / unknown_email / invalid_or_expired / issued / ok." },
                    "created_at": { "type": "string", "format": "date-time" },
                    "updated_at": { "type": "string", "format": "date-time" } } },
                "AccountUserExport": { "type": "object",
                    "description": "The users row as exported for GDPR right of access. Excludes the password hash, api key, and live magic-link material (secrets/credentials, not subject data).",
                    "required": ["pid", "email", "name", "created_at", "updated_at"], "properties": {
                    "pid": { "type": "string", "format": "uuid" },
                    "email": { "type": "string", "format": "email" },
                    "name": { "type": "string" },
                    "email_verified_at": { "type": "string", "format": "date-time", "nullable": true },
                    "attributes": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "string" } }, "description": "ABAC subject attributes assigned to the account (subject data, not a secret); {} until an operator assigns any." },
                    "created_at": { "type": "string", "format": "date-time" },
                    "updated_at": { "type": "string", "format": "date-time" } } },
                "AccountSessionExport": { "type": "object",
                    "description": "One session row exported for GDPR right of access. jid is the session id (an opaque id, not a credential); no token is ever included.",
                    "required": ["jid", "issued_at", "expires_at"], "properties": {
                    "jid": { "type": "string", "description": "Session id — an opaque identifier, not a credential." },
                    "issued_at": { "type": "string", "format": "date-time" },
                    "expires_at": { "type": "string", "format": "date-time" },
                    "revoked_at": { "type": "string", "format": "date-time", "nullable": true },
                    "user_agent": { "type": "string", "nullable": true } } },
                "AccountAuditExport": { "type": "object",
                    "description": "One audit-trail row exported for GDPR right of access. Never carries a token or secret.",
                    "required": ["event", "created_at"], "properties": {
                    "event": { "type": "string", "description": "signup / magic_link_requested / magic_link_redeemed / signout / account_erased." },
                    "email": { "type": "string", "format": "email", "nullable": true },
                    "user_pid": { "type": "string", "format": "uuid", "nullable": true },
                    "detail": { "type": "string", "nullable": true },
                    "created_at": { "type": "string", "format": "date-time" } } },
                "AccountExport": { "type": "object",
                    "description": "GDPR Art. 15 (right of access) export: everything the service holds about the authenticated subject. No tokens, key material, password hash, or api key.",
                    "required": ["user", "sessions", "auth_events"], "properties": {
                    "user": { "$ref": "#/components/schemas/AccountUserExport" },
                    "sessions": { "type": "array", "items": { "$ref": "#/components/schemas/AccountSessionExport" } },
                    "auth_events": { "type": "array", "items": { "$ref": "#/components/schemas/AccountAuditExport" } } } },
                "UserAttributes": { "type": "object",
                    "description": "A user's ABAC subject attributes (the string→strings map minted into the PASETO attrs claim).",
                    "required": ["pid", "email", "attributes"], "properties": {
                    "pid": { "type": "string", "format": "uuid" },
                    "email": { "type": "string", "format": "email" },
                    "attributes": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "string" } },
                        "description": "e.g. { \"access\": [\"write\"], \"dept\": [\"cardiology\"] }." } } },
                "ReplaceUserAttributes": { "type": "object",
                    "description": "Full replacement ABAC attribute map. Keys/values are short lowercase tokens; sub/email/entity are reserved; no key may map to an empty list (send {} to clear).",
                    "required": ["attributes"], "properties": {
                    "attributes": { "type": "object", "additionalProperties": { "type": "array", "items": { "type": "string" } } } } },
                "Error": { "type": "object", "properties": {
                    "error": { "type": "string", "description": "Machine-readable error code, e.g. rate_limited." },
                    "description": { "type": "string" } } }
    })
}

/// The `AuditIntegrityReport` schema, split out of [`core_schemas`] to
/// stay under the pedantic line budget.
fn compliance_schemas() -> Value {
    json!({
                "AuditIntegrityReport": { "type": "object",
                    "description": "The outcome of recomputing and verifying digests over a run of auth_events rows. verified:true attests only that no examined row's content was altered — it does NOT attest that no row was deleted (see caveat).",
                    "required": ["rows", "intact", "unhashed", "sha3_intact", "sha3_unhashed", "mac_valid", "mac_absent", "mac_unverifiable", "mismatched", "verified", "caveat"],
                    "properties": {
                    "rows": { "type": "integer", "description": "Rows examined." },
                    "intact": { "type": "integer", "description": "Rows whose SHA-256 was recomputed and matched." },
                    "unhashed": { "type": "integer", "description": "Rows carrying no SHA-256 digest (written before the column existed)." },
                    "sha3_intact": { "type": "integer", "description": "Rows whose SHA-3 was recomputed and matched." },
                    "sha3_unhashed": { "type": "integer", "description": "Rows carrying no SHA-3 digest." },
                    "mac_valid": { "type": "integer", "description": "Rows whose HMAC was recomputed and matched." },
                    "mac_absent": { "type": "integer", "description": "Rows carrying no MAC (written before a key was configured)." },
                    "mac_unverifiable": { "type": "integer", "description": "Rows naming a key or scheme this service cannot check." },
                    "mismatched": { "type": "array", "items": { "type": "integer", "format": "int64" }, "description": "Row ids whose content did not match what was stored. No PII — ids only." },
                    "verified": { "type": "boolean", "description": "true when no mismatch was found." },
                    "caveat": { "type": "string", "description": "What this result does and does not attest to (deletion is not detected)." } } }
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
        assert!(s["paths"].is_object());
        assert!(s["components"]["schemas"]["LoginResponse"]["properties"]["token"].is_object());
    }

    #[test]
    fn spec_documents_every_auth_endpoint() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/auth/signup"]["post"].is_object());
        assert!(paths["/api/auth/magic-link"]["post"].is_object());
        assert!(paths["/api/auth/magic-link/{token}"]["get"].is_object());
        assert!(paths["/api/auth/me"]["get"].is_object());
        assert!(paths["/api/auth/signout"]["post"].is_object());
        assert!(paths["/api/auth/audit/recent"]["get"].is_object());
        assert!(paths["/api/auth/account/export"]["get"].is_object());
        assert!(paths["/api/auth/account/audit"]["get"].is_object());
        assert!(paths["/api/auth/account"]["delete"].is_object());
        assert!(paths["/.well-known/paseto-keys"]["get"].is_object());
        assert!(paths["/metrics.prom"]["get"].is_object());
        assert!(paths["/api/compliance/audit/verify"]["get"].is_object());
    }

    /// PRO-P23: the endpoint requires a bearer (401 documented) but is
    /// deliberately not admin-gated — no 403 — since its report carries
    /// no PII; the bearer requirement is about recomputation cost, not
    /// disclosure. See `src/controllers/compliance.rs`.
    #[test]
    fn documents_audit_verify_as_bearer_gated_not_admin_gated() {
        let s = spec();
        let ep = &s["paths"]["/api/compliance/audit/verify"]["get"];
        assert_eq!(ep["security"][0]["bearer"], serde_json::json!([]));
        assert!(ep["responses"]["401"].is_object());
        assert!(
            ep["responses"]["403"].is_null(),
            "not admin-gated: no 403 response is documented"
        );
        assert_eq!(
            ep["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/AuditIntegrityReport"
        );
        let schemas = &s["components"]["schemas"];
        assert!(schemas["AuditIntegrityReport"]["properties"]["verified"].is_object());
        assert!(schemas["AuditIntegrityReport"]["properties"]["mismatched"].is_object());
    }

    #[test]
    fn documents_the_admin_attribute_endpoints_as_admin_gated() {
        let s = spec();
        let ep = &s["paths"]["/api/auth/admin/users/{pid}/attributes"];
        // Both verbs exist and require the bearer.
        assert_eq!(ep["get"]["security"][0]["bearer"], serde_json::json!([]));
        assert_eq!(ep["put"]["security"][0]["bearer"], serde_json::json!([]));
        // The 403 (non-admin) and 422 (bad body) responses are documented.
        assert!(ep["get"]["responses"]["403"].is_object());
        assert!(ep["put"]["responses"]["403"].is_object());
        assert!(ep["put"]["responses"]["422"].is_object());
        // The referenced schemas exist.
        let schemas = &s["components"]["schemas"];
        assert!(schemas["UserAttributes"]["properties"]["attributes"].is_object());
        assert!(schemas["ReplaceUserAttributes"]["required"][0] == "attributes");
    }

    #[test]
    fn documents_the_gdpr_account_endpoints_as_bearer_gated() {
        let s = spec();
        let p = &s["paths"];
        // Export returns an AccountExport and requires the bearer token.
        assert_eq!(
            p["/api/auth/account/export"]["get"]["responses"]["200"]["content"]["application/json"]
                ["schema"]["$ref"],
            "#/components/schemas/AccountExport"
        );
        assert!(p["/api/auth/account/export"]["get"]["security"][0]["bearer"].is_array());
        // The per-subject audit + erasure are bearer-gated too.
        assert!(p["/api/auth/account/audit"]["get"]["security"][0]["bearer"].is_array());
        assert!(p["/api/auth/account"]["delete"]["security"][0]["bearer"].is_array());
        // Each documents the 401 for missing/erased.
        assert!(p["/api/auth/account/export"]["get"]["responses"]["401"].is_object());
        assert!(p["/api/auth/account"]["delete"]["responses"]["401"].is_object());
    }

    #[test]
    fn account_export_schema_excludes_secrets() {
        let s = spec();
        let schemas = &s["components"]["schemas"];
        let user = &schemas["AccountUserExport"]["properties"];
        // The export must not advertise credential/secret fields.
        for forbidden in ["password", "api_key", "magic_link_token"] {
            assert!(
                user[forbidden].is_null(),
                "AccountUserExport must not expose {forbidden}"
            );
        }
        // The composite export references the three parts.
        let export = &schemas["AccountExport"]["properties"];
        assert!(export["user"]["$ref"].is_string());
        assert!(export["sessions"]["items"]["$ref"].is_string());
        assert!(export["auth_events"]["items"]["$ref"].is_string());
    }

    #[test]
    fn claims_schema_documents_the_abac_attrs_map() {
        let s = spec();
        let attrs = &s["components"]["schemas"]["Claims"]["properties"]["attrs"];
        // The ABAC subject-attribute claim: string→strings map, optional
        // on the wire (omitted when empty).
        assert_eq!(attrs["type"], "object");
        assert_eq!(attrs["additionalProperties"]["items"]["type"], "string");
        assert!(
            !s["components"]["schemas"]["Claims"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "attrs"),
            "attrs must stay optional (empty maps are omitted from the wire)"
        );
    }

    #[test]
    fn documents_the_audit_endpoint_and_schema() {
        let s = spec();
        // The endpoint returns an array of AuthEvent.
        let resp = &s["paths"]["/api/auth/audit/recent"]["get"]["responses"]["200"];
        assert_eq!(
            resp["content"]["application/json"]["schema"]["items"]["$ref"],
            "#/components/schemas/AuthEvent"
        );
        // The schema is present and must not advertise a token field.
        let schema = &s["components"]["schemas"]["AuthEvent"];
        assert!(schema["properties"]["event"].is_object());
        assert!(schema["properties"]["detail"].is_object());
        assert!(
            schema["properties"]["token"].is_null(),
            "auth audit rows must not expose tokens"
        );
    }

    #[test]
    fn paseto_keys_endpoint_documents_multiple_keys_for_rotation() {
        let s = spec();
        // The key-set description notes it may publish multiple keys during
        // a rotation.
        let desc = s["paths"]["/.well-known/paseto-keys"]["get"]["description"]
            .as_str()
            .unwrap();
        assert!(
            desc.to_lowercase().contains("multiple"),
            "key-set description must mention multiple keys (rotation)"
        );
        // The PasetoKeys schema keys array is an array of PasetoKey.
        assert_eq!(
            s["components"]["schemas"]["PasetoKeys"]["properties"]["keys"]["items"]["$ref"],
            "#/components/schemas/PasetoKey"
        );
    }

    #[test]
    fn issuance_endpoints_document_the_429_rate_limit() {
        let s = spec();
        assert!(s["paths"]["/api/auth/signup"]["post"]["responses"]["429"].is_object());
        assert!(s["paths"]["/api/auth/magic-link"]["post"]["responses"]["429"].is_object());
    }

    #[test]
    fn bearer_security_scheme_is_present_and_applied() {
        let s = spec();
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["bearerFormat"],
            "PASETO"
        );
        // The two protected endpoints require it.
        assert!(s["paths"]["/api/auth/me"]["get"]["security"][0]["bearer"].is_array());
        assert!(s["paths"]["/api/auth/signout"]["post"]["security"][0]["bearer"].is_array());
    }

    #[test]
    fn documents_the_core_schemas() {
        let s = spec();
        let schemas = &s["components"]["schemas"];
        for name in [
            "SignupParams",
            "MagicLinkParams",
            "LoginResponse",
            "CurrentResponse",
            "Claims",
            "PasetoKeys",
            "PasetoKey",
            "AuthEvent",
            "AccountUserExport",
            "AccountSessionExport",
            "AccountAuditExport",
            "AccountExport",
        ] {
            assert!(schemas[name].is_object(), "missing schema {name}");
        }
    }
}
