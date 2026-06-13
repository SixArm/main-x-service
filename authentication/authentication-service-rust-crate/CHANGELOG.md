# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **GDPR subject-rights workflow** (entity spec T-9). Three bearer-gated
  account endpoints on the `auth` controller:
  - `GET /api/auth/account/export` — **right of access** (Art. 15):
    a JSON document of everything the service holds about the
    authenticated subject — their `users` row, their `sessions`
    (issuance/expiry/revocation + user agent), and their `auth_events`
    audit trail (matched by pid *or* email). Excludes the password hash,
    api key, and any token / key material (`views/auth::AccountExport`).
  - `DELETE /api/auth/account` — **right to erasure** (Art. 17):
    soft-delete + anonymise. New `users.deleted_at` column (migration
    `m20220101_000004_users_deleted_at`); `email`→`deleted+<pid>@invalid`
    tombstone (keeps `UNIQUE(email)`, RFC 2606 unroutable),
    `name`→`"deleted user"`; **all** the subject's sessions revoked; an
    `account_erased` audit row written. The row survives so referential
    history + the audit trail keep integrity. Post-erasure `/me` and the
    export treat the subject as gone (`401` via
    `users::find_active_by_pid`), though the issued bearer token still
    verifies cryptographically until `exp`. Idempotent.
  - `GET /api/auth/account/audit` — the subject's own audit trail
    (bearer-gated, per-subject counterpart to the open system-wide
    `/api/auth/audit/recent`, which stays open by decision — see spec
    §12). OpenAPI documents all three endpoints + the `AccountExport` /
    `AccountUserExport` / `AccountSessionExport` / `AccountAuditExport`
    schemas + bearer security. Un-gated unit tests (tombstone transform,
    export assembly + secret-exclusion, OpenAPI `spec()`) plus DB-gated
    request tests (export, erasure, post-erasure `401`, unauthenticated
    `401`).
- **Rate-limited magic-link issuance** (`src/rate_limit.rs`): a per-email
  (normalised: trimmed + lowercased) monotonic-clock sliding-window
  limiter — at most `MAX_REQUESTS` (5) requests per `WINDOW` (5 min).
  Wired into `POST /api/auth/signup` + `POST /api/auth/magic-link` before
  any account lookup; over the cap returns `429 Too Many Requests`
  (`{"error":"rate_limited",…}`) and issues no token / sends no mail,
  while keeping the always-`200` anti-enumeration shape. Un-gated unit
  tests (clock-injectable `check_at`, `reset()` helper) plus a DB-gated
  request test. *(entity spec T-6)*
- **OpenAPI 3 + Swagger UI**: hand-written `src/openapi.rs` (OpenAPI
  3.0.3, no `utoipa`) served by `src/controllers/docs.rs` at
  `GET /api-docs/openapi.json` + `GET /swagger-ui`. Documents all six
  endpoints, the request/response + `Claims`/`Jwks` schemas, the `429`
  rate-limit responses, and a bearer `securityScheme` on `me`/`signout`.
  Un-gated `spec()` unit tests. *(entity spec T-8)*
- **Cross-crate sign→verify contract test**
  (`tests/sign_verify_contract.rs`): signs with this crate's `auth`
  module and verifies through the sibling
  [`authentication-verifier`](../authentication-verifier-rust-crate)
  crate (new dev-dependency), pinning the duplicated-by-convention
  `Claims` shape and the `kid = base64url(SHA-256(modulus))`
  derivation. DB-free; runs in every `cargo test`.
- **Magic-link request tests**: `tests/requests/auth.rs` now covers
  signup / magic-link / redeem (single-use, anti-enumeration) / me /
  signout / JWKS with direct assertions. Postgres-backed tests are
  `#[ignore]`d (run with `cargo test -- --ignored`) so plain
  `cargo test` stays green; DB-free route-table and params-contract
  tests always run.

### Removed

- The starter's password-flow request tests and their insta snapshots
  (`register`/`login`/`forgot`/`reset`/`verify` endpoints no longer
  exist).

### Added (inaugural)

- **Inaugural scaffold (v0.1.0).** The Main X Index family's central
  single sign-on provider and reference loco.rs application.
  - Real loco.rs 0.16 app generated via `loco new` (Postgres,
    Postgres-backed queue, no asset tier).
  - **Passwordless magic-link** flow: `POST /api/auth/signup`,
    `POST /api/auth/magic-link`, `GET /api/auth/magic-link/{token}`,
    `GET /api/auth/me`, `POST /api/auth/signout`.
  - **RS256 JWT** issuance with a self-contained `src/auth` module
    (`jsonwebtoken` + `rsa`), and a **JWKS** endpoint at
    `/.well-known/jwks.json` so peer services verify tokens offline —
    no shared secret, no introspection hop.
  - **sessions** table (`jid` = JWT `jti`) for real signout/revocation,
    honoured locally by `/me` and `/signout`.
  - Console magic-link delivery in development (SMTP disabled); env-based
    RSA key configuration with a committed dev keypair under
    `config/keys/`.
  - DB-free unit tests covering the sign/verify roundtrip, JWKS shape,
    and rejection of tampered/garbage tokens. Green `cargo build`,
    clippy clean.

### Notes

- The Postgres-backed model tests (`tests/models/users.rs`) are
  `#[ignore]`d so `cargo test` stays green without a database; several
  still exercise password-era model helpers that survive only to
  satisfy the schema.
