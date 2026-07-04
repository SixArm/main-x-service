# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed

- **`cargo fmt` drift.** Reformatted `src/auth/mod.rs`,
  `src/controllers/auth.rs`, `src/controllers/mod.rs`, `src/cookie.rs`,
  and `tests/sign_verify_contract.rs` so `cargo fmt --check` passes
  again (no behavioural change).

### Changed

- **Auth model pivot: cookie sessions + PASETO replace RS256 JWT + JWKS
  (spec-level; code follow-up pending).** Per the new family-wide design
  in [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  the **session** becomes a server-side **Postgres-backed cookie session**
  (opaque id in an httpOnly `__Host-mxi_session` cookie; no token in the
  browser), and cross-service authentication becomes a short-lived
  **PASETO v4.public** (Ed25519) token minted from the session and
  verified **offline** against the published key(s) at
  `/.well-known/paseto-keys` (replacing `/.well-known/jwks.json`). The
  passwordless magic-link *mechanism* is unchanged; only its **outcome**
  changes — verifying the link now establishes a session + sets the
  cookie instead of returning a JWT. Front-ends adopt a **BFF** (the
  SvelteKit server holds the session and mints/forwards the PASETO;
  browser holds no token, no `localStorage`/`mxi_access_token`). Blanket
  `/api/*` enforcement now checks a valid session (BFF/browser) or a
  valid PASETO (service-to-service). **RS256 JWT + JWKS are
  decommissioned.**

  **Token core implemented.** `src/auth` now mints and verifies **PASETO
  v4.public** (Ed25519, `rusty_paseto` + `ed25519-dalek`); `Claims` is
  aligned to the peer verifier (`sid` replaces `jti`; adds `nbf` /
  `scope` / `roles`); the public key set is served at
  `/.well-known/paseto-keys` (controller renamed `jwks` → `paseto_keys`);
  and magic-link redemption mints a PASETO bound to a session row keyed by
  `sid`. The cross-crate contract test verifies a service-minted PASETO
  through the (path-linked) `authentication-verifier` 0.2; lib 35/35 +
  contract 5/5 green, clippy `-D warnings` clean.

  **Cookie sessions + token exchange implemented.** A new `src/cookie`
  module builds/clears/parses the httpOnly, `Secure`, host-locked
  `__Host-mxi_session` cookie (4 DB-free unit tests). Magic-link redemption
  now **sets** that cookie (carrying the opaque session id); `POST
  /api/auth/token` exchanges a valid session cookie for a fresh short-lived
  PASETO (the BFF path — with an `AUTH_ALLOWED_ORIGINS` CSRF backstop);
  signout **clears** it. This reuses the existing `sessions` table
  (`sid` = the `jid` column), so no migration was needed. Magic-link
  redemption still *also* returns the PASETO in the body transitionally
  until every front-end adopts the BFF. lib 39/39 + contract 5/5, clippy
  clean. **Deferred refinements (DB-gated, not blocking the BFF):** the
  sessions-table reshape to the design columns (`data` JSONB, idle/absolute
  TTLs) and full double-submit CSRF (an `Origin` allow-list is in place);
  the new cookie/`token` request flows are compile-checked here (runtime
  needs Postgres).

  **Per-app magic-link return URL (per-app SSO).** `signup` /
  `request_magic_link` now accept an optional `return_url`; the magic-link
  email lands on THAT front-end's `/verify` when `return_url` exactly
  matches the `AUTH_ALLOWED_FRONTENDS` allow-list (else the default
  `FRONTEND_URL`) — no open redirect. This lets each operator front-end run
  the BFF login flow against its own origin (the chosen inter-app SSO
  model). The `choose_frontend` decision is pure + unit-tested (allow-listed
  honoured; non-listed/empty/absent → default). lib 41/41 + contract 5/5,
  clippy clean.

- **Doc/test harmonization pass for i18n.** Reconciled the spec ↔ code ↔
  tests for the localized magic-link email: the previously-claimed
  "DB-free mailer-localization unit test" now genuinely exists as
  `selected_locale_renders_the_mailer_email_copy` (pins the exact
  `select_locale → magic_link_email → render` bridge the mailer uses,
  including the `{frontend}/verify?token={token}` link substitution and
  the English fallback). Updated spec §6.11 / §11 / §13, the
  `controllers/auth.rs` module-doc request bodies (`{email, …, locale?}`),
  and the `rate_limit` module-doc summary (Postgres-backed, not
  in-memory). No behaviour change.

- **Magic-link rate limiter is now Postgres-backed.** The per-email
  sliding-window limiter (`src/rate_limit.rs`, `MAX_REQUESTS` = 5 /
  `WINDOW` = 5 min) moved from a process-local in-memory `OnceLock` map to
  the new `auth_rate_limits` table (migration
  `m20220101_000005_auth_rate_limits`). Each check runs in one transaction
  under a per-key advisory lock (`pg_advisory_xact_lock(hashtext(key))`) —
  prune aged-out rows, count, insert iff under the cap — so the quota is
  **exact and shared across horizontally-scaled instances**. The window is
  now wall-clock (`TIMESTAMPTZ`); `check`/`check_at`/`reset` are async and
  take the DB connection. A DB error fails open (logged WARN). Behaviour
  (cap, window, anti-enumeration always-`200` shape, `429` on breach) is
  unchanged. Sliding-window tests moved to DB-gated
  `tests/requests/rate_limit.rs`; the pure key-normalisation unit test
  stays DB-free.

### Added

- **Localized magic-link email (English + Welsh).** New dependency-light
  `src/i18n.rs` catalog renders the magic-link email subject + text + HTML
  bodies in **English (`en`)** and **Welsh (`cy`)** — a pure-Rust lookup,
  no templating engine and no DB. `SignupParams` / `MagicLinkParams` gain
  an optional `locale` field; both issuance handlers call
  `i18n::select_locale(params.locale)` and render via
  `Emailer::send_magic_link(ctx, user, locale)`. Unknown/absent locales
  fall back to English; a region subtag reduces to its primary language
  (`cy-GB` → `cy`); the only wired selection input is the request-body
  field (no `Accept-Language` parsing). Locale changes only the email
  language — the always-`200`, identical-shape anti-enumeration response
  is unchanged across locales. Compliance basis: the Welsh Language
  (Wales) Measure 2011 (spec §12). OpenAPI documents the `locale` field;
  un-gated i18n + params unit tests, a DB-free mailer-bridge render test
  (`selected_locale_renders_the_mailer_email_copy`, pinning the exact
  `select_locale → magic_link_email → render` expression
  `Emailer::send_magic_link` evaluates, link substitution + English
  fallback included), plus a DB-gated anti-enumeration request test
  (`signup_locale_does_not_change_the_response_shape`). *(spec §6.11)*
- **Prometheus metrics endpoint.** `GET /metrics.prom` (root path, no
  `/api` prefix, unauthenticated) renders a process-wide
  `prometheus::Registry` in text-exposition format (`Content-Type:
  text/plain; version=0.0.4`), for parity with the older Axum services
  in the family. The metric set is auth-specific (this service has no
  entity CRUD): `auth_signup_total`, `auth_magic_link_issued_total`,
  `auth_magic_link_redeemed_total`, `auth_signout_total`,
  `auth_rate_limited_total` counters plus a `http_requests_total`
  counter vec (`method` / `path` / `status`). The auth controllers
  increment them on signup success, magic-link issuance (signup +
  sign-in), redeem success, signout, and the `429` rate-limited path.
  Labels never carry a subject identifier (no email/token/pid), so the
  monitoring system holds no personal data; a DB-free unit test pins
  both the valid-exposition shape and the no-secret-labels contract.
  New module `src/metrics.rs` + controller `src/controllers/metrics.rs`;
  registered in `src/app.rs`; documented in OpenAPI. Adds the
  `prometheus = "0.13"` dependency.
- **Zero-downtime key rotation** (entity spec T-5). `auth::AuthKeys` is
  now a **key set**: one *primary* signing key plus zero or more
  *additional* verify-only public keys.
  - New config: `JWT_ADDITIONAL_PUBLIC_KEY_FILES` (comma-separated file
    paths) and `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` (inline PEM blocks,
    comma- or newline-separated) load the additional keys. Unset/empty ⇒
    a single-key set, byte-for-byte the previous behaviour (same `kid`).
  - `sign_access_token` signs with the primary and stamps its `kid`
    (unchanged for the common case); `verify_token` selects the verifying
    key by the token header `kid` from {primary} ∪ {additional}, so a
    token signed by a key that has since rotated down to "additional"
    still verifies locally until it expires; an unknown `kid` is rejected.
  - `/.well-known/jwks.json` now publishes **all** keys in the set
    (primary first). `kid` stays `base64url(SHA-256(modulus))` for every
    key. OpenAPI updated to note the JWKS may publish multiple keys.
  - A `load_from(...)` constructor builds a deterministic multi-key set
    from explicit PEMs (no env mutation), used by the un-gated unit
    tests. The cross-crate `tests/sign_verify_contract.rs` gains a
    multi-key case (a verifier built from the full set verifies a
    primary-signed token and rejects an unknown `kid`).
  - Operator runbook: `config/keys/README.md` + spec §8.4. No
    auto-rotation scheduler (planned follow-up).
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
- The generated auth mailer keeps unwired password-era scaffolding
  (`Emailer::send_welcome` / `forgot_password`, the `welcome` / `forgot`
  template dirs, `users::set_forgot_password_sent`). This is an
  intentional retain-as-loco-scaffolding decision, now recorded in spec
  §5 with a removal task in §13; the live magic-link path renders from
  the `src/i18n.rs` catalog, not these templates.
