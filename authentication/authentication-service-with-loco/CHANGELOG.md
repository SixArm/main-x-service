# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

## [0.1.1] - 2026-08-26

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

## [0.1.0] - 2026-08-04

### Fixed — email lookup was broken against Postgres (2026-08-01)

- **Every `LOWER(email)` lookup failed.** `find_by_email` and the
  duplicate-account guard in `create_passwordless` filtered with
  `Expr::cust_with_values("LOWER(email) = ?", …)`. sea-query emits that
  `?` **verbatim** — it is the `MySQL` placeholder where Postgres wants
  `$n` — so the driver sent `… WHERE LOWER(email) = ? LIMIT $1` and
  Postgres rejected the statement with `syntax error at or near "LIMIT"`.
  That is the **signup and magic-link sign-in path**: both returned a
  500. Replaced with sea-query's typed `LOWER()`
  (`Expr::expr(Func::lower(…)).eq(…)`), which renders the right
  placeholder for any backend.

  Nothing caught it because it cannot reproduce without a real Postgres,
  and this crate's DB-gated suite had never been run. It is now enrolled
  in [`ci/db-suites.txt`](../../ci/db-suites.txt), so CI runs it.

- **The seed fixture could not load.** `src/fixtures/users.yaml` was
  never updated when the ABAC `users.attributes` column landed. loco
  seeds by deserializing YAML straight into the entity, so an absent key
  is a hard failure (`missing field 'attributes'`) rather than a fallback
  to the column's `{}` default — every model test aborted before its
  first assertion. Both fixture users now carry `attributes: {}`, the
  shipped read-only posture.

- **Two request tests redeemed a hashed token.** They read
  `users.magic_link_token` back out of the database and presented it as
  the magic link, but SEC-A9 stores only `hash(token)` and redemption
  hashes what the caller presents — so the lookup could never match. The
  shared helper now issues a link through the production path
  (`create_magic_link`) and takes the plaintext from the model it
  returns, which is exactly where the mailer takes it from.

- **One request test asserted a decommissioned endpoint.**
  `jwks_endpoint_publishes_the_signing_key` still fetched
  `/.well-known/jwks.json` and asserted RSA/RS256 keys; the family moved
  to PASETO v4.public and the route is gone. Rewritten as
  `paseto_keys_endpoint_publishes_the_signing_key`, asserting the Ed25519
  key set at `/.well-known/paseto-keys` **and** that nothing serves a key
  set at the old path. That second assertion checks the response *body*,
  not its status: loco's default fallback middleware answers every
  unmatched route with `200`, so a status check would have passed whether
  or not the route existed.

### Security

- **SEC-A9: hash bearer-equivalent secrets at rest.** Three server-side
  secrets were stored in **plaintext**, so a read at rest (leaked backup,
  SQL-injection read, over-broad log) yielded a directly replayable
  credential: the magic-link token (`users.magic_link_token`), the opaque
  session id (`sessions.jid` — the value in the `__Host-mxi_session`
  cookie and the PASETO `sid` claim), and the per-session CSRF
  synchroniser token (`sessions.data.csrf`). The database now stores only
  a one-way **SHA-256 hash** (new `secret_hash` module); the plaintext
  lives only in transit (email link, cookie, header) and is never
  persisted.
  - A **fast, unsalted** hash is deliberate and correct here: these are
    high-entropy CSPRNG tokens, not passwords, so brute-force is
    infeasible regardless of hash speed, and a deterministic hash lets the
    server look a presented token up by its hash in one indexed query.
    Argon2 would be the *wrong* tool (salted ⇒ not lookup-able; its cost
    buys nothing against a high-entropy input).
  - `create_magic_link` persists `hash(plaintext)` but hands the caller a
    model carrying the plaintext (for the email/log); `consume_magic_token`
    / `find_by_magic_token` match on the hash of the presented token.
    `sessions::issue` / `find_by_jid` hash the session id; `session_data`
    stores the CSRF token's hash and `POST /token` compares the hash of
    the presented `X-CSRF-Token`.
  - Migration `m20220101_000009_hash_credentials_at_rest` enables
    `pgcrypto` and hashes existing rows **in place**
    (`encode(digest(x,'sha256'),'hex')` — the exact encoding the Rust
    helper produces), guarded on `length <> 64`, so live magic links and
    sessions keep working across the deploy.
  - Tests: `secret_hash` unit vectors (empty/`abc` FIPS values, determinism,
    lowercase-hex width); a DB-free `session_data` assertion that the CSRF
    token is stored hashed; and DB-gated assertions that
    `users.magic_link_token` and `sessions.jid`/`data.csrf` hold the hash,
    not the plaintext, while `find_by_jid` and magic-link redemption still
    resolve from the presented plaintext. (Repo tasks.md Phase 5 SEC-A9.)

