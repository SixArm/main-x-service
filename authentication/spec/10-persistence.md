## 10. Persistence

### 10.1 Service — PostgreSQL via sqlx

sqlx, no ORM (2026-11-08): every statement is a compile-checked
`query!` / `query_as!` / `query_scalar!` literal in
[`src/models/`](../authentication-service-with-axum/src/models/), and the
schema is plain SQL under
[`migrations/`](../authentication-service-with-axum/migrations/), run by
`sqlx::migrate!` — `2022010100000001_users`, `…02_sessions`,
`…03_auth_events`, and the later column migrations through
`2026090600000002_auth_events_source_ip` (the service spec §10 lists
them all). The cookie-session reshape landed as added columns on the
existing `sessions` table rather than a new one (see the pivot note).
`auto_migrate` is on in development, off in production.

> **Pivot (2026-06-17), as shipped.** The cookie session took over the
> original `sessions` table (one row per issued JWT, keyed by `jid`)
> rather than replacing it: the shared
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> §3 shape — a `sid` text primary key — never shipped, and a rename to
> it was judged lower-value and is not planned. The opaque session id is
> the `jid` column (`sid` = `jid`), and the §3 payload and TTL columns
> were added to the existing table (`…07_sessions_data`,
> `…08_sessions_ttls`). The tables below are the schema the migrations
> build. Every table also carries `created_at` / `updated_at`
> (`timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP`).

**`users`** (from
[`2022010100000001_users.up.sql`](../authentication-service-with-axum/migrations/2022010100000001_users.up.sql),
plus `…04_users_deleted_at` and `…06_users_attributes`):

| Column | Type | Notes |
|---|---|---|
| `id` | bigint identity, pk | Internal id |
| `pid` | uuid | Public id; the token `sub` |
| `email` | varchar, unique | Sign-in identity; personal data (§12); a tombstone after erasure |
| `password` | varchar | Unusable random hash — no password flow |
| `api_key` | varchar, unique | loco-starter convention (`lo-{uuid}`) |
| `name` | varchar | Display name; `deleted user` after erasure |
| `reset_token` / `reset_sent_at` | nullable | Legacy loco-starter (unused) |
| `email_verification_token` / `email_verification_sent_at` | nullable | Legacy loco-starter (unused) |
| `email_verified_at` | timestamptz, nullable | Set on first redemption |
| `magic_link_token` | varchar, nullable | SHA-256 hex of the live link token, never the token (SEC-A9, `…09_hash_credentials_at_rest`) |
| `magic_link_expiration` | timestamptz, nullable | Now + 5 min |
| `deleted_at` | timestamptz, nullable | GDPR Art. 17 erasure; set ⇒ every read path treats the user as gone |
| `attributes` | jsonb, `NOT NULL DEFAULT '{}'` | ABAC subject attributes (string → strings) |

**`sessions`** (from
[`2022010100000002_sessions.up.sql`](../authentication-service-with-axum/migrations/2022010100000002_sessions.up.sql),
plus `…07_sessions_data`, `…08_sessions_ttls` and
`2026090600000001_sessions_source_ip`) — the unit of the human login
and of revocation:

| Column | Type | Notes |
|---|---|---|
| `id` | bigint identity, pk | Internal id |
| `jid` | varchar, unique | The opaque session id (the shared design's `sid`, **not** a JWT); stored as its SHA-256 hex (SEC-A9). The `__Host-mxi_session` cookie and the PASETO `sid` claim carry the plaintext |
| `user_pid` | uuid | Holder |
| `expires_at` | timestamptz | The original expiry column, kept |
| `revoked_at` | timestamptz, nullable | Set on logout / admin revoke / erasure |
| `user_agent` | varchar, nullable | Captured at issuance; scrubbed on erasure (SEC-A7) |
| `data` | jsonb, `NOT NULL DEFAULT '{}'` | Session payload: the copied ABAC attributes under `attrs`, the CSRF token's hash under `csrf` |
| `last_seen_at` | timestamptz, nullable | Sliding idle marker, bumped on use |
| `idle_expires_at` | timestamptz, nullable | `now() + idle TTL` (30 min default), bumped on use |
| `absolute_expires_at` | timestamptz, nullable | Hard ceiling (12 h default), never extended |
| `source_ip` | varchar, nullable | Connecting address at issuance (T-14), emptied by the retention sweep |

The three TTL columns are nullable so the reshape migrated a populated
table without a backfill; new sessions set all three, and a NULL bound
reads as unbounded. Partial index `sessions_active_user ON (user_pid)
WHERE revoked_at IS NULL`. Valid iff `revoked_at IS NULL AND now() <
idle_expires_at AND now() < absolute_expires_at`.

**`auth_events`** (from
[`2022010100000003_auth_events.up.sql`](../authentication-service-with-axum/migrations/2022010100000003_auth_events.up.sql),
plus `2026072800000001_add_auth_event_mac` and
`2026090600000002_auth_events_source_ip`) — the durable authentication
audit trail (T-10, §12):

| Column | Type | Notes |
|---|---|---|
| `id` | bigint identity, pk | Monotonic; newest = largest |
| `event` | varchar | `signup` / `magic_link_requested` / `magic_link_redeemed` / `signout` / `account_erased` / `attributes_assigned` / … |
| `email` | varchar, nullable | Normalised (trimmed, lowercased) subject email where applicable; NULLed on erasure (SEC-A7) |
| `user_pid` | uuid, nullable | Subject pid when known |
| `detail` | varchar, nullable | Outcome marker (`rate_limited` / `unknown_email` / `ok` / …) — never a token or secret |
| `hash` / `hash_sha3` / `mac` | varchar, nullable | SHA-256 and SHA3-256 digests and the keyed HMAC over the row (service spec §6.13) |
| `source_ip` | varchar, nullable | Client address (T-14); outside the digest pre-image |

Writes are best-effort and never fail the request; the row may
distinguish outcomes the HTTP response deliberately hides
(anti-enumeration).

**`auth_rate_limits`** (from
[`2022010100000005_auth_rate_limits.up.sql`](../authentication-service-with-axum/migrations/2022010100000005_auth_rate_limits.up.sql))
— the magic-link issuance limiter's window log, one row per allowed
request:

| Column | Type | Notes |
|---|---|---|
| `id` | bigint identity, pk | Internal id |
| `email_key` | varchar | Normalised email |
| `requested_at` | timestamptz | When the request was allowed |

Index `idx_auth_rate_limits_key_requested_at ON (email_key,
requested_at)`. `db truncate` clears it with the other three tables.

The Postgres-backed axa worker queue (`bg_pg`) shares the same
database — family convention, no external broker.

### 10.2 Key storage

Not in the database. The **Ed25519 keypair** (PASETO v4.public signing
+ verify keys) comes from env / files (§8.4); the dev keypair is
committed at
[`config/keys/`](../authentication-service-with-axum/config/keys/). The
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
