## 9. API Surface

Complete reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

### 9.1 Service REST API

Source:
[`src/controllers/auth.rs`](../authentication-service-rust-crate/src/controllers/auth.rs),
[`src/controllers/jwks.rs`](../authentication-service-rust-crate/src/controllers/jwks.rs).

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/api/auth/signup` | — | Create account, send magic link (always `200`) |
| POST | `/api/auth/magic-link` | — | Request magic link, sign in (always `200`) |
| GET | `/api/auth/magic-link/{token}` | — | Redeem link → RS256 access token + session |
| GET | `/api/auth/me` | Bearer | Current user; rejects revoked sessions |
| POST | `/api/auth/signout` | Bearer | Revoke the current session |
| GET | `/.well-known/jwks.json` | — | Public keys for offline verification |

Plus loco's default routes (`AppRoutes::with_default_routes()` in
[`src/app.rs`](../authentication-service-rust-crate/src/app.rs)),
including the framework health/ping endpoints.

Responses are **raw loco JSON** — no `{success, data, error}` envelope
(unlike the sibling entity services). Errors use loco's standard
responses: `401` unauthorized, `400` bad request.

### 9.2 Verifier library API

Source: [`src/lib.rs`](../authentication-verifier-rust-crate/src/lib.rs).

| Item | Signature |
|---|---|
| `Verifier::from_jwks_value` | `(&serde_json::Value, issuer: &str, audience: &str) -> Result<Verifier, VerifyError>` |
| `Verifier::from_jwks_url` | `async (url, issuer, audience) -> Result<Verifier, VerifyError>` — `fetch` feature |
| `Verifier::verify` | `(&self, token: &str) -> Result<Claims, VerifyError>` |
| `Verifier::key_count` | `(&self) -> usize` |
| `Claims` | §5.3 claim struct (serde) |
| `VerifyError` | `Jwks(String)` \| `MissingKid` \| `UnknownKid(String)` \| `Jwt(jsonwebtoken::errors::Error)` \| `Fetch(String)` (feature `fetch`) |

### 9.3 Front-end routes

Source:
[`src/routes/`](../authentication-front-end-with-svelte/src/routes/).

| Route | Calls |
|---|---|
| `/` | `GET /api/auth/me` (bearer); sign out → `POST /api/auth/signout` |
| `/signup` | `POST /api/auth/signup` |
| `/signin` | `POST /api/auth/magic-link` |
| `/verify?token=…` | `GET /api/auth/magic-link/{token}` → store session → redirect `/` |

### 9.4 Contract gaps

No OpenAPI / Swagger documentation yet (the sibling services ship
Swagger UI) — §13 T-8. No OIDC discovery document
(`/.well-known/openid-configuration`) — roadmap §15.
