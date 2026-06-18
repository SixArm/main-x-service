## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](../AGENTS/models.md).
Source: service
[`src/models/users.rs`](../authentication-service-with-loco/src/models/users.rs),
[`src/models/sessions.rs`](../authentication-service-with-loco/src/models/sessions.rs),
[`src/auth/mod.rs`](../authentication-service-with-loco/src/auth/mod.rs);
verifier [`src/lib.rs`](../authentication-verifier-rust-crate/src/lib.rs).

### 5.1 `User` (service)

The sign-in account — **not** a person registry record.

- **Identity** — internal `id` (auto-increment) + public `pid` (UUID,
  the token `sub`) + unique `email`.
- **Profile** — `name` (display name; defaulted from the email local
  part when omitted at sign-up).
- **Magic link** — `magic_link_token` (random 32 chars) +
  `magic_link_expiration` (now + 5 min); both cleared on redemption.
- **Verification** — `email_verified_at`, set on first successful
  magic-link redemption.
- **Legacy loco-starter columns** — `password` (holds an unusable
  random Argon2 hash; never checked), `api_key`, `reset_token` /
  `reset_sent_at`, `email_verification_token` /
  `email_verification_sent_at`. Retained to satisfy the schema; no
  password flow exists.

### 5.2 `Session` (service) — the human login

The server-side session, per
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
§3. One row per logged-in browser; the unit of revocation. The browser
holds only the opaque `sid` in the `__Host-mxi_session` cookie.

| Column | Type | Content |
|---|---|---|
| `sid` | text, pk | Opaque, high-entropy session id (**not** a JWT); rotated on privilege change |
| `user_pid` | uuid | The holder's `pid` |
| `data` | jsonb | Session attributes — roles, scopes, MFA state, … (default `{}`) |
| `created_at` | timestamptz | Login time |
| `last_seen_at` | timestamptz | Sliding idle marker, bumped on use |
| `idle_expires_at` | timestamptz | `now() + idle TTL`, bumped on use |
| `absolute_expires_at` | timestamptz | Hard ceiling, never extended |
| `revoked_at` | timestamptz, null | Explicit logout / admin revoke |

A session is valid iff
`revoked_at IS NULL AND now() < idle_expires_at AND now() < absolute_expires_at`.
Partial index `sessions_user ON (user_pid) WHERE revoked_at IS NULL`.

### 5.3 PASETO `Claims` (cross-service contract — both crates)

Carried in the short-lived **PASETO v4.public** token minted by
`POST /token` (shared §5). Defined identically in the service and the
verifier (`authentication_verifier::Claims`) so a token signed at one
round-trips at the other:

| Claim | Type | Content |
|---|---|---|
| `sub` | String | User `pid` (UUID string) |
| `iss` | String | Issuer — default `authentication-service` (`PASETO_ISSUER`) |
| `aud` | String | Audience — default `main-x-service` (`PASETO_AUDIENCE`) |
| `iat` | i64 | Issued-at, unix seconds |
| `nbf` | i64 | Not-before, unix seconds |
| `exp` | i64 | Expiry, unix seconds (`iat` + ~5 min, `PASETO_EXPIRATION`) |
| `sid` | String | Originating `sessions.sid` (for revocation correlation) |
| `scope` / `roles` | String / [String] | Authorization hints carried from `sessions.data` |

Token footer carries `kid` (selects the verifying Ed25519 key).
`email` / `name` are no longer carried by default (fetched at the edge
where needed); add them to `data`/claims only if a peer requires them.

### 5.4 PASETO public-key set (contract)

The Ed25519 public key(s) published at `/.well-known/paseto-keys` — the
JWKS analog. Document shape (one entry per `kid`):

```json
{ "keys": [ { "kty": "OKP", "crv": "Ed25519", "use": "sig",
              "alg": "EdDSA", "kid": "…", "x": "…" } ] }
```

`x` is the base64url-no-pad Ed25519 public key. One key today; the
document is an array so rotation can publish old + new keys side by
side (the footer `kid` selects the verifier key).

### 5.5 Invariants

The implementations MUST enforce:

- The **human session is a server-side cookie session**, never a token
  in browser JS; the cookie is `__Host-mxi_session` (HttpOnly, Secure,
  SameSite, host-locked) carrying only the opaque `sid`.
- A magic-link token is single-use and expires within 5 minutes; its
  redemption **establishes a session + sets the cookie** (no JWT/PASETO
  returned to the browser).
- Cross-service tokens are **PASETO v4.public** (Ed25519) — never JWT,
  never HS256, never a shared secret — minted only by exchanging a
  valid session at `POST /token`, with a ~5-minute `exp`.
- The published key-set `kid` equals the `kid` stamped into token
  footers.
- Logout / revocation sets `sessions.revoked_at`; cross-service
  revocation relies on the short PASETO `exp` (shared §10 open
  question).
- CSRF protection is enforced on cookie-authenticated mutating requests
  (shared §4).
- `signup` / `magic-link` responses never reveal account existence.
- The verifier rejects tokens whose footer `kid` is absent or unknown,
  and validates signature, `iss`, `aud`, and `exp` on every call.

> RS256 JWT + `/.well-known/jwks.json` are **decommissioned** by this
> pivot (§1). See
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> §9 for the rollout.