- **SEC-A10: CSRF origin backstop on `POST /api/auth/token`.** The
  cookie-authenticated token exchange required a matching `X-CSRF-Token`
  only when the session carried a synchroniser token; a **legacy**
  session predating CSRF (no stored token) skipped the check entirely,
  and the `Origin` allow-list was only enforced when `AUTH_ALLOWED_ORIGINS`
  happened to be set — so a legacy session could bypass **both** the CSRF
  and the origin checks. Now the decision is a single pure
  `csrf_token_gate(is_production, origin_ok, session_csrf, provided_csrf)`:
  a token-carrying session must echo its token (constant-time compared);
  a legacy session must instead prove same-origin (an `Origin` on the
  allow-list) and is **refused in production** without that proof —
  development stays permissive. When the allow-list is unset in
  production the service warns once (`warn_missing_allowed_origins`) that
  the backstop is off. A `csrf_gate_matrix` unit test pins the full grid,
  including that a matching origin does not excuse a bad token and that the
  legacy-session bypass is closed in production. (Repo tasks.md Phase 5
  SEC-A10.)

- **SEC-A6: canonicalise the rate-limit bucket + case-consistent email
  identity.** Two related abuses: the per-email throttle keyed on the
  lowercased-but-otherwise-raw email, so `victim+1@gmail.com` /
  `v.ictim@gmail.com` / `Victim@…` were *different* buckets — an attacker
  could email-bomb one inbox around the cap; and `find_by_email` /
  `create_passwordless` compared the email **case-sensitively**, so
  `Victim@x` and `victim@x` spawned *duplicate* accounts. Now:
  - `rate_limit::normalize_key` folds aggressively (trim + lowercase, strip
    `+tag`, Gmail/`googlemail` dot-folding) so lookalikes of one inbox share
    a single throttle bucket. This only tightens the quota and does **not**
    decide identity.
  - `users::find_by_email` and `create_passwordless` are **case-insensitive**
    (`LOWER(email)` compare + emails stored normalised via
    `normalize_email`), so a case variant resolves to the same account and
    never inserts a duplicate. Case-only on purpose — `+tag`/dot folding is
    the throttle bucket's job, not account identity's.
  Pure `normalize_key` tests pin the plus/dot/case collapse (and that
  non-Gmail dots are preserved); a DB-gated request test pins that
  case-variant signups yield exactly one lowercased account.

- **SEC-A5: constant-work signup timing (defeat timing enumeration).**
  `create_passwordless` returns `EntityAlreadyExists` **before** its Argon2
  hash, so `signup` paid for the one deliberately-slow hash only on the
  **new-account** path — the already-registered path returned measurably
  faster, a timing oracle for account enumeration despite the identical
  always-`200` response. The existing-email branch now runs one equivalent
  Argon2 hash (`constant_work_hash`, discarded), so signup latency is
  indistinguishable between a new and an existing email. Unit-tested that
  the helper performs a real `$argon2` hash (and a fresh one per call).

- **SEC-A7: complete the GDPR erasure.** `DELETE /api/auth/account`
  tombstoned `users.email`/`name` and revoked sessions, but the subject's
  email survived in the audit trail (`auth_events.email`, including
  pre-account `unknown_email` rows) and in `sessions.user_agent`, and the
  terminal `account_erased` audit row was written *with* the email. Erasure
  now also `AuthEvent::scrub_subject_email` (NULLs the email on every audit
  row matched by pid OR normalised email) and
  `sessions::Model::scrub_user_agent_for_user`, and records `account_erased`
  with only the pid. `account_erasure_…` request test extended.
- **SEC-A8: revoke sessions on an attribute change (privilege-revocation
  latency).** A session snapshots the user's ABAC attributes at
  establishment and mints them into the PASETO, so removing `access=admin`
  did **not** downgrade a live session — it kept issuing admin tokens until
  its absolute TTL (up to 12 h). The admin API (`PUT …/attributes`) and the
  `user_attributes` CLI task now `revoke_all_for_user` after a change, so it
  takes effect on the next login (which copies the new attributes). Admin
  request test extended.
- **SEC-A4: atomic single-use magic-link consume.** Redemption was a
  `find_by_magic_token` (SELECT) followed by a separate `clear_magic_link`
  (UPDATE), so two concurrent requests with the same token both passed the
  read and each minted a session — the link was not single-use under
  concurrency. New `Model::consume_magic_token` does the clear-and-return in
  **one** `UPDATE users SET magic_link_token=NULL … WHERE magic_link_token=$1
  AND magic_link_expiration >= now() RETURNING *`, so exactly one concurrent
  redemption wins and the other gets `401`. DB-gated test
  `concurrent_magic_link_redemptions_only_one_wins`.
