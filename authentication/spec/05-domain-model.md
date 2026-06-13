## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](../AGENTS/models.md).
Source: service
[`src/models/users.rs`](../authentication-service-rust-crate/src/models/users.rs),
[`src/models/sessions.rs`](../authentication-service-rust-crate/src/models/sessions.rs),
[`src/auth/mod.rs`](../authentication-service-rust-crate/src/auth/mod.rs);
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

### 5.2 `Session` (service)

One row per issued token; the unit of revocation.

- `jid` (unique) — equals the token `jti`.
- `user_pid` — the holder's `pid`.
- `expires_at` — mirrors the token `exp`.
- `revoked_at` — set on signout; `is_active()` = `revoked_at IS NULL`.
- `user_agent` — optional issuance context.

### 5.3 JWT `Claims` (contract — both crates)

Defined identically in the service (`auth::Claims`) and the verifier
(`authentication_verifier::Claims`) so a token signed at one
round-trips at the other:

| Claim | Type | Content |
|---|---|---|
| `sub` | String | User `pid` (UUID string) |
| `email` | String | User email, for convenience at the edge |
| `name` | String | Display name |
| `iss` | String | Issuer — default `authentication-service` (`JWT_ISSUER`) |
| `aud` | String | Audience — default `main-x-service` (`JWT_AUDIENCE`) |
| `exp` | i64 | Expiry, unix seconds (`iat` + `JWT_EXPIRATION`, default 3600) |
| `iat` | i64 | Issued-at, unix seconds |
| `jti` | String | JWT id (UUID) = `sessions.jid` |

Token header: `alg: RS256`, `kid` = base64url(SHA-256(public modulus)).

### 5.4 JWKS key set (contract)

```json
{ "keys": [ { "kty": "RSA", "use": "sig", "alg": "RS256",
              "kid": "…", "n": "…", "e": "…" } ] }
```

`n` / `e` are base64url-no-pad big-endian RSA components. One key
today; the document is an array so rotation can publish old + new
keys side by side (§13 T-5).

### 5.5 Invariants

The implementations MUST enforce:

- Access tokens are RS256 — never HS256, never a shared secret.
- A magic-link token is single-use and expires within 5 minutes.
- `sessions.jid` equals the token `jti`; signout sets `revoked_at`.
- The published JWKS `kid` equals the `kid` stamped into token headers.
- `signup` / `magic-link` responses never reveal account existence.
- The verifier rejects tokens whose `kid` is absent or unknown, and
  validates signature, `iss`, `aud`, and `exp` on every call.
