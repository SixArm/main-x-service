## 8. Architecture

> **Pivot (2026-06-17, landed).** The flows and key-management below
> describe the **cookie-session + PASETO v4.public** model
> ([`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md);
> §1, §5, §9, §13 T-12), which **supersedes** the decommissioned
> RS256 JWT + JWKS model: §8.1 redemption sets a
> `__Host-mxi_session` cookie and `POST /token`
> mints the PASETO the BFF carries to peers; §8.2 verifies PASETO
> v4.public (Ed25519) offline against `/.well-known/paseto-keys`; §8.4's
> key material is an **Ed25519** keypair (the rotation runbook structure
> carried over from the old model, swapping RSA→Ed25519 and
> JWKS→paseto-keys); §8.5's UI
> state lives in the SvelteKit-server BFF (no `localStorage`).

### 8.1 Issuance flow (sign-in)

```
+-----------+      +---------------------------+      +------------------------+
|  Browser  |      |  auth front-end (BFF)     |      |  auth service (loco)   |
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
      |                          |                  INSERT sessions (opaque sid)
      |                          | Set-Cookie: __Host-mxi_session |
      |                          | {pid, name, email, ...}        |
      |   httpOnly cookie only   |<-------------------------------|
      |<-------------------------|  (no token reaches browser JS) |
```

For cross-service calls the BFF exchanges the session for a short-lived
token: `POST /api/auth/token` (session cookie) → PASETO v4.public
(Ed25519, `exp` ~5 min, footer `kid`), which the BFF sends to peers as
`Authorization: Bearer v4.public.…`. *(Transitionally, magic-link
redemption also still returns the PASETO in the response body until
every front-end adopts the BFF.)*

### 8.2 Verification flow (every peer request)

```
+-----------+        +--------------------------------------+
|  Client   |        |  peer service (person, worker, ...)  |
+-----+-----+        |  +--------------------------------+  |
      | Authorization:  |  authentication-verifier        |  |
      | Bearer          |  Verifier { Ed25519 keys by kid,|  |
      | v4.public.…     |             iss/aud policy }    |  |
      |---------------->|  verify(): footer kid -> key -> |  |
      |                 |  signature + iss/aud/exp/nbf    |  |
      |                 +---------------+----------------+  |
      +--------------------------------|--------------------+
                                       |  boot-time only (not per request)
                                       v
                      GET https://auth/.well-known/paseto-keys
```

The key set is fetched **once at boot**
(`Verifier::from_paseto_keys_url`,
`fetch` feature) or supplied from config, then cached for the process
lifetime; a `VerifyError::UnknownKid` is the refetch trigger after a
key rotation. The request hot path never leaves the process.

### 8.3 Deployment topology

- **Auth service** — stateless loco.rs app over PostgreSQL
  (`users` + `sessions` + the Postgres-backed worker queue). Scales
  horizontally; dev port `5150`.
- **Front-end** — SvelteKit app acting as a **BFF**: its server routes
  hold the `__Host-mxi_session` cookie and call the service; the
  browser talks only to the front-end's own origin.
- **Verifier** — not deployed; compiled into each peer service.
- **Blast-radius property** — an auth-service outage stops new
  sign-ins, token minting, and key-set refresh only; peers keep
  verifying cached-key
  traffic until token expiry (NFR-2).

### 8.4 Key management

The service holds a **key set**, not a single key: one *primary*
signing key plus zero or more *additional* verify-only public keys.

- The **primary** Ed25519 signing seed loads once at boot
  (`auth::load_keys`), resolution order: `TOKEN_PRIVATE_KEY_SEED`
  (inline base64url 32-byte seed) → `TOKEN_PRIVATE_KEY_FILE` (a file
  holding the same) → the built-in dev seed (`DEV_SEED`, development
  only; no key files are committed).
- **Additional** verify-only public keys load from
  `TOKEN_ADDITIONAL_PUBLIC_KEYS` (comma-separated base64url 32-byte
  Ed25519 public keys). Unset/empty ⇒ just the primary
  (fully backward-compatible single-key behaviour). Keys are
  de-duplicated by `kid`; the primary always wins.
- `kid` = base64url(SHA-256(public key bytes)) for **every** key —
  derived, stable, and identical in the published key set and the
  token footer.
- `sign_access_token` signs with the **primary** and stamps its `kid`
  into the footer;
  `verify_token` selects the verifying key from {primary} ∪ {additional}
  by the token footer `kid`. So a token signed by a key that has since
  been rotated down to "additional" still verifies locally until it
  expires; an unknown `kid` is rejected.
- The key set (`/.well-known/paseto-keys`) publishes the **whole set**,
  primary first, so peers trust every live `kid`.
- Misconfigured keys are a **fatal boot error** (panic with actionable
  context), never silent degradation.

#### Rotation runbook (operator-driven, zero-downtime)

This is **config-driven** — no database, no auto-rotation scheduler
(that is a follow-up). To rotate the signing key with no downtime:

1. **Generate** a fresh Ed25519 seed (32 random bytes, base64url — per
   `config/keys/README.md`).
2. **Promote** the new seed to primary: set `TOKEN_PRIVATE_KEY_SEED`
   (or `TOKEN_PRIVATE_KEY_FILE`) to it, and **move the OLD public key**
   into `TOKEN_ADDITIONAL_PUBLIC_KEYS` so its still-live
   tokens keep verifying.
3. **Restart** the service. The key set now publishes both keys (new
   primary first, old key as additional). Peers refresh the key set at
   their next boot / on the first `UnknownKid` and now trust both
   `kid`s. New tokens are signed by the new key; old tokens still
   verify against the retained old key.
4. **Wait** at least the max access-token lifetime (`TOKEN_EXPIRATION`,
   default 300 s) so every token signed by the old key has expired.
5. **Retire** the old key: drop it from the additional list and
   restart. The grace window (step 4) guarantees no live token is
   orphaned.

### 8.5 Module boundaries

| Module | Home | Rule |
|---|---|---|
| Token crypto | service `src/auth/` | Self-contained: `rusty_paseto` (v4.public) + `ed25519-dalek` (key handling / `kid` derivation). The bearer extractor is plain Axum `FromRequestParts` — reusable shape for peers. |
| Session cookie | service `src/cookie.rs` | Pure helpers to build / clear / parse the httpOnly `__Host-mxi_session` cookie. |
| Controllers | service `src/controllers/{auth,paseto_keys}.rs` | loco controllers registered in `app.rs`; raw loco JSON, no envelope. |
| Verification | verifier `src/lib.rs` | Mirrors `auth::verify_token` keyed off the *published* key set instead of local key material. Same `Claims`, same footer-`kid` selection. |
| UI state | front-end SvelteKit server (`hooks.server.ts` / `+page.server.ts`) | The BFF holds the session cookie server-side; browser JS holds no credential and no `localStorage` token. |