- **SEC-A2: admin-gate `GET /api/auth/audit/recent`.** The system-wide
  audit trail was unauthenticated and returned registered **emails** plus
  outcome markers (`created`/`existing`, `unknown_email`/`issued`), an
  account-enumeration oracle (trigger a signup, read back the outcome by
  timing) that undid the always-`200` anti-enumeration contract. It now
  requires a PASETO bearer with `access=admin` (`401` no token / `403`
  non-admin); a subject's own trail stays reachable via the gated
  `GET /api/auth/account/audit`. Supersedes the "left open" §12 decision.
  Unit test `recent_audit_requires_admin`; the DB-gated request test now
  pins the `401`.
- **SEC-A3: log the magic-link token/URL only in development.**
  `deliver_magic_link` wrote the full verify URL (embedding the live login
  token — a ~5-minute account-takeover primitive) at `info` in every
  environment. It is now emitted only when the loco environment is
  `Development` (where there is no SMTP and the console is authoritative);
  elsewhere the issuance is logged without the token. Pure gate
  `log_magic_link_url` + unit test `magic_link_url_logged_only_in_development`.

- **SEC-A1: refuse the built-in `DEV_SEED` signing key in production.**
  `load_seed()` fell back to the committed development Ed25519 seed
  whenever `TOKEN_PRIVATE_KEY_SEED` / `TOKEN_PRIVATE_KEY_FILE` were unset,
  with no environment guard — a production deploy that forgot the variable
  would sign PASETOs anyone could forge (e.g. `attrs.access = ["admin"]`),
  defeating the whole federation's auth. When the loco environment is
  `production` (`LOCO_ENV`/`RUST_ENV`), the dev-seed fallback is now
  refused: `load_keys()` errors and the service fails to boot
  (`keys()` panics with actionable guidance) instead of serving with a
  publicly-known key. Development/test still fall back to `DEV_SEED` so
  local runs stay offline. Pure `dev_seed_fallback` helper + unit test
  `dev_seed_fallback_refused_in_production`.

### Added

- **CSRF synchroniser token** on cookie-authenticated mutating requests
  (`agents/share/authentication-sessions.md` §4). A per-session random
  token is minted at session establishment (`verify`), stored server-side
  (`sessions.data.csrf`), and delivered in a readable `__Host-mxi_csrf`
  cookie (not `HttpOnly`). `POST /api/auth/token` now requires the client
  to echo it in the `X-CSRF-Token` header, **constant-time** compared
  against the session copy — a mismatch is `403` (distinct from the
  `401`s). Composes with the existing `Origin` allow-list backstop.
  Sessions predating CSRF have no token stored and skip the check
  (graceful). New `src/csrf.rs` (generate / cookie / constant-time
  compare) with DB-free tests; `signout` clears the CSRF cookie too.
- **Sessions-table reshape** (`authentication-sessions.md` §3). New
  migration `m20220101_000008_sessions_ttls` adds `last_seen_at` /
  `idle_expires_at` / `absolute_expires_at` (nullable — no backfill) plus
  the `sessions_active_user` partial index (`WHERE revoked_at IS NULL`).
  A session now has a sliding **idle** window (`AUTH_SESSION_IDLE_TTL_SECS`,
  default 30 min) and a hard **absolute** ceiling
  (`AUTH_SESSION_ABSOLUTE_TTL_SECS`, default 12 h). `Model::is_active`
  now enforces both (previously it checked only revocation, ignoring the
  boot-time `expires_at`); `Model::issue` sets the TTLs (independent of
  the ~5-min token exp); `Model::touch` slides the idle window on `/me`
  (capped at the absolute ceiling). Legacy rows (nullable bounds) stay
  valid-until-revoked. `sid` rotation already happens per magic-link
  login (a fresh `sid` each `verify`). Pure `is_active_at` boundary test.

