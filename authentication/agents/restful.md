# RESTful API Reference — Authentication Entity

Entity-level summary. Normative contract: entity spec
[§9 API Surface](../spec/09-api-surface.md).

> **Auth model source of truth:**
> [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> Browser/BFF requests carry the httpOnly `__Host-mxi_session` **cookie**;
> cross-service requests carry a short-lived **PASETO v4.public** bearer,
> verified offline via the published Ed25519 key at
> `/.well-known/paseto-keys`. RS256 JWT + JWKS are **decommissioned**
> and removed from the code.

## Service REST API

loco.rs, dev port `5150`. Responses are **raw loco JSON — no
`{success, data, error}` envelope** (deliberate divergence from the
sibling entity services; the front-end client is correspondingly
leaner).

### Authentication flow

| Method | Path | Auth | Returns |
|---|---|---|---|
| POST | `/api/auth/signup` | — | `{}` always `200` — body `{email, name?}`; creates account + sends magic link |
| POST | `/api/auth/magic-link` | — | `{}` always `200` — body `{email}`; sends magic link for an existing account |
| GET | `/api/auth/magic-link/{token}` | — | establishes session + sets `__Host-mxi_session` cookie; body `{pid, name, email, is_verified}` or `401` on invalid/expired link |
| POST | `/api/auth/token` | Session | exchanges the valid session for a short-lived PASETO v4.public bearer (~5 min) |
| GET | `/api/auth/me` | Session | `{pid, name, email}`; `401` if session invalid, revoked, or account erased |
| POST | `/api/auth/signout` | Session | `{}` — revokes the session |

### Audit + GDPR subject rights

| Method | Path | Auth | Returns |
|---|---|---|---|
| GET | `/api/auth/audit/recent` | — | `AuthEvent[]` (newest 100) — open, operator-facing system feed |
| GET | `/api/auth/account/export` | Session | `AccountExport` — GDPR right of access: the subject's `users` + `sessions` + `auth_events`; `401` if erased |
| GET | `/api/auth/account/audit` | Session | `AccountAuditExport[]` — the subject's own audit trail |
| DELETE | `/api/auth/account` | Session | `{}` — GDPR erasure: soft-delete + anonymise + revoke sessions + `account_erased` audit |

The export carries no token, key material, password hash, or api key.
Erasure is soft-delete + anonymise (`users.deleted_at`; email→tombstone;
name→`"deleted user"`); see entity spec [§12](../spec/12-compliance.md).

### Key publication

| Method | Path | Auth | Returns |
|---|---|---|---|
| GET | `/.well-known/paseto-keys` | — | published Ed25519 public key(s) for offline PASETO v4.public verification: `{"keys":[{kty:"OKP",crv:"Ed25519",use,kid,x}]}` |

### Status codes

| Code | Meaning |
|---|---|
| 200 | Success (including anti-enumeration "success") |
| 400 | Bad request (loco standard) |
| 401 | Invalid/expired magic link, invalid/missing session or PASETO, revoked session, erased account, or missing credential on a gated route |

OpenAPI 3 + Swagger UI at `/api-docs/openapi.json` + `/swagger-ui`
(entity spec §13 T-8; hand-written `src/openapi.rs`).

**Source:**
[`src/controllers/auth.rs`](../authentication-service-with-loco/src/controllers/auth.rs),
[`src/controllers/paseto_keys.rs`](../authentication-service-with-loco/src/controllers/paseto_keys.rs)
(published-key endpoint), routes registered in
[`src/app.rs`](../authentication-service-with-loco/src/app.rs).

## Verifier library API

The verifier crate (already harmonized) is a **PASETO v4.public**
verifier. Building from the published Ed25519 key(s) replaces the old
JWKS constructors:

| Item | Purpose |
|---|---|
| `Verifier::from_paseto_keys_value(&keys, iss, aud)` | Build from an in-memory published-key document |
| `Verifier::from_paseto_keys_url(url, iss, aud)` | Build by fetching `/.well-known/paseto-keys` over HTTPS (`fetch` feature, async) |
| `verifier.verify(token) -> Result<Claims, VerifyError>` | Per-request verification |
| `verifier.key_count()` | Loaded public-key count |

Full usage rules: [`verification.md`](verification.md).

## Front-end consumption

| Route / action | Endpoint |
|---|---|
| `/signup` | `POST /api/auth/signup` |
| `/signin` | `POST /api/auth/magic-link` |
| `/verify?token=…` | `GET /api/auth/magic-link/{token}` (sets session cookie via the BFF) |
| `/` load | `GET /api/auth/me` (session cookie via the BFF) |
| `/` sign out | `POST /api/auth/signout` (session cookie via the BFF) |

The browser talks only to its own SvelteKit **BFF** origin carrying the
httpOnly `__Host-mxi_session` cookie; the BFF calls the service
server-side. The browser holds no token (no `localStorage`/
`mxi_access_token`). See the front-end docs (already harmonized) for the
BFF client/repository specifics.

## Configuration (service env)

| Var | Default | Purpose |
|---|---|---|
| `TOKEN_PRIVATE_KEY_SEED` | — | Primary Ed25519 signing seed (base64url 32 bytes; takes precedence) |
| `TOKEN_PRIVATE_KEY_FILE` | — | Path to a file holding the same seed; unset ⇒ built-in dev seed |
| `TOKEN_ADDITIONAL_PUBLIC_KEYS` | — | Comma-separated verify-only Ed25519 public keys (rotation) |
| `TOKEN_ISSUER` | `authentication-service` | `iss` claim |
| `TOKEN_AUDIENCE` | `main-x-service` | `aud` claim |
| `TOKEN_EXPIRATION` | `300` | PASETO TTL (seconds) |
| `FRONTEND_URL` | `http://localhost:5173` | Magic-link base |
| `DATABASE_URL` | loco config default | PostgreSQL |
