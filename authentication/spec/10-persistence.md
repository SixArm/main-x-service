## 10. Persistence

### 10.1 Service — PostgreSQL via SeaORM

Migrations (`sea-orm-migration`, registered in
[`src/migration/mod.rs`](../authentication-service-with-loco/src/migration/mod.rs)):
`m20220101_000001_users`, `m20220101_000002_sessions` (replaced by the
cookie-session schema below — see the pivot note), `m20220101_000003_auth_events`,
and the pivot's new `m2026..._sessions_cookie` (the
`sid`/`data`/idle+absolute-TTL schema). `auto_migrate` is on in
development, off in production.

> **Pivot (2026-06-17).** The old `sessions` table (one row per issued
> JWT, keyed by `jid` = `jti`) is replaced by the server-side
> cookie-session table below, per
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> §3. The migration is a §13 T-12 follow-up.

**`users`** (from
[`m20220101_000001_users.rs`](../authentication-service-with-loco/src/migration/m20220101_000001_users.rs)):

| Column | Type | Notes |
|---|---|---|
| `id` | pk auto | Internal id |
| `pid` | uuid | Public id; the token `sub` |
| `email` | string, unique | Sign-in identity; personal data (§12) |
| `password` | string | Unusable random hash — no password flow |
| `api_key` | string, unique | loco-starter convention (`lo-{uuid}`) |
| `name` | string | Display name |
| `reset_token` / `reset_sent_at` | nullable | Legacy loco-starter (unused) |
| `email_verification_token` / `email_verification_sent_at` | nullable | Legacy loco-starter (unused) |
| `email_verified_at` | timestamptz, nullable | Set on first redemption |
| `magic_link_token` | string, nullable | Live link token (32 chars) |
| `magic_link_expiration` | timestamptz, nullable | Now + 5 min |

**`sessions`** (cookie-session schema, per shared §3 — the unit of the
human login and of revocation):

| Column | Type | Notes |
|---|---|---|
| `sid` | text, pk | Opaque, high-entropy session id (**not** a JWT) |
| `user_pid` | uuid | Holder |
| `data` | jsonb, `'{}'` | Roles / scopes / MFA state / … |
| `created_at` | timestamptz | Login time |
| `last_seen_at` | timestamptz | Sliding idle marker, bumped on use |
| `idle_expires_at` | timestamptz | `now() + idle TTL`, bumped on use |
| `absolute_expires_at` | timestamptz | Hard ceiling, never extended |
| `revoked_at` | timestamptz, nullable | Set on logout / admin revoke |

Partial index `sessions_user ON (user_pid) WHERE revoked_at IS NULL`.
Valid iff `revoked_at IS NULL AND now() < idle_expires_at AND now() < absolute_expires_at`.

**`auth_events`** (from
[`m20220101_000003_auth_events.rs`](../authentication-service-with-loco/src/migration/m20220101_000003_auth_events.rs)) —
the durable authentication audit trail (T-10, §12):

| Column | Type | Notes |
|---|---|---|
| `id` | pk auto | Monotonic; newest = largest |
| `event` | string | `signup` / `magic_link_requested` / `magic_link_redeemed` / `signout` (`me` reserved) |
| `email` | string, nullable | Normalised (trimmed, lowercased) subject email where applicable |
| `user_pid` | uuid, nullable | Subject pid when known |
| `detail` | string, nullable | Outcome marker (`rate_limited` / `unknown_email` / `invalid_or_expired` / `issued` / `created` / `existing` / `ok` / `rejected`) — never a token or secret |
| `created_at` | timestamptz | Event time |

Writes are best-effort and never fail the request; the row may
distinguish outcomes the HTTP response deliberately hides
(anti-enumeration).

The Postgres-backed loco worker queue (`bg_pg`) shares the same
database — family convention, no external broker.

### 10.2 Key storage

Not in the database. The **Ed25519 keypair** (PASETO v4.public signing
+ verify keys) comes from env / files (§8.4); the dev keypair is
committed at
[`config/keys/`](../authentication-service-with-loco/config/keys/). The
private key signs `POST /token`; the public key(s) are published at
`/.well-known/paseto-keys`. *(Replaces the decommissioned RSA keypair.)*

### 10.3 Verifier

Stateless — no persistence. The cached Ed25519 key set lives in process
memory for the process lifetime.

### 10.4 Front-end

No browser credential storage. The browser holds only the httpOnly
`__Host-mxi_session` cookie; the SvelteKit-server BFF holds the session
server-side and mints PASETO tokens for outbound calls (shared §6). The
prior `localStorage` keys (`mxi.auth.token`, `mxi.auth.user`) are
removed.
