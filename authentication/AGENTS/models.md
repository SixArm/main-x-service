# Domain Model Reference — Authentication Entity

Entity-level summary. Normative contract: entity spec
[§5 Domain Model](../spec/05-domain-model.md).

> **Auth model source of truth:**
> [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> The session is a server-side httpOnly **cookie session**; cross-service
> auth is short-lived **PASETO v4.public** verified offline via the
> published Ed25519 key at `/.well-known/paseto-keys`. RS256 JWT + JWKS
> are **decommissioned**. **Pivot in progress** — the service code
> follow-up is tracked in the service spec §13, so the `Session` columns
> and key endpoint below may still reflect the RS256 era until then.

## User

The sign-in account (not a person registry record — identity
attributes live in the person entity).

**Files:**
[`src/models/users.rs`](../authentication-service-with-loco/src/models/users.rs),
entity in `src/models/_entities/users.rs`, migration
[`m20220101_000001_users.rs`](../authentication-service-with-loco/src/migration/m20220101_000001_users.rs).

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
| deleted_at | Option\<DateTime\> | GDPR Art. 17 soft-delete (migration `m20220101_000004_users_deleted_at`); when set the account is anonymised + treated as gone |

**Key methods:** `create_passwordless(db, email, name)`,
`find_by_email`, `find_by_pid`, `find_active_by_pid` (excludes
GDPR-erased accounts), `is_deleted()`, `find_by_magic_token` (enforces
expiry), `ActiveModel::create_magic_link`,
`ActiveModel::clear_magic_link`, `ActiveModel::verified`,
`ActiveModel::erase` (GDPR soft-delete + anonymise). The pure
`tombstone_email(pid)` + `TOMBSTONE_NAME` shape the anonymised values.

## Session

The unit of revocation. In the **target** model (auth-sessions design)
a session is the source of truth for being logged in: an opaque,
high-entropy `sid` carried in the httpOnly `__Host-mxi_session` cookie,
with sliding idle + absolute TTLs and a `data` JSONB blob. The columns
below are the **current (RS256-era)** shape — one row per issued JWT
(`jid` = the token `jti`) — and survive only until the cookie-session
migration tracked in the service spec §13.

**Files:**
[`src/models/sessions.rs`](../authentication-service-with-loco/src/models/sessions.rs),
migration
[`m20220101_000002_sessions.rs`](../authentication-service-with-loco/src/migration/m20220101_000002_sessions.rs).

| Field | Type | Description |
|---|---|---|
| id | i32 (pk auto) | |
| jid | String (unique) | = the token `jti` |
| user_pid | Uuid | Holder |
| expires_at | DateTime\<FixedOffset\> | = the token `exp` |
| revoked_at | Option\<DateTime\> | Set on signout |
| user_agent | Option\<String\> | Issuance context |

**Key methods:** `issue(db, jid, user_pid, expires_at, user_agent)`,
`find_by_jid`, `find_all_by_user_pid` (export), `is_active()` (=
`revoked_at.is_none()`), `revoke_all_for_user` (GDPR erasure),
`ActiveModel::revoke`.

## Claims (cross-crate contract)

Defined twice, byte-compatible by convention — service
[`src/auth/mod.rs`](../authentication-service-with-loco/src/auth/mod.rs)
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
| jti | String | UUID; correlates the originating session |

In the target model the same `Claims` shape is carried in a **PASETO
v4.public** token whose **footer** holds the `kid` (key id) selecting the
verifier's Ed25519 key. (Current RS256-era runtime puts `alg: RS256` +
`kid` = base64url(SHA-256(RSA public modulus)) in the JWT header until the
spec §13 follow-up lands.)

## AuthKeys / published keys (service)

`auth::AuthKeys` holds the signing/verification key(s), `kid`, `issuer`,
`audience`, `expiration`, and the pre-rendered published-key JSON. The
**target** publishes Ed25519 public key(s) at `/.well-known/paseto-keys`
for offline PASETO verification; the **current (RS256-era)** runtime still
serves an RSA JWKS at `/.well-known/jwks.json`:

```json
{ "keys": [ { "kty": "RSA", "use": "sig", "alg": "RS256",
              "kid": "…", "n": "…", "e": "…" } ] }
```

## Verifier (peer side)

`Verifier { keys: HashMap<kid, …>, validation }` — see
[`verification.md`](verification.md) for the full API and usage rules.

## View models

**Service** ([`src/views/auth.rs`](../authentication-service-with-loco/src/views/auth.rs)):
`LoginResponse { token, pid, name, email, is_verified }`,
`CurrentResponse { pid, name, email }`.

**Front-end** (`src/lib/api/types.ts`): mirrors of the two views. The
browser holds **no token** — the SvelteKit **BFF** holds the httpOnly
`__Host-mxi_session` cookie and calls the service server-side. There is
no `localStorage` bearer / `mxi_access_token`. See the front-end docs
(already harmonized) for the BFF specifics.
