# Authentication Service — Specification

> **Single source of truth.** Code conforms to this spec, not the other
> way around. A behavioural change is a three-part PR: spec edit + code
> edit + test edit. Live work queue is §13; open questions are §16.
>
> Sibling front-end:
> [authentication-front-end-with-svelte](../../authentication-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

The Authentication Service is the **central, single sign-on provider**
for the Main X Index family. It authenticates human operators with
**passwordless email magic links**. The human session is a server-side
Postgres **cookie session** (an opaque `sid` in an httpOnly
`__Host-mxi_session` cookie — no token in browser JS); cross-service
calls exchange that session for a short-lived **PASETO v4.public**
(Ed25519) token that every other service verifies **offline** against a
published Ed25519 key set. There is one place to sign in, one place to
revoke, and one set of keys to trust.

> **Pivot (2026-06-17).** This supersedes the previous **RS256 JWT +
> JWKS** model, per
> [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
> (principle: [`agents/share/jwt.md`](../../../agents/share/jwt.md) —
> "JWTs must not be used to keep users logged in"). RS256 signing and
> `/.well-known/jwks.json` are **decommissioned** (§9, §13 T-12, §15).
> The entity umbrella spec
> ([`../../spec/index.md`](../../spec/index.md)) carries the full
> contract; this file mirrors it.

It is also the family's **reference loco.rs application**. The existing
service crates declare `loco-rs` but run hand-rolled Axum; they will be
converted to idiomatic loco using this crate as the template.

## 2. Scope

In scope: sign up, sign in, sign out — all via magic link; server-side
cookie sessions; PASETO v4.public minting (`POST /token`); Ed25519
public-key publication (`/.well-known/paseto-keys`); CSRF; session
revocation; the user record; **ABAC attribute sourcing** — the
`users.attributes` map, copied into the session at establishment and
minted into the token's `attrs` claim (shared
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md)
§6).

Out of scope (for now): passwords, OAuth/social login, multi-factor,
authorization *enforcement* (peer services evaluate ABAC policies
locally over the verified `attrs` claim — this service only *sources*
the attributes; the earlier "roles/permissions" wording is superseded
by ABAC), organization/tenant modelling, account self-service beyond
sign-in.

## 3. Stakeholders and users

- **Operators** — humans who sign into any Main X front-end.
- **Peer services** — person/worker/place/thing/event/course services
  that accept this service's tokens.
- **Front-end** — `authentication-front-end-with-svelte`.

## 4. Glossary

- **Magic link** — a one-time, short-lived URL containing an opaque
  token that signs a user in without a password. Redemption now
  establishes a session + cookie (not a token).
- **Session** — server-side `sessions` row keyed by an opaque `sid`,
  carried to the browser only via the `__Host-mxi_session` httpOnly
  cookie. The unit of revocation. Not a JWT.
- **PASETO v4.public** — the short-lived (~5 min) Ed25519-signed
  cross-service token minted by `POST /token`; replaces the RS256 JWT.
- **paseto-keys** — the Ed25519 public key set at
  `/.well-known/paseto-keys` (the JWKS analog) used to verify tokens
  offline, keyed by footer `kid`.
- **CSRF token** — per-session token required on cookie-authenticated
  mutating requests (shared §4).
- **pid** — a user's public UUID, carried as the token `sub` and
  `sessions.user_pid`.

## 5. Domain model

- **users** — `id`, `pid` (UUID), `email` (unique), `name`,
  `email_verified_at`, magic-link columns, `deleted_at`, `attributes`
  (JSONB `NOT NULL DEFAULT '{}'` — the ABAC string→strings
  subject-attribute map, e.g. `{"access": ["write"]}`, per shared
  authorization-attributes.md §6; `{}` = read-only under the family's
  default policy until an operator assigns attributes), audit
  timestamps. `password` exists only to satisfy `NOT NULL` and holds an
  unusable random hash. `deleted_at` (GDPR Art. 17) soft-deletes the
  account: when set, the `email`/`name` are anonymised to a tombstone
  and every read path treats the user as gone.
- **sessions** — server-side cookie session (per shared §3): `sid`
  (opaque pk, **not** a JWT), `user_pid`, `data` (JSONB — session
  payload; holds the user's ABAC attributes under `attrs`, copied at
  establishment so `POST /token` mints the `attrs` claim from the
  session alone), `created_at`, `last_seen_at`, `idle_expires_at`,
  `absolute_expires_at`, `revoked_at`. One row per logged-in browser;
  the unit of revocation. Valid iff `revoked_at IS NULL AND now() <
  idle_expires_at AND now() < absolute_expires_at`. *(Target shape per
  shared §3; the `data` JSONB column has landed (ABAC sourcing); the
  code otherwise still reuses the legacy `jid`/`expires_at` columns with
  `sid` = `jid`, pending the §13 reshape. Either way the session is an
  opaque server-side row — not a JWT.)*
- **PASETO key material** — an Ed25519 keypair (not in the DB; env /
  files per §8). The private key signs `POST /token`; the public
  key(s) are published at `/.well-known/paseto-keys`, keyed by `kid`.
- **auth_events** — `id`, `event`, `email`, `user_pid`, `detail`,
  timestamps. The durable authentication audit trail (T-10). Never
  stores tokens or secrets.

**Retained loco scaffolding (decision).** The generated mailer
(`src/mailers/auth.rs`) still carries `Emailer::send_welcome` /
`Emailer::forgot_password`, the `welcome` / `forgot` embedded template
dirs, and the password-era model helpers (`set_forgot_password_sent`,
etc.). They are **intentionally retained, unwired** loco starter code:
the passwordless flow never checks a password and no route calls them.
They survive to keep the generated schema/model surface intact and to
ease future diffs against fresh loco scaffolds. The live magic-link
email path does **not** use these templates — it renders from the
`src/i18n.rs` catalog (§6.11). This records the code↔spec agreement
noted in the CHANGELOG "Notes". See §13 for the removal task.

