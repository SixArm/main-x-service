## 8. Architecture

### 8.1 Issuance flow (sign-in)

```
+-----------+      +---------------------------+      +------------------------+
|  Browser  |      |  auth front-end (Svelte)  |      |  auth service (loco)   |
+-----+-----+      +-------------+-------------+      +-----------+------------+
      |  /signup or /signin      |                                |
      |------------------------->| POST /api/auth/signup          |
      |                          |  or  /api/auth/magic-link      |
      |                          |------------------------------->|
      |                          |                  create/find user
      |                          |                  set magic_link_token (32 ch, 5 min)
      |                          |          200 {} <--------------|
      |                          |                                |--> email (prod SMTP)
      |                          |                                |--> tracing log (dev)
      |   user opens the link: {FRONTEND_URL}/verify?token=...    |
      |------------------------->| GET /api/auth/magic-link/{token}
      |                          |------------------------------->|
      |                          |                  validate + clear token
      |                          |                  mark email verified
      |                          |                  sign RS256 JWT (kid header)
      |                          |                  INSERT sessions (jid = jti)
      |                          | {token, pid, name, email, ...} |
      |   store in localStorage  |<-------------------------------|
      |<-------------------------|                                |
```

### 8.2 Verification flow (every peer request)

```
+-----------+        +--------------------------------------+
|  Client   |        |  peer service (person, worker, ...)  |
+-----+-----+        |  +--------------------------------+  |
      | Authorization:  |  authentication-verifier        |  |
      | Bearer <jwt>    |  Verifier { keys by kid,        |  |
      |---------------->|             iss/aud policy }    |  |
      |                 |  verify(): kid -> key ->        |  |
      |                 |  signature + iss + aud + exp    |  |
      |                 +---------------+----------------+  |
      +--------------------------------|--------------------+
                                       |  boot-time only (not per request)
                                       v
                      GET https://auth/.well-known/jwks.json
```

The JWKS is fetched **once at boot** (`Verifier::from_jwks_url`,
`fetch` feature) or supplied from config, then cached for the process
lifetime; a `VerifyError::UnknownKid` is the refetch trigger after a
key rotation. The request hot path never leaves the process.

### 8.3 Deployment topology

- **Auth service** — stateless loco.rs app over PostgreSQL
  (`users` + `sessions` + the Postgres-backed worker queue). Scales
  horizontally; dev port `5150`.
- **Front-end** — static SPA (SvelteKit, SSR off) served from any
  static host; talks to the service via `PUBLIC_API_BASE_URL`.
- **Verifier** — not deployed; compiled into each peer service.
- **Blast-radius property** — an auth-service outage stops new
  sign-ins and JWKS refresh only; peers keep verifying cached-key
  traffic until token expiry (NFR-2).

### 8.4 Key management

- Signing material loads once at boot (`auth::load_keys`), resolution
  order: `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` inline env →
  `JWT_PRIVATE_KEY_FILE` / `JWT_PUBLIC_KEY_FILE` paths → committed dev
  keypair `config/keys/jwt_{private,public}_dev.pem`.
- `kid` = base64url(SHA-256(public modulus)) — derived, stable, and
  identical in the JWKS and every token header.
- Misconfigured keys are a **fatal boot error** (panic with actionable
  context), never silent degradation.
- Rotation (roadmap §13 T-5): publish old + new JWKs together, sign
  with the new key, retire the old after a grace window ≥ max token
  TTL.

### 8.5 Module boundaries

| Module | Home | Rule |
|---|---|---|
| Token crypto | service `src/auth/` | Self-contained: `jsonwebtoken` (RS256) + `rsa` (JWK derivation). The bearer extractor is plain Axum `FromRequestParts` — reusable shape for peers. |
| Controllers | service `src/controllers/{auth,jwks}.rs` | loco controllers registered in `app.rs`; raw loco JSON, no envelope. |
| Verification | verifier `src/lib.rs` | Mirrors `auth::verify_token` keyed off the *published* JWKS instead of local key material. Same `Claims`, same `kid` selection. |
| UI state | front-end `src/lib/auth/session.svelte.ts` | Single source of client auth state (Svelte 5 runes), persisted to `localStorage`. |
