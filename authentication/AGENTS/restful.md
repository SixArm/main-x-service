# RESTful API Reference — Authentication Entity

Entity-level summary. Normative contract: entity spec
[§9 API Surface](../spec/09-api-surface.md).

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
| GET | `/api/auth/magic-link/{token}` | — | `{token, pid, name, email, is_verified}` or `401` on invalid/expired link |
| GET | `/api/auth/me` | Bearer | `{pid, name, email}`; `401` if token invalid or session revoked |
| POST | `/api/auth/signout` | Bearer | `{}` — revokes the session |

### Key publication

| Method | Path | Auth | Returns |
|---|---|---|---|
| GET | `/.well-known/jwks.json` | — | `{"keys":[{kty,use,alg,kid,n,e}]}` |

### Status codes

| Code | Meaning |
|---|---|
| 200 | Success (including anti-enumeration "success") |
| 400 | Bad request (loco standard) |
| 401 | Invalid/expired magic link, invalid token, revoked session |

No OpenAPI/Swagger yet (entity spec §13 T-8).

**Source:**
[`src/controllers/auth.rs`](../authentication-service-rust-crate/src/controllers/auth.rs),
[`src/controllers/jwks.rs`](../authentication-service-rust-crate/src/controllers/jwks.rs),
routes registered in
[`src/app.rs`](../authentication-service-rust-crate/src/app.rs).

## Verifier library API

| Item | Purpose |
|---|---|
| `Verifier::from_jwks_value(&jwks, iss, aud)` | Build from an in-memory JWKS |
| `Verifier::from_jwks_url(url, iss, aud)` | Build by fetching over HTTPS (`fetch` feature, async) |
| `verifier.verify(token) -> Result<Claims, VerifyError>` | Per-request verification |
| `verifier.key_count()` | Loaded RSA key count |

Errors: `VerifyError::{Jwks, MissingKid, UnknownKid, Jwt, Fetch}`.
Full usage rules: [`verification.md`](verification.md).

## Front-end consumption

| Route / action | Endpoint |
|---|---|
| `/signup` | `POST /api/auth/signup` |
| `/signin` | `POST /api/auth/magic-link` |
| `/verify?token=…` | `GET /api/auth/magic-link/{token}` |
| `/` load | `GET /api/auth/me` (bearer) |
| `/` sign out | `POST /api/auth/signout` (bearer) |

Base URL via `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
Client: `src/lib/api/client.ts` (lean fetch wrapper + `ApiError`),
repository: `src/lib/api/auth.ts`.

## Configuration (service env)

| Var | Default | Purpose |
|---|---|---|
| `JWT_PRIVATE_KEY_FILE` / `JWT_PUBLIC_KEY_FILE` | `config/keys/jwt_{private,public}_dev.pem` | RSA PEM paths |
| `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` | — | Inline PEM (takes precedence) |
| `JWT_ISSUER` | `authentication-service` | `iss` claim |
| `JWT_AUDIENCE` | `main-x-service` | `aud` claim |
| `JWT_EXPIRATION` | `3600` | Token TTL (seconds) |
| `FRONTEND_URL` | `http://localhost:5173` | Magic-link base |
| `DATABASE_URL` | loco config default | PostgreSQL |