## 6. Functional requirements

1. `POST /api/auth/signup {email, name?, locale?}` — create a
   passwordless account and issue a magic link. Always `200` (no
   enumeration). The optional `locale` (BCP-47; `en` / `cy`; unknown or
   absent ⇒ `en`) selects only the **language of the magic-link email**;
   it never changes the response shape, so the anti-enumeration contract
   holds (see §6.11).
2. `POST /api/auth/magic-link {email, locale?}` — issue a magic link for
   an existing account. Always `200`. `locale` behaves exactly as in
   §6.1.
3. `GET /api/auth/magic-link/{token}` — validate the (unexpired) token,
   mark the email verified, **create a session row and set the
   `__Host-mxi_session` cookie** (HttpOnly/Secure/SameSite/`Path=/`),
   issue a CSRF token, and return `{pid, name, email, is_verified}`. It
   **no longer returns a bearer token** — the credential is the
   `Set-Cookie`. Mechanism unchanged per shared §7; only the outcome
   changes. Session establishment **copies the user's ABAC
   `attributes` into the session** (`sessions.data.attrs`) per shared
   authorization-attributes.md §6, so token minting reads only the
   session.
4. `POST /token` (session cookie, CSRF-protected) — exchange the valid
   session for a short-lived **PASETO v4.public** (`exp` ~5 min, footer
   `kid`) for use as `Authorization: Bearer v4.public.…` at peers. The
   token's **`attrs` claim** carries the session's copied ABAC
   attributes (empty map ⇒ the claim is omitted from the wire, keeping
   the pre-ABAC payload shape).
5. `GET /api/auth/me` (session cookie) — resolve + slide the session,
   reject expired/revoked (`401`), return the current user.
6. `POST /api/auth/signout` (session cookie, CSRF-protected) — set
   `sessions.revoked_at` and clear the cookie.
6a. `GET /.well-known/paseto-keys` — publish the Ed25519 public key(s)
   (the JWKS analog). **Decommissions** `/.well-known/jwks.json`.
6b. **CSRF.** Cookie-authenticated mutating requests
   (`POST`/`PUT`/`PATCH`/`DELETE`, incl. `POST /token`, signout,
   `DELETE /api/auth/account`) require a per-session CSRF token
   (`X-CSRF-Token`) backstopped by an `Origin`/`Referer` allow-list
   (shared §4).

Magic links expire after 5 minutes (`MAGIC_LINK_EXPIRATION_MIN`) and are
single-use (cleared on consumption).

7. **Rate-limited issuance.** The two issuance endpoints (signup,
   magic-link) are throttled per normalised (trimmed, lowercased) email:
   at most `MAX_REQUESTS` (5) requests per `WINDOW` (5 minutes). Over the
   limit the endpoint returns `429 Too Many Requests`
   (`{"error":"rate_limited",…}`) and issues no token / sends no mail.
   The limiter keys on request *volume*, not account existence, so the
   always-`200` anti-enumeration shape of the success path is preserved.
   The window log is stored in Postgres (the `auth_rate_limits` table) and
   each check runs under a per-key advisory lock, so the quota is exact and
   **shared across horizontally-scaled instances**. The window is wall-clock
   (`TIMESTAMPTZ`); `src/rate_limit.rs` exposes a clock-injecting
   `check_at(db, key, now)` plus a `reset(db)` test helper, so the
   sliding-window behaviour is verified by DB-gated tests. A DB error fails
   open (the request is allowed) — the surrounding handler needs the DB
   anyway, so failing closed would only lock out legitimate sign-ins.

8. `GET /api-docs/openapi.json` + `GET /swagger-ui` — the hand-written
   OpenAPI 3 document and a Swagger UI page.

9. **GDPR subject rights (account).** All cookie-authenticated; the
   mutating `DELETE` is CSRF-protected (§6b).
   - `GET /api/auth/account/export` (session cookie) — **right of
     access** (Art. 15): a JSON document of everything the service holds
     about the authenticated subject — their `users` row, their
     `sessions`, and their `auth_events` audit trail. No tokens, key
     material, password hash, or api key.
   - `GET /api/auth/account/audit` (session cookie) — the subject's own
     audit trail (per-subject counterpart to the open `/audit/recent`).
   - `DELETE /api/auth/account` (session cookie, CSRF-protected) —
     **right to erasure** (Art. 17): soft-delete + anonymise the `users`
     row (stamp `deleted_at`, tombstone `email`/`name`), revoke all the
     subject's sessions, and record an `account_erased` audit row. After
     erasure `/me` and the export treat the subject as gone (`401`); any
     already-minted PASETO expires within its ~5-min TTL. Idempotent.

10. **Prometheus metrics.** `GET /metrics.prom` (root path, no `/api`
    prefix, unauthenticated) renders a process-wide registry in
    Prometheus text-exposition format (`Content-Type:
    text/plain; version=0.0.4`), for parity with the older Axum services.
    The metric set is auth-specific (this service has no entity CRUD):
    `auth_signup_total`, `auth_magic_link_issued_total`,
    `auth_magic_link_redeemed_total`, `auth_signout_total`,
    `auth_rate_limited_total` (counters) plus `http_requests_total`
    (counter vec labelled `method` / `path` / `status`). Labels never
    carry a subject identifier — no email, token, pid, or magic-link
    material — so the monitoring system holds no personal data
    (`src/metrics.rs`; the no-secret-labels contract is unit-tested
    DB-free).

