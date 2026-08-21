## 9. API Surface

Complete reference: [`agents/restful.md`](../agents/restful.md).

### 9.1 Service REST API

Source:
[`src/controllers/auth.rs`](../authentication-service-with-loco/src/controllers/auth.rs),
[`src/controllers/token.rs`](../authentication-service-with-loco/src/controllers/token.rs),
[`src/controllers/paseto_keys.rs`](../authentication-service-with-loco/src/controllers/paseto_keys.rs).

Auth column: **Cookie** = `__Host-mxi_session` session cookie;
**+CSRF** = also requires a valid CSRF token (§6.4c, FR-8f).

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/api/auth/signup` | — | Create account, send magic link (always `200`) |
| POST | `/api/auth/magic-link` | — | Request magic link, sign in (always `200`) |
| GET | `/api/auth/magic-link/{token}` | — | Redeem link → **`Set-Cookie: __Host-mxi_session`** + session row (no token in body) |
| POST | `/token` | Cookie +CSRF | Exchange the session for a short-lived **PASETO v4.public** (`exp` ~5 min) |
| GET | `/api/auth/me` | Cookie | Current user; resolves + slides the session, rejects expired/revoked |
| POST | `/api/auth/signout` | Cookie +CSRF | Revoke the session (`revoked_at`) + clear the cookie |
| GET | `/api/auth/audit/recent` | — | Recent authentication audit events (`AuthEvent[]`, newest 100) |
| GET | `/api/auth/account/export` | Cookie | GDPR right of access: the subject's `users` + `sessions` + `auth_events` (`AccountExport`) |
| GET | `/api/auth/account/audit` | Cookie | GDPR right of access: the subject's own audit trail (`AccountAuditExport[]`) |
| DELETE | `/api/auth/account` | Cookie +CSRF | GDPR right to erasure: soft-delete + anonymise + revoke sessions + audit |
| GET | `/.well-known/paseto-keys` | — | Ed25519 public keys for offline PASETO verification |

> **Decommissioned (this pivot).** `GET /.well-known/jwks.json` and
> RS256 access-token issuance are removed; the magic-link redemption no
> longer returns a bearer token. The shared design allowed keeping the
> JWKS transitionally during peer migration
> ([`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> §9); in practice it was removed outright — no JWKS or RS256 path
> remains in the code. Bearer-token auth is
> superseded by the session cookie (browser↔service) and PASETO
> (service↔service).

`/api/auth/audit/recent` returns the durable `auth_events` audit trail
(T-10; see §10 + §12). It is deliberately unauthenticated (operator
system feed), mirroring the sibling care-pathway service's
`/audit/recent`; rows carry no tokens or secrets. The GDPR right of
access (T-9) is served instead by the bearer-gated, per-subject
`/api/auth/account/audit` + `/api/auth/account/export`, so a subject's
own data is reachable only by that subject.

Plus loco's default routes (`AppRoutes::with_default_routes()` in
[`src/app.rs`](../authentication-service-with-loco/src/app.rs)),
including the framework health/ping endpoints.

Responses are **raw loco JSON** — no `{success, data, error}` envelope
(unlike the sibling entity services). Errors use loco's standard
responses: `401` unauthorized, `400` bad request.

### 9.2 Verifier library API

Source: [`src/lib.rs`](../authentication-verifier-rust-crate/src/lib.rs).

| Item | Signature |
|---|---|
| `Verifier::from_paseto_keys_value` | `(&serde_json::Value, issuer: &str, audience: &str) -> Result<Verifier, VerifyError>` |
| `Verifier::from_paseto_keys_url` | `async (url, issuer, audience) -> Result<Verifier, VerifyError>` — `fetch` feature |
| `Verifier::verify` | `(&self, token: &str) -> Result<Claims, VerifyError>` |
| `Verifier::key_count` | `(&self) -> usize` |
| `Claims` | §5.3 claim struct (serde) |
| `VerifyError` | `Keys(String)` \| `MissingKid` \| `UnknownKid(String)` \| `Paseto(String)` \| `Fetch(String)` (feature `fetch`) |

*(These replace the prior RS256 `from_jwks_value` / `from_jwks_url` /
`Jwks` / `Jwt` items; see shared §5.)*

### 9.3 Front-end routes

Source:
[`src/routes/`](../authentication-front-end-with-svelte/src/routes/).

All calls go through the SvelteKit-server BFF (shared §6); the browser
carries only the `__Host-mxi_session` cookie.

| Route | Calls |
|---|---|
| `/` | `GET /api/auth/me` (cookie); sign out → `POST /api/auth/signout` (cookie +CSRF) |
| `/signup` | `POST /api/auth/signup` |
| `/signin` | `POST /api/auth/magic-link` |
| `/verify?token=…` | `GET /api/auth/magic-link/{token}` → server receives `Set-Cookie` → redirect `/` |

### 9.4 Contract gaps

No OpenAPI / Swagger documentation yet (the sibling services ship
Swagger UI) — §13 T-8. No OIDC discovery document
(`/.well-known/openid-configuration`) — roadmap §15.
