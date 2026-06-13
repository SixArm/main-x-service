//! Hand-written `OpenAPI` 3 description of the authentication REST API.
//!
//! The family authors `OpenAPI` documents by hand (no `utoipa`), which
//! keeps the schema accurate to the wire format and the crate
//! dependency-light. This document covers the passwordless magic-link
//! surface (FR-1…FR-8): signup, magic-link issuance, redemption, `me`,
//! signout, and the public JWKS. The bearer `securityScheme` describes the
//! RS256 access token that `me` and `signout` require.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
// One contiguous `json!` literal: splitting it into helpers would scatter
// the document and hurt readability, so the length is allowed.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Authentication Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Central single sign-on provider for the Main X Index family. Passwordless email magic-link authentication; issues RS256 JWT access tokens verifiable offline against the JWKS at /.well-known/jwks.json. The unauthenticated issuance endpoints (signup, magic-link) always return 200 to avoid account enumeration, and are rate-limited per email (429 when exceeded). me and signout require a bearer token."
        },
        "paths": {
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
                    "summary": "Consume a magic link → RS256 access token + session",
                    "description": "Validates the unexpired, single-use token, marks the email verified, issues an RS256 access token, and records a revocable session.",
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
                    "description": "Returns the current user. Honors local revocation: a signed-out session is rejected even though the JWT signature is still valid.",
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
                    "description": "Marks the bearer token's session revoked. Peer services that cached the token keep honoring it until expiry (offline JWKS verification); TTLs are kept short to bound this window.",
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
            },
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
            },
            "/.well-known/jwks.json": {
                "get": {
                    "tags": ["jwks"],
                    "summary": "Public keys for offline token verification",
                    "description": "The JSON Web Key Set peer services fetch once to verify RS256 tokens locally — no shared secret, no introspection round-trip. May publish MULTIPLE keys during a key rotation: the primary (active signing) key is first, followed by any additional verify-only keys whose recently-issued tokens are still within their lifetime. Peers select the verifying key by the token header kid, so every published kid is trusted until it is retired.",
                    "responses": {
                        "200": { "description": "JWKS document", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Jwks" } } } }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT",
                    "description": "RS256 access token issued by this service, verified offline against its JWKS." }
            },
            "schemas": {
                "SignupParams": { "type": "object", "required": ["email"], "properties": {
                    "email": { "type": "string", "format": "email", "description": "Email to register and the magic-link recipient." },
                    "name": { "type": "string", "nullable": true, "description": "Optional display name; defaults from the email local-part." },
                    "locale": { "type": "string", "nullable": true, "description": "Optional BCP-47 locale (e.g. 'en', 'cy') for the magic-link email language; unknown/absent falls back to English." } } },
                "MagicLinkParams": { "type": "object", "required": ["email"], "properties": {
                    "email": { "type": "string", "format": "email", "description": "Email of the existing account to sign in." },
                    "locale": { "type": "string", "nullable": true, "description": "Optional BCP-47 locale (e.g. 'en', 'cy') for the magic-link email language; unknown/absent falls back to English." } } },
                "LoginResponse": { "type": "object", "required": ["token", "pid", "name", "email", "is_verified"], "properties": {
                    "token": { "type": "string", "description": "RS256 access token (bearer)." },
                    "pid": { "type": "string", "format": "uuid", "description": "User public id." },
                    "name": { "type": "string" },
                    "email": { "type": "string", "format": "email" },
                    "is_verified": { "type": "boolean", "description": "Whether the email has been verified." } } },
                "CurrentResponse": { "type": "object", "required": ["pid", "name", "email"], "properties": {
                    "pid": { "type": "string", "format": "uuid" },
                    "name": { "type": "string" },
                    "email": { "type": "string", "format": "email" } } },
                "Claims": { "type": "object",
                    "description": "Decoded RS256 JWT payload. sub carries the user pid; jti also indexes the revocable sessions row.",
                    "required": ["sub", "email", "name", "iss", "aud", "exp", "iat", "jti"], "properties": {
                    "sub": { "type": "string", "format": "uuid", "description": "Subject — the user pid." },
                    "email": { "type": "string", "format": "email" },
                    "name": { "type": "string" },
                    "iss": { "type": "string", "description": "Issuer (default authentication-service)." },
                    "aud": { "type": "string", "description": "Audience (default main-x-service)." },
                    "exp": { "type": "integer", "format": "int64", "description": "Expiry (unix seconds)." },
                    "iat": { "type": "integer", "format": "int64", "description": "Issued-at (unix seconds)." },
                    "jti": { "type": "string", "format": "uuid", "description": "JWT id; also sessions.jid for revocation." } } },
                "Jwk": { "type": "object", "required": ["kty", "use", "alg", "kid", "n", "e"], "properties": {
                    "kty": { "type": "string", "example": "RSA" },
                    "use": { "type": "string", "example": "sig" },
                    "alg": { "type": "string", "example": "RS256" },
                    "kid": { "type": "string", "description": "base64url(SHA-256(public modulus))." },
                    "n": { "type": "string", "description": "RSA modulus (base64url)." },
                    "e": { "type": "string", "description": "RSA exponent (base64url)." } } },
                "Jwks": { "type": "object", "required": ["keys"], "properties": {
                    "keys": { "type": "array", "description": "One or more RSA signing keys. The primary (active signer) is first; additional entries are verify-only keys retained across a rotation until their last-issued tokens expire.", "items": { "$ref": "#/components/schemas/Jwk" } } } },
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
                    "created_at": { "type": "string", "format": "date-time" },
                    "updated_at": { "type": "string", "format": "date-time" } } },
                "AccountSessionExport": { "type": "object",
                    "description": "One session row exported for GDPR right of access. jid is the JWT jti (an opaque id, not a credential); no token is ever included.",
                    "required": ["jid", "issued_at", "expires_at"], "properties": {
                    "jid": { "type": "string", "description": "JWT id (jti) — an opaque identifier, not a credential." },
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
                "Error": { "type": "object", "properties": {
                    "error": { "type": "string", "description": "Machine-readable error code, e.g. rate_limited." },
                    "description": { "type": "string" } } }
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
        assert!(paths["/.well-known/jwks.json"]["get"].is_object());
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
    fn jwks_endpoint_documents_multiple_keys_for_rotation() {
        let s = spec();
        // The JWKS description notes it may publish multiple keys during a
        // rotation (key-rotation T-5).
        let desc = s["paths"]["/.well-known/jwks.json"]["get"]["description"]
            .as_str()
            .unwrap();
        assert!(
            desc.to_lowercase().contains("multiple"),
            "JWKS description must mention multiple keys (rotation)"
        );
        // The Jwks schema keys array is still an array of Jwk.
        assert_eq!(
            s["components"]["schemas"]["Jwks"]["properties"]["keys"]["items"]["$ref"],
            "#/components/schemas/Jwk"
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
            "JWT"
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
            "Jwks",
            "Jwk",
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