11. **Localized magic-link email (en / cy).** The magic-link email ships
    in **English (`en`)** and **Welsh (`cy`)**. `SignupParams` and
    `MagicLinkParams` carry an optional `locale` field; both issuance
    handlers call `i18n::select_locale(params.locale)` and render the
    email via `Emailer::send_magic_link(ctx, user, locale)`. The catalog
    (`src/i18n.rs`) is a dependency-light pure function over a locale
    string — no templating engine, no on-disk template, testable DB-free.
    `select_locale` normalises the request-body `locale` (case-insensitive,
    region subtag dropped: `cy-GB` → `cy`), falling back to `en` for any
    unsupported or absent tag. The **only** wired selection input is the
    request-body field (no `Accept-Language` parsing today). Locale
    affects only the email language: the always-`200`,
    identical-shape anti-enumeration response is unchanged across locales.
    The magic-link URL is locale-independent. The compliance basis is the
    Welsh Language (Wales) Measure 2011 (see §12); add a locale by
    extending `SUPPORTED_LOCALES` + `magic_link_email`.

12. **ABAC attribute sourcing** (shared
    [`authorization-attributes.md`](../../../agents/share/authorization-attributes.md)
    §6). The auth service is the *sourcing* side of the family's ABAC
    model; peers enforce. Three legs:
    - `users.attributes` (JSONB `NOT NULL DEFAULT '{}'`) holds the
      subject's string→strings attribute map (e.g. `access: ["write"]`,
      `svc: ["true"]` for machine peers).
    - Magic-link redemption copies the map into the new session's
      `data.attrs` (§6.3) — the session is then the single read for
      minting.
    - `POST /token` (and the transitional redemption-body token) mints
      the map into the PASETO **`attrs`** claim (§6.4), mirrored
      byte-for-byte by `authentication_verifier::Claims` 0.3. Parsing of
      the stored JSONB is tolerant (`users::attributes_map`):
      malformed entries are inert and can never fail minting.
    - Attribute **assignment** is an operator action. The **CLI task**
      has **landed** (`user_attributes`, `src/tasks/attributes.rs`):
      `op:show|set|unset|clear` over one user's `users.attributes`,
      selected by `email:` or `pid:`, with lowercase key/value
      validation and the reserved pseudo-attributes `sub`/`email`/
      `entity` refused. An HTTP admin API is a later follow-up (§13).
      Until assigned, users hold `{}` and are read-only under the
      family's default policy.

## 7. Non-functional requirements

- **Asymmetric, not HS256.** Post-pivot: PASETO v4.public (Ed25519) —
  no shared secret; peer services verify offline.
- **No token in the browser.** The human session is a server-side
  cookie session; only the httpOnly `__Host-mxi_session` cookie reaches
  the browser. No `localStorage` credentials.
- **Short token TTL** (PASETO ~5 min) bounds the staleness window of
  offline verification against revocation.
- **No SMTP in dev**: links are logged to the tracing console.
- **Dependency-light i18n**: user-facing email copy is a pure-Rust
  catalog (`src/i18n.rs`, en / cy) — no templating engine, no extra
  crate, no DB. See §6.11.
- **Deterministic keys**: a stable built-in dev Ed25519 seed (no
  committed key files) so the
  `/.well-known/paseto-keys` set is stable across restarts in dev.
- **Abuse resistance**: per-email sliding-window rate limiting on
  magic-link issuance (`MAX_REQUESTS` = 5 per `WINDOW` = 5 min) bounds
  email-bombing and account-probing without breaking anti-enumeration.
  Backed by Postgres (the `auth_rate_limits` table) under a per-key
  advisory lock, so the quota is exact and shared across
  horizontally-scaled instances.

## 8. Architecture

loco.rs `App` (`src/app.rs`) registers the `auth`, `token`,
`paseto_keys`, `docs`, and `metrics` controllers and the
Postgres-backed worker queue. Token crypto is a self-contained module
(`src/auth`); it mints + verifies **PASETO v4.public**
(Ed25519, via `rusty_paseto`) — RS256 JWTs are decommissioned. The
session/cookie extractor and the PASETO bearer extractor are plain Axum
`FromRequestParts`, so peer services can reuse the same verification
approach.

> The key-set + zero-downtime rotation structure below carried over
> unchanged from the decommissioned RS256/JWKS model, swapping
> RSA→Ed25519 and `/.well-known/jwks.json`→`/.well-known/paseto-keys`;
> see the umbrella
> [`../../spec/08-architecture.md`](../../spec/08-architecture.md)
> and shared
> [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).

**Key set & rotation.** `auth::AuthKeys` holds a **set** of keys: one
primary Ed25519 signing seed (`TOKEN_PRIVATE_KEY_SEED` /
`TOKEN_PRIVATE_KEY_FILE`; unset ⇒ the built-in dev seed) plus zero or
more additional verify-only public keys
(`TOKEN_ADDITIONAL_PUBLIC_KEYS`; unset ⇒ just the primary,
backward-compatible). Signing uses the primary and stamps its `kid`
into the token footer; verification selects the key by `kid` from
{primary} ∪ {additional};
the published key set carries all keys (primary first). This enables
operator-driven, **zero-downtime key rotation** — see the entity spec
§8.4 runbook and
[`config/keys/README.md`](../config/keys/README.md). No auto-rotation
scheduler (follow-up).

## 9. API surface

See §6. Responses are raw loco JSON (no envelope). Errors use loco's
standard error responses (`401` unauthorized, `400` bad request, `403`
CSRF failure, `429` too many requests on throttled issuance). The
login/verify path returns `Set-Cookie: __Host-mxi_session`; `POST /token`
returns a PASETO v4.public; `/.well-known/paseto-keys` publishes the
Ed25519 keys. **Decommissioned:** `/.well-known/jwks.json` and the
RS256 bearer-token response body (shared §9).

