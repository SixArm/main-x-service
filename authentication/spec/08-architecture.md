## 8. Architecture

> **Pivot (2026-06-17).** The flows and key-management below describe the
> **RS256 JWT + JWKS** model now **superseded** by cookie sessions +
> PASETO v4.public
> ([`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md);
> §1, §5, §9, §13 T-12). After T-12: §8.1 redemption sets a
> `__Host-mxi_session` cookie (no token in the body) and `POST /token`
> mints the PASETO the BFF carries to peers; §8.2 verifies PASETO
> v4.public (Ed25519) offline against `/.well-known/paseto-keys`; §8.4's
> key material is an **Ed25519** keypair (the rotation runbook structure
> carries over, swapping RSA→Ed25519 and JWKS→paseto-keys); §8.5's UI
> state moves to the SvelteKit-server BFF (no `localStorage`).

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

The service holds a **key set**, not a single key: one *primary*
signing key plus zero or more *additional* verify-only public keys.

- The **primary** signing material loads once at boot
  (`auth::load_keys`), resolution order: `JWT_PRIVATE_KEY_PEM` /
  `JWT_PUBLIC_KEY_PEM` inline env → `JWT_PRIVATE_KEY_FILE` /
  `JWT_PUBLIC_KEY_FILE` paths → committed dev keypair
  `config/keys/jwt_{private,public}_dev.pem`.
- **Additional** verify-only public keys load from
  `JWT_ADDITIONAL_PUBLIC_KEY_FILES` (comma-separated file paths) and/or
  `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` (inline PEM blocks, comma- or
  newline-separated). Unset/empty ⇒ just the primary
  (fully backward-compatible single-key behaviour). Keys are
  de-duplicated by `kid`; the primary always wins.
- `kid` = base64url(SHA-256(public modulus)) for **every** key —
  derived, stable, and identical in the JWKS and the token header.
- `sign_access_token` signs with the **primary** and stamps its `kid`;
  `verify_token` selects the verifying key from {primary} ∪ {additional}
  by the token header `kid`. So a token signed by a key that has since
  been rotated down to "additional" still verifies locally until it
  expires; an unknown `kid` is rejected.
- The JWKS (`/.well-known/jwks.json`) publishes the **whole set**,
  primary first, so peers trust every live `kid`.
- Misconfigured keys are a **fatal boot error** (panic with actionable
  context), never silent degradation.

#### Rotation runbook (operator-driven, zero-downtime)

This is **config-driven** — no database, no auto-rotation scheduler
(that is a follow-up). To rotate the signing key with no downtime:

1. **Generate** a fresh RSA keypair (`openssl genpkey` / `rsa` per
   `config/keys/README.md`).
2. **Promote** the new keypair to primary: set `JWT_PRIVATE_KEY_*` /
   `JWT_PUBLIC_KEY_*` to it, and **move the OLD public key** into
   `JWT_ADDITIONAL_PUBLIC_KEY_FILES` (or `…_PEMS`) so its still-live
   tokens keep verifying.
3. **Restart** the service. The JWKS now publishes both keys (new
   primary first, old key as additional). Peers refresh the JWKS at
   their next boot / on the first `UnknownKid` and now trust both
   `kid`s. New tokens are signed by the new key; old tokens still
   verify against the retained old key.
4. **Wait** at least the max access-token lifetime (`JWT_EXPIRATION`,
   default 1h) so every token signed by the old key has expired.
5. **Retire** the old key: drop it from the additional list and
   restart. The grace window (step 4) guarantees no live token is
   orphaned.

### 8.5 Module boundaries

| Module | Home | Rule |
|---|---|---|
| Token crypto | service `src/auth/` | Self-contained: `jsonwebtoken` (RS256) + `rsa` (JWK derivation). The bearer extractor is plain Axum `FromRequestParts` — reusable shape for peers. |
| Controllers | service `src/controllers/{auth,jwks}.rs` | loco controllers registered in `app.rs`; raw loco JSON, no envelope. |
| Verification | verifier `src/lib.rs` | Mirrors `auth::verify_token` keyed off the *published* JWKS instead of local key material. Same `Claims`, same `kid` selection. |
| UI state | front-end `src/lib/auth/session.svelte.ts` | Single source of client auth state (Svelte 5 runes), persisted to `localStorage`. |
