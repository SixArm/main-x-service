# Domain Model Reference — Authentication Entity

Entity-level summary. Normative contract: entity spec
[§5 Domain Model](../spec/05-domain-model.md).

## User

The sign-in account (not a person registry record — identity
attributes live in the person entity).

**Files:**
[`src/models/users.rs`](../authentication-service-rust-crate/src/models/users.rs),
entity in `src/models/_entities/users.rs`, migration
[`m20220101_000001_users.rs`](../authentication-service-rust-crate/src/migration/m20220101_000001_users.rs).

| Field | Type | Description |
|---|---|---|
| id | i32 (pk auto) | Internal id |
| pid | Uuid | Public id — the token `sub` |
| email | String (unique) | Sign-in identity (personal data) |
| password | String | Unusable random Argon2 hash — no password flow exists |
| api_key | String (unique) | loco-starter convention (`lo-{uuid}`) |
| name | String | Display name (defaulted from email local part) |
| reset_token / reset_sent_at | Option | Legacy loco-starter, unused |
| email_verification_token / email_verification_sent_at | Option | Legacy loco-starter, unused |
| email_verified_at | Option\<DateTime\> | Set on first magic-link redemption |
| magic_link_token | Option\<String\> | Live link token — 32 random chars (`MAGIC_LINK_LENGTH`) |
| magic_link_expiration | Option\<DateTime\> | Now + 5 min (`MAGIC_LINK_EXPIRATION_MIN`) |

**Key methods:** `create_passwordless(db, email, name)`,
`find_by_email`, `find_by_pid`, `find_by_magic_token` (enforces
expiry), `ActiveModel::create_magic_link`,
`ActiveModel::clear_magic_link`, `ActiveModel::verified`.

## Session

One row per issued token; the unit of revocation.

**Files:**
[`src/models/sessions.rs`](../authentication-service-rust-crate/src/models/sessions.rs),
migration
[`m20220101_000002_sessions.rs`](../authentication-service-rust-crate/src/migration/m20220101_000002_sessions.rs).

| Field | Type | Description |
|---|---|---|
| id | i32 (pk auto) | |
| jid | String (unique) | = the token `jti` |
| user_pid | Uuid | Holder |
| expires_at | DateTime\<FixedOffset\> | = the token `exp` |
| revoked_at | Option\<DateTime\> | Set on signout |
| user_agent | Option\<String\> | Issuance context |

**Key methods:** `issue(db, jid, user_pid, expires_at, user_agent)`,
`find_by_jid`, `is_active()` (= `revoked_at.is_none()`),
`ActiveModel::revoke`.

## Claims (cross-crate contract)

Defined twice, byte-compatible by convention — service
[`src/auth/mod.rs`](../authentication-service-rust-crate/src/auth/mod.rs)
and verifier
[`src/lib.rs`](../authentication-verifier-rust-crate/src/lib.rs):

| Claim | Type | Content |
|---|---|---|
| sub | String | User `pid` (UUID string) |
| email | String | User email |
| name | String | Display name |
| iss | String | Default `authentication-service` |
| aud | String | Default `main-x-service` |
| exp | i64 | Unix seconds; default `iat` + 3600 |
| iat | i64 | Unix seconds |
| jti | String | UUID = `sessions.jid` |

Header: `alg: RS256`, `kid` = base64url(SHA-256(RSA public modulus)).

## AuthKeys / JWKS (service)

`auth::AuthKeys` holds the encoding/decoding keys, `kid`, `issuer`,
`audience`, `expiration`, and the pre-rendered `jwks` JSON value
served verbatim by the JWKS controller:

```json
{ "keys": [ { "kty": "RSA", "use": "sig", "alg": "RS256",
              "kid": "…", "n": "…", "e": "…" } ] }
```

## Verifier (peer side)

`Verifier { keys: HashMap<kid, DecodingKey>, validation }` — see
[`verification.md`](verification.md) for the full API and usage rules.

## View models

**Service** ([`src/views/auth.rs`](../authentication-service-rust-crate/src/views/auth.rs)):
`LoginResponse { token, pid, name, email, is_verified }`,
`CurrentResponse { pid, name, email }`.

**Front-end** (`src/lib/api/types.ts`): mirrors of the two views;
session state in `src/lib/auth/session.svelte.ts` persisted to
`localStorage` (`mxi.auth.token`, `mxi.auth.user`).