The API is described by a hand-written **OpenAPI 3.0.3** document
(`src/openapi.rs`; the family authors these by hand rather than via
`utoipa`). It is served by the docs controller
(`src/controllers/docs.rs`) at `GET /api-docs/openapi.json`, with a
Swagger UI page at `GET /swagger-ui` (CDN assets). The document covers
every endpoint plus the `SignupParams` /
`MagicLinkParams` / `CurrentResponse` / `Claims` (PASETO) /
`PasetoKeys` / `PasetoKey` / `AuthEvent` / `AccountExport`
(+ `AccountUserExport` / `AccountSessionExport` / `AccountAuditExport`)
schemas, the `429` rate-limit and `403` CSRF responses, the
`__Host-mxi_session` cookie security scheme applied to the cookie-gated
endpoints (`me` / `token` / `signout` / `account/export` /
`account/audit` / `account` DELETE), and the `GET /metrics.prom`
Prometheus endpoint. The decommissioned `LoginResponse` / `Jwks` /
`Jwk` / JWKS path are removed. Un-gated `spec()` unit tests pin its
well-formedness, the documented paths (including `/metrics.prom`), the
security scheme, and the schemas.

A separate `GET /metrics.prom` (root, unauthenticated) serves the
Prometheus registry (see §6.10) — not JSON, so it sits outside the
loco-JSON envelope above.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations:
`m20220101_000001_users`, `m20220101_000002_sessions`,
`m20220101_000003_auth_events`, `m20220101_000004_users_deleted_at` (the
GDPR-erasure soft-delete column), `m20220101_000005_auth_rate_limits`
(the magic-link rate-limiter window log),
`m20220101_000006_users_attributes` (the ABAC subject-attribute map,
JSONB `NOT NULL DEFAULT '{}'`), and `m20220101_000007_sessions_data`
(the session payload JSONB holding the copied `attrs`). `auto_migrate`
is on in development, off in production.

**Cookie sessions.** The cookie session currently reuses the existing
`sessions` table (`sid` = the legacy `jid` column); the `data` JSONB
column has landed (ABAC sourcing, §6.12), and the reshape to the rest
of the shared-§3 schema (`last_seen_at`, idle/absolute TTLs, partial
index on `(user_pid) WHERE revoked_at IS NULL`) is a deferred §13
follow-up. The **Ed25519** PASETO signing seed lives in
env / a file (not the DB; built-in dev seed otherwise), public keys
published at `/.well-known/paseto-keys`. RS256 RSA keys are
decommissioned.

## 11. Testing strategy

- **Unit (DB-free):** `src/auth` — PASETO sign/verify roundtrip,
  published key-set shape (Ed25519 entries, `kid` =
  base64url(SHA-256(public bytes))),
  tampered/garbage-token rejection, and **key rotation**: a single-key
  set is backward-compatible (same `kid`), a multi-key set
  publishes all keys (primary first), a token signed by a now-additional
  (verify-only) key still verifies via footer-`kid` lookup, and an
  unknown `kid` is
  rejected. Run with `cargo test --lib`.
- **Request tests:** loco's `tests/requests/auth.rs` exercises the §6
  magic-link surface (signup / magic-link / redeem incl. single-use and
  anti-enumeration / me / signout / paseto-keys). The HTTP tests require a
  Postgres instance (standard loco) and are `#[ignore]`d so plain
  `cargo test` stays green; run them with `cargo test -- --ignored`.
  DB-free route-table and params-contract assertions always run.
- **Rate-limit tests:** the pure key normalisation is unit-tested DB-free
  in `src/rate_limit.rs`. The sliding-window behaviour is DB-gated
  (`tests/requests/rate_limit.rs`, `#[ignore]`d): allow up to
  `MAX_REQUESTS`, reject the next, window reset, sliding-window single-slot
  release, per-key isolation, normalised-key sharing, non-consuming
  rejection — each driving the real `auth_rate_limits` table with a `now`
  injected via `check_at`. The end-to-end `magic_link_issuance_is_rate_limited`
  request test asserts the `(MAX_REQUESTS+1)`th magic-link POST returns `429`.
- **OpenAPI unit tests (DB-free):** `src/openapi.rs` `spec()` — well-formed,
  documents every endpoint, the bearer scheme is present + applied (incl.
  the GDPR account endpoints), core schemas exist, and the account export
  schema advertises no credential/secret fields. The docs route table is
  asserted in `tests/requests/auth.rs`.
- **GDPR unit tests (DB-free):** `src/models/users.rs` — the
  `tombstone_email` transform is pid-keyed, unroutable, irreversible;
  `src/views/auth.rs` — `AccountExport::new` assembles user + sessions +
  audit rows from in-memory models and serialises **no** password hash /
  api key / token. DB-gated request tests (`tests/requests/auth.rs`,
  `#[ignore]`d): export returns the caller's data; erasure soft-deletes +
  anonymises + revokes sessions + writes the `account_erased` row;
  post-erasure `/me` + export are `401`; unauthenticated
  export/audit/delete are `401`.
- **i18n unit tests (DB-free):** `src/i18n.rs` — `magic_link_email`
  returns the English / Welsh copy, an unknown tag falls back to English,
  a region subtag reduces to its primary language (`cy-GB` → `cy`),
  `render` substitutes the `{link}` placeholder in both bodies,
  `select_locale` maps input to a supported tag (or `en`), and every
  `SUPPORTED_LOCALES` entry has distinct copy (guards a missing
  translation). `tests/requests/auth.rs` adds the params-contract tests
  (`SignupParams` / `MagicLinkParams` accept an optional `locale`), a
  DB-gated anti-enumeration test
  (`signup_locale_does_not_change_the_response_shape`), and a DB-free
  mailer-bridge unit test (`selected_locale_renders_the_mailer_email_copy`)
  that pins the exact `select_locale → magic_link_email → render`
  expression `Emailer::send_magic_link` evaluates — including the
  `{frontend}/verify?token={token}` link substitution and the
  English fallback — so the `params.locale → …` path is pinned at the
  catalog boundary without SMTP or a database. A connected
  handler→mailer SMTP test (needing a mail-capture harness) remains the
  §13 follow-up.
