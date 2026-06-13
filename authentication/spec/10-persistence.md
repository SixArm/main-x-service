## 10. Persistence

### 10.1 Service — PostgreSQL via SeaORM

Migrations (`sea-orm-migration`, registered in
[`src/migration/mod.rs`](../authentication-service-rust-crate/src/migration/mod.rs)):
`m20220101_000001_users`, `m20220101_000002_sessions`,
`m20220101_000003_auth_events`. `auto_migrate` is on in development,
off in production.

**`users`** (from
[`m20220101_000001_users.rs`](../authentication-service-rust-crate/src/migration/m20220101_000001_users.rs)):

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

**`sessions`** (from
[`m20220101_000002_sessions.rs`](../authentication-service-rust-crate/src/migration/m20220101_000002_sessions.rs)):

| Column | Type | Notes |
|---|---|---|
| `id` | pk auto | |
| `jid` | string, unique | = token `jti` |
| `user_pid` | uuid | Holder |
| `expires_at` | timestamptz | = token `exp` |
| `revoked_at` | timestamptz, nullable | Set on signout |
| `user_agent` | string, nullable | Issuance context |

**`auth_events`** (from
[`m20220101_000003_auth_events.rs`](../authentication-service-rust-crate/src/migration/m20220101_000003_auth_events.rs)) —
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

Not in the database. PEM key material comes from env / files (§8.4);
the dev keypair is committed at
[`config/keys/`](../authentication-service-rust-crate/config/keys/).

### 10.3 Verifier

Stateless — no persistence. Key cache lives in process memory for the
process lifetime.

### 10.4 Front-end

Browser `localStorage` only: `mxi.auth.token`, `mxi.auth.user`. No
server-side persistence.