- **ABAC attribute sourcing (`attrs` claim).** The sourcing side of the
  family's attribute-based access control, per
  [`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md)
  §6 (peers enforce via the shared `abac` engine in
  `authentication-verifier` 0.3; this supersedes any per-crate
  roles/RBAC sketch).
  - New `users.attributes` column (JSONB `NOT NULL DEFAULT '{}'`,
    migration `m20220101_000006_users_attributes`): the subject's
    string→strings attribute map (e.g. `{"access": ["write"]}`). `{}`
    until an operator assigns attributes — read-only under the family's
    default policy. The assignment surface is the new `user_attributes`
    CLI task (below).
  - New `sessions.data` column (JSONB `NOT NULL DEFAULT '{}'`,
    migration `m20220101_000007_sessions_data` — the first slice of the
    shared-§3 sessions reshape): magic-link redemption **copies** the
    user's attributes into the session (`data.attrs`,
    `sessions::session_data`), so `POST /api/auth/token` mints from the
    session alone.
  - `auth::Claims` gains `attrs: BTreeMap<String, Vec<String>>` with
    `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]` —
    byte-identical to `authentication_verifier::Claims` 0.3. An empty
    map is omitted from the wire (old/attribute-less tokens keep the
    pre-ABAC payload shape); `sign_access_token` takes the map and both
    minting paths (redemption + `POST /token`) supply it. Parsing of
    the stored JSONB is tolerant (`users::attributes_map`): malformed
    entries are inert and can never fail minting.
  - The GDPR right-of-access export now includes `attributes`
    (subject data, not a secret) on `AccountUserExport`. OpenAPI
    documents the `attrs` claim and the export field, and marks
    `scope`/`roles` as deprecated for authorization.
  - Tests: DB-free units (tolerant `attributes_map` parsing; the
    §6 copy round-trip `users.attributes` → `session_data` →
    `sessions::Model::attrs`; claim serialization omits an empty map;
    the OpenAPI `attrs` schema) plus two new cross-crate contract tests
    (a non-empty `attrs` map round-trips service-mint → peer-verify; an
    empty map is absent from the wire payload and still verifies to an
    empty map). Users model snapshots updated for the new column.
- **ABAC attribute assignment — `user_attributes` CLI task.** The
  operator surface for assigning `users.attributes`
  (authorization-attributes.md §6; spec §13). New
  `src/tasks/attributes.rs`, registered in `App::register_tasks`:

  ```text
  cargo loco task user_attributes email:alice@example.com            # show
  cargo loco task user_attributes op:set email:alice@example.com key:access values:write
  cargo loco task user_attributes op:set email:peer@svc      key:svc    values:true
  cargo loco task user_attributes op:unset email:alice@example.com key:dept
  cargo loco task user_attributes op:clear email:alice@example.com
  ```

  The target user is selected by `email:` or `pid:`; `op` is
  `show` (default) / `set` / `unset` / `clear`. `set` replaces a key's
  value list; keys and values are validated as short lowercase tokens
  and the reserved pseudo-attributes `sub`/`email`/`entity` are
  refused. Writes go through the new
  `users::ActiveModel::set_attributes` (canonicalised by the new
  `users::attributes_to_value`, the lossless inverse of
  `attributes_map`); the task prints a before/after report. DB-free unit
  tests cover value parsing, key / value validation, command parsing,
  and the map-mutation ops, plus the `attributes_to_value` ↔
  `attributes_map` round-trip.
- **ABAC attribute assignment — HTTP admin API.** `GET` / `PUT
  /api/auth/admin/users/{pid}/attributes` (`src/controllers/admin.rs`,
  mounted in `App::routes`): show / replace a user's `users.attributes`
  from an authenticated caller whose own attributes include
  `access=admin` (the bootstrap admin is assigned via the CLI task).
  `401` no/invalid token, `403` valid non-admin, `404` unknown/erased
  user, `422` on an invalid body (keys/values validated with the CLI
  task's `validate_key` / `validate_value` — reserved `sub`/`email`/
  `entity` refused, no empty value lists). OpenAPI documents both verbs
  (`UserAttributes` / `ReplaceUserAttributes` schemas, `admin` tag).
  DB-free tests (`require_admin`, `validate_map`, OpenAPI assertions)
  plus DB-gated request tests (`tests/requests/admin.rs`).
- **Per-assignment audit rows.** Both assignment surfaces now write an
  **`attributes_assigned`** `auth_events` row
  (`Model::record_attribute_assignment_best_effort`): subject = the
  target user, `detail` carries the op, the affected key, and the actor
  (`cli` or the admin's `pid:<uuid>`). Attribute **values** are omitted
  from the audit detail (a value can itself be sensitive); the value set
  lives in `users.attributes`.

- **Attribute vocabulary (typo guard).** Optional per-deployment
  allow-set of attribute keys → values, configured via
  `AUTH_ATTRIBUTE_VOCABULARY` (inline JSON) or
  `AUTH_ATTRIBUTE_VOCABULARY_FILE` (path), e.g.
  `{ "access": ["read","write","admin"], "dept": ["cardiology"], "svc": [] }`
  (an empty value list ⇒ any value for that key).
  `tasks::attributes::AttributeVocabulary` + the cached `vocabulary()`
  are enforced by **both** assignment surfaces — the `user_attributes`
  CLI task (on `set`) and the admin `PUT` (`validate_map`) — so a typo
  like `dept=cardiolgy` or an unknown key is rejected instead of
  silently granting nothing. Unset/unparsable ⇒ unrestricted
  (warn-logged on a parse error) — assignment always works. DB-free
  tests for the vocabulary checks.

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