- **Metrics unit tests (DB-free):** `src/metrics.rs` — `render()` emits
  valid Prometheus text (every metric name plus its `# HELP` / `# TYPE`
  header lines after the counters are incremented) and the exposition
  carries **no secret-ish labels** (`email` / `token` / `magic_link` /
  `pid` / `jti` / `user_pid`). Run with `cargo test --lib`.
- **Cross-crate contract test (DB-free):** `tests/sign_verify_contract.rs`
  pins the convention shared with
  [`authentication-verifier`](../../authentication-verifier-rust-crate/index.md):
  a PASETO signed by `auth::sign_access_token` verifies through the
  verifier crate built from this service's published key set; the claims
  round-trip and `kid` = base64url(SHA-256(public key bytes)) holds; a
  `kid`
  mismatch fails. A **multi-key** case asserts a verifier built from a
  key set carrying more than the primary key still verifies a
  primary-signed
  token and rejects a token whose `kid` is absent from the set. Since
  verifier 0.3 the contract also pins the ABAC **`attrs`** claim: a
  non-empty map minted here round-trips through the peer verifier, and
  an empty map is **omitted from the wire** (`skip_serializing_if`) yet
  still verifies to an empty map (pre-ABAC payload shape preserved).
- **ABAC sourcing unit tests (DB-free):** `src/models/users.rs` —
  `attributes_map` parses the string→strings shape and is tolerant of
  malformed stored values (bare string coerces, non-string list items
  skipped, other shapes inert); `src/models/sessions.rs` — the §6 copy
  path round-trips (`users.attributes` → `session_data` →
  `Model::attrs`), and a pre-ABAC session (`{}` / no `attrs`) yields an
  empty map; `src/auth` — a non-empty `attrs` claim round-trips
  mint→verify and an empty map is omitted from the serialized payload;
  `src/openapi.rs` — the `Claims` schema documents `attrs` as an
  optional string→string-array map.

## 12. Compliance

Email is personal data: minimise, never log tokens alongside avoidable
PII in production, and honour the family's GDPR posture. Sessions give
an audit trail of login and revocation; `auth_events` is the durable
authentication audit trail.

**Session-cookie posture (post-pivot, GDPR Art. 32).** The human
credential is a server-side cookie session: the browser holds only the
`__Host-mxi_session` httpOnly/Secure/host-locked cookie (no token in
browser JS — killing the `localStorage` exfiltration class), CSRF
protection guards cookie-authenticated mutations, and the cross-service
PASETO is short-lived (~5 min) and held server-side at the BFF, never
in the browser. The decommissioned RS256 JWT + `localStorage` storage
are removed.

**Welsh-language duty.** As a UK governmental-style single sign-on
provider, the service honours the **Welsh Language (Wales) Measure
2011** (the public-sector "treat Welsh no less favourably than English"
expectation): the magic-link email ships in both English (`en`) and
Welsh (`cy`), selected per request via the optional `locale` field
(§6.11). Additional locales are added by extending the `src/i18n.rs`
catalog.

**GDPR subject rights (T-9).**
- *Right of access (Art. 15)* — `GET /api/auth/account/export` (bearer)
  returns the subject's `users` row + `sessions` + `auth_events`. No
  tokens, key material, password hash, or api key are exported.
- *Right to erasure (Art. 17)* — `DELETE /api/auth/account` (bearer)
  **soft-deletes + anonymises**: stamp `users.deleted_at`, replace
  `email` with a `pid`-keyed unroutable tombstone
  (`deleted+<pid>@invalid`) and `name` with `"deleted user"`, revoke all
  the subject's sessions, and record an `account_erased` audit row. The
  row survives so referential history and the audit trail keep their
  integrity (erasure is anonymisation, not deletion). Every read path
  (`/me`, export) treats a `deleted_at` user as gone (`401`), even while
  the already-issued bearer token verifies cryptographically until `exp`
  (bounded by the short TTL).

**Audit gating decision (revised — SEC-A2, 2026-07-13).** The system-wide
`GET /api/auth/audit/recent` is now **admin-gated** (a valid PASETO bearer
whose attributes include `access=admin`; `401` without a token, `403` for a
non-admin). Although the rows carry no tokens or secrets, they DO carry
registered **emails** plus outcome markers (`created`/`existing`,
`unknown_email`/`issued`, `rate_limited`), which — left unauthenticated —
form an **account-enumeration oracle**: an attacker triggers a signup for a
target email and reads back the outcome, undoing the always-`200`
anti-enumeration contract the unauthenticated endpoints preserve. (This
supersedes the earlier "left open, mirrors care-pathway" decision.) The
GDPR right-of-access requirement is still met by the session/bearer-gated
per-subject `GET /api/auth/account/audit` — a subject's own trail reachable
only by that subject.

## 13. Tasks (live work queue)

- [x] **SEC-A7 (security): complete GDPR erasure.** Account erasure now
      scrubs the subject's email from `auth_events` (`scrub_subject_email`,
      pid OR normalised-email match) and `sessions.user_agent`
      (`scrub_user_agent_for_user`), and writes the terminal `account_erased`
      row without the email. (Repo tasks.md SEC-A7.)
- [x] **SEC-A8 (security): revoke sessions on attribute change.** The admin
      attribute API + the `user_attributes` CLI task now
      `revoke_all_for_user` after a change, so a session that snapshotted the
      old ABAC attributes can't keep minting stale-attribute tokens until its
      absolute TTL — the next login re-copies fresh attributes. (Repo
      tasks.md SEC-A8.)
- [x] **SEC-A4 (security): atomic single-use magic-link consume.** Redemption
      was `find_by_magic_token` (SELECT) + `clear_magic_link` (UPDATE), so two
      concurrent redemptions both passed the read and each minted a session.
      `Model::consume_magic_token` now clears-and-returns in one
      `UPDATE … WHERE magic_link_token=$1 AND not-expired RETURNING *`, so
      exactly one concurrent redemption wins (loser ⇒ `401`). DB-gated
      `concurrent_magic_link_redemptions_only_one_wins`. (Repo tasks.md SEC-A4.)
- [x] **SEC-A1 (security): refuse the dev signing seed in production.**
      `load_seed()` fell back to the committed `DEV_SEED` with no
      environment guard, so a prod deploy missing `TOKEN_PRIVATE_KEY_SEED`
      would sign forgeable PASETOs. Now the `DEV_SEED` fallback is refused
      when `LOCO_ENV`/`RUST_ENV` = `production` (`load_keys()` errors →
      `keys()` boot-panics); dev/test still fall back. Pure
      `dev_seed_fallback` + unit test. (Repo tasks.md Phase 5 SEC-A1.)
- [x] Rework `tests/requests/auth.rs` + snapshots for the magic-link /
      signout / me / JWKS surface (drop password-flow tests). Done:
      assertion-based tests covering signup / magic-link / redeem
      (incl. single-use + anti-enumeration) / me / signout / JWKS;
      Postgres-requiring tests are `#[ignore]`d (run with
      `cargo test -- --ignored`); the old password-flow snapshots are
      removed.
- [x] A reusable verifier crate/snippet for peer services to consume the
      JWKS (input to the loco conversion of the other crates). Done:
      [`../authentication-verifier-rust-crate/`](../../authentication-verifier-rust-crate/index.md)
      — offline RS256 verification (`Verifier::from_jwks_value` /
      `from_jwks_url` behind the `fetch` feature), mirrored `Claims`.
- [x] Cross-crate contract test: `tests/sign_verify_contract.rs` signs
      with this crate's `auth` module and verifies through the
      `authentication-verifier` dev-dependency, pinning the `Claims`
      round-trip and the `kid` thumbprint contract. DB-free, un-gated.
- [x] Binary/library lint conformance: `src/bin/main.rs` carries the
      `//!` crate doc, `#![warn(clippy::pedantic)]`,
      `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, and the
      `cfg(target_env = "musl")` `mimalloc` global allocator; `src/lib.rs`
      carries the same three lints with `deny(missing_docs)` satisfied
      across the whole crate (generated `_entities` are `allow`ed at the
      module). Clippy is warning-free. *(2026-06-13)*
- [x] Rate-limit magic-link issuance (`src/rate_limit.rs`): per-email
      sliding window (`MAX_REQUESTS` = 5 / `WINDOW` = 5 min), monotonic
      `Instant` clock, wired into signup + magic-link → `429` over the
      cap with the always-`200` anti-enumeration shape preserved.
      Un-gated unit tests + a DB-gated request test. *(2026-06-13;
      entity spec T-6.)*
- [x] OpenAPI 3 + Swagger UI: hand-written `src/openapi.rs` served by
      `src/controllers/docs.rs` at `/api-docs/openapi.json` +
      `/swagger-ui`; documents all six endpoints + schemas + bearer
      scheme; un-gated `spec()` tests. *(2026-06-13; entity spec T-8.)*
- [x] Authentication event audit trail (`auth_events` table, migration
      `m20220101_000003_auth_events`): durable rows
      `(id, event, email, user_pid, detail, created_at)` for signup /
      magic-link request / redeem / signout, written best-effort
      (`models/auth_events.rs`, never fails the request, never stores a
      token). Anti-enumeration preserved (the row distinguishes
      `unknown_email` / `rate_limited`, the 200 response does not).
      Queryable at `GET /api/auth/audit/recent`; OpenAPI documents the
      endpoint + `AuthEvent` schema. Un-gated unit tests + a DB-gated
      request test (`auth_events_are_recorded_and_queryable`).
      *(2026-06-13; entity spec T-10.)*
- [x] GDPR subject-rights workflow (`account` controller surface):
      **right of access** `GET /api/auth/account/export` (bearer) — the
      subject's `users` row + `sessions` + `auth_events` (no tokens / key
      material / password hash / api key; `views/auth::AccountExport`);
      **right to erasure** `DELETE /api/auth/account` (bearer) —
      soft-delete + anonymise (`users.deleted_at`, migration
      `m20220101_000004_users_deleted_at`; email→`deleted+<pid>@invalid`,
      name→`"deleted user"`), revoke all sessions, write an
      `account_erased` audit row; `/me` + export then `401` for the
      erased subject (`users::find_active_by_pid`). Per-subject
      `GET /api/auth/account/audit` (bearer) added; system-wide
      `/audit/recent` left open by decision (§12). Un-gated unit tests +
      DB-gated request tests; OpenAPI documents the three endpoints +
      `AccountExport`/`AccountSessionExport`/`AccountAuditExport`
      schemas. *(2026-06-13; entity spec T-9.)*
- [x] Key rotation (`src/auth`): `AuthKeys` is a **key set** — one
      primary signing key + zero or more additional verify-only public
      keys from `JWT_ADDITIONAL_PUBLIC_KEY_FILES` /
      `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` (unset ⇒ single primary,
      backward-compatible). `sign_access_token` signs with the primary;
      `verify_token` selects by token-header `kid`; the JWKS publishes all
      keys (primary first); unknown `kid` rejected. `load_from(...)` test
      constructor + un-gated unit tests; multi-key contract test in
      `tests/sign_verify_contract.rs`. Operator runbook in §8.4 /
      `config/keys/README.md`. *(2026-06-13; entity spec T-5.)*
- [x] Localized magic-link email (en / cy): dependency-light
      `src/i18n.rs` catalog (`magic_link_email` / `select_locale` /
      `EmailStrings::render`); optional `locale` field on `SignupParams`
      / `MagicLinkParams`; both issuance handlers select the locale and
      `Emailer::send_magic_link` renders it. OpenAPI documents the field;
      anti-enumeration response shape unchanged across locales; Welsh
      Language (Wales) Measure 2011 compliance basis (§12). Un-gated i18n
      + params + mailer-bridge render unit tests + a DB-gated
      anti-enumeration request test. *(2026-06-15.)*
- [ ] Optional Mailpit docker-compose service for realistic dev email.
- [ ] Remove the unwired password-era loco scaffolding (mailer
      `send_welcome` / `forgot_password`, the `welcome` / `forgot`
      template dirs, `users::set_forgot_password_sent` and the password-era
      model tests) now that the decision to retain them is recorded (§5).
      Tracked so code and spec stay aligned; low priority.
- [ ] Add an end-to-end handler test that a `cy` issuance request renders
      the Welsh subject/body through `Emailer::send_magic_link` (the
      current coverage pins the catalog + params + anti-enumeration shape,
      and a DB-free mailer-render unit test, but not the connected
      handler→mailer SMTP path). Needs a mail capture harness.
- [x] **Pivot off JWT-for-sessions → cookie sessions + PASETO v4.public**
      (entity spec §13 T-12; supersedes the RS256 JWT + JWKS model). Per
      [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
      **Landed** (see CHANGELOG `[Unreleased]`), with two deferred
      refinements tracked below:
  - [x] **Sessions + cookie issuance.** Magic-link redemption creates a
        session and sets the
        `__Host-mxi_session` cookie (HttpOnly/Secure/SameSite/`Path=/`);
        `/me` resolves the
        session; signout sets `revoked_at` + clears the cookie. The
        cookie session reuses the existing `sessions` table
        (`sid` = the legacy `jid` column). Transitionally, redemption
        *also* still returns the PASETO in the body until every
        front-end adopts the BFF.
  - [x] **`POST /token` PASETO minting.** Exchanges a valid session for a
        short-lived (~5 min) PASETO **v4.public** (Ed25519, claims as
        §5, footer `kid`); `rusty_paseto` + `ed25519-dalek`
        (`#![forbid(unsafe_code)]` holds).
  - [x] **Ed25519 key publication.** `/.well-known/paseto-keys`
        (`src/controllers/paseto_keys.rs`); Ed25519 seed from
        `TOKEN_PRIVATE_KEY_SEED` / `TOKEN_PRIVATE_KEY_FILE`, built-in
        dev seed otherwise — no committed key files.
  - [x] **Remove RS256/JWKS.** Dropped `src/controllers/jwks.rs`,
        `/.well-known/jwks.json`, RS256 signing + the `Jwks` /
        `Jwk` schemas from OpenAPI; the `jsonwebtoken`/`rsa` stack is
        gone.
  - [x] **CSRF synchroniser token** *(2026-07-05)*. Per-session token
        minted at `verify`, stored in `sessions.data.csrf`, delivered in
        the readable `__Host-mxi_csrf` cookie; `POST /token` requires the
        `X-CSRF-Token` header to match (constant-time) → `403` on
        mismatch; composes with the `Origin` backstop. Legacy sessions
        (no token) skip the check. `src/csrf.rs` + DB-free tests.
  - [x] **Sessions-table reshape (idle/absolute TTLs)** *(2026-07-05)*.
        Migration `…_000008_sessions_ttls` adds `last_seen_at` /
        `idle_expires_at` / `absolute_expires_at` (nullable) + the
        `sessions_active_user` partial index. `is_active` now enforces
        idle + absolute (was revocation-only); `issue` sets the TTLs
        (`AUTH_SESSION_IDLE_TTL_SECS` def 30 m, `AUTH_SESSION_ABSOLUTE_TTL_SECS`
        def 12 h, independent of the token exp); `touch` slides the idle
        window on `/me` (capped at absolute). `sid` rotates per
        magic-link login already. Pure `is_active_at` test. *(The `jid`
        column stays the opaque `sid`; a `sid`-pk rename was judged a
        larger, lower-value migration and deliberately deferred.)*
  - **Acceptance (met):** redemption returns
        `Set-Cookie: __Host-mxi_session`; `POST /token` mints a PASETO a
        verifier built from
        `/.well-known/paseto-keys` accepts; signout sets `revoked_at`; no
        RS256/JWKS path remains; OpenAPI + the cross-crate contract test
        cover PASETO.
- [x] **ABAC sourcing (attrs)** (shared
      [`authorization-attributes.md`](../../../agents/share/authorization-attributes.md)
      §6, rollout step 4; supersedes any per-crate roles/RBAC sketch).
      `users.attributes` JSONB `NOT NULL DEFAULT '{}'` (migration
      `m20220101_000006_users_attributes`) + `sessions.data` JSONB
      (migration `m20220101_000007_sessions_data`); magic-link
      redemption copies the user's attributes into the session
      (`sessions::session_data`); `POST /api/auth/token` (and the
      transitional redemption body) mints the session's attributes into
      the PASETO **`attrs`** claim — `auth::Claims` stays byte-identical
      to `authentication_verifier::Claims` 0.3
      (`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`).
      Tolerant `users::attributes_map` parsing (malformed stored values
      are inert, never fail minting); the GDPR account export includes
      `attributes` (subject data); OpenAPI documents `attrs` +
      `attributes` and deprecates `scope`/`roles` for authorization.
      DB-free unit tests (parser, copy round-trip, claim
      serialization, OpenAPI) + two new cross-crate contract tests
      (non-empty `attrs` round-trips; empty `attrs` omitted on the wire
      and still verifies). *(2026-07-05.)*
- [x] **ABAC attribute assignment surface — CLI task** (follow-up to
      the above, per shared authorization-attributes.md §6). The
      `user_attributes` loco task (`src/tasks/attributes.rs`, registered
      in `App::register_tasks`) sets/inspects `users.attributes` for one
      operator-selected user: `cargo loco task user_attributes
      (email:<addr>|pid:<uuid>) [op:show|set|unset|clear] [key:<name>]
      [values:<v1,v2,…>]`. `set` replaces a key's value list, `unset`
      removes a key, `clear` empties the map, `show` (default) prints it;
      it writes via `users::ActiveModel::set_attributes` (canonicalised
      by `users::attributes_to_value`, the lossless inverse of
      `attributes_map`) and prints a before/after report, logging the
      change via `tracing`. Keys/values are validated as short lowercase
      tokens and the reserved pseudo-attributes `sub`/`email`/`entity`
      are refused. Machine peers get `svc=true` tokens from ops. DB-free
      unit tests pin value parsing, key/value validation, command
      parsing, and the map-mutation ops. *(2026-07-05.)*
  - [x] **HTTP admin API + per-assignment audit** *(2026-07-05)*.
        `GET`/`PUT /api/auth/admin/users/{pid}/attributes`
        (`src/controllers/admin.rs`, mounted in `App::routes`): show /
        replace a user's `users.attributes` from an authenticated caller
        whose own attributes include `access=admin` (bootstrap admin via
        the CLI). `401` no/invalid token; `403` valid non-admin; `404`
        unknown/erased user; `422` on a bad body (keys/values validated
        with the CLI task's `validate_key`/`validate_value` — reserved
        `sub`/`email`/`entity` refused, no empty value lists). Both the
        CLI task and this endpoint now write an **`attributes_assigned`**
        `auth_events` row (`Model::record_attribute_assignment_best_effort`;
        actor = `cli` or the admin's `pid:<uuid>`; attribute values
        omitted from the audit detail). OpenAPI documents both verbs
        (`UserAttributes` / `ReplaceUserAttributes` schemas, `admin`
        tag). DB-free tests: `require_admin` gate, `validate_map`,
        OpenAPI admin-path assertions; DB-gated request tests
        (`tests/requests/admin.rs`): admin replace+show+audit, non-admin
        `403`, missing-token `401`.

## 14. Implementation status

> **Pivot landed.** The code reality is cookie sessions + PASETO
> v4.public (§1, §13 T-12,
> [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md));
> RS256 JWT + JWKS are removed. Remaining refinements: full
> double-submit CSRF and the sessions-table reshape (§13).

Done: real loco scaffold; passwordless magic-link flow; PASETO
v4.public signing (Ed25519); `/.well-known/paseto-keys` key set;
cookie sessions (`__Host-mxi_session`) + `POST /token` session→PASETO
exchange + signout; console magic links; Postgres queue;
green `cargo build`, clippy clean, DB-free unit tests passing;
magic-link request tests (Postgres-gated); peer-service verifier crate
(`../authentication-verifier-rust-crate/`) with a DB-free cross-crate
contract test; per-email rate limiting on magic-link issuance (`429`);
hand-written OpenAPI 3 + Swagger UI; durable `auth_events` audit trail
(`GET /api/auth/audit/recent`); GDPR subject-rights workflow
(`GET /api/auth/account/export`, `GET /api/auth/account/audit`,
`DELETE /api/auth/account` — right of access + soft-delete/anonymise
erasure, `users.deleted_at`); operator-driven **zero-downtime key
rotation** (`AuthKeys` is a primary + additional verify-only key set;
the published key set carries all keys; `verify_token` selects by the
footer `kid`); localized
magic-link email (en / cy via `src/i18n.rs`, optional request `locale`);
**ABAC attribute sourcing** (`users.attributes` → session `data.attrs`
→ PASETO `attrs` claim, per shared authorization-attributes.md §6;
assignment surface deferred — §13).

## 15. Roadmap

v0.1: core magic-link + RS256/JWKS + signout, reworked request
tests, peer-service verifier + contract test, operator-driven key
rotation (multi-key set). **v0.2 (the pivot — landed):**
the human session moved to a `__Host-mxi_session` cookie session,
cross-service auth to **PASETO v4.public** (`POST /token` +
`/.well-known/paseto-keys`), front-ends to the BFF
pattern, and RS256 + JWKS decommissioned — per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
This **supersedes** the JWT model. Remaining v0.2 items: full
double-submit CSRF, the sessions-table reshape, Mailpit,
auto-rotation scheduler. v0.3: begin loco conversion of the sibling
services using this as the template (peers adopt the PASETO
`authentication-verifier`); **ABAC** — the sourcing side (attributes →
session → `attrs` claim) has landed here, peers enforce via the shared
`abac` engine in verifier 0.3 (per shared authorization-attributes.md —
authorization is attribute-based, not a fixed role list); next: the
operator attribute-assignment surface (§13).

## 16. Open questions

- Refresh tokens vs. short-lived tokens only? (Post-pivot the human
  session is a cookie session with idle + absolute TTLs; the
  cross-service PASETO is ~5 min and re-minted from the session.)
- Should cross-service revocation propagate to peers (an optional `sid`
  deny-list peers poll) or rely on the short PASETO `exp`? (Lean:
  expiry only — shared §10.)
- Audience model when peers need distinct audiences.
- ~~PASETO library choice~~ — resolved: `rusty_paseto` (+
  `ed25519-dalek`) shipped. Still open: BFF token-exchange
  caching (shared §10).

## 17. References

- loco.rs — https://loco.rs/
- [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  (session + PASETO design — the pivot's source of truth);
  [`agents/share/jwt.md`](../../../agents/share/jwt.md) (the principle).
- PASETO v4 — https://paseto.io/ ; Ed25519 (RFC 8032); RFC 6265bis
  (cookies — `__Host-` prefix, `SameSite`).
- RFC 7519 (JWT), RFC 7517 (JWK), RFC 7518 (JWA / RS256) —
  **decommissioned** model, kept for historical reference.

## 18. Change control

Update this spec in the same PR as any behavioural change. Bump
`CHANGELOG.md` under `[Unreleased]`.
