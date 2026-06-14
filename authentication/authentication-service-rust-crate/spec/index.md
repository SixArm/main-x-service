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
**passwordless email magic links** and issues **RS256 JWT** access
tokens that every other service in the family verifies **offline**
against a published JWKS. There is one place to sign in, one place to
revoke, and one set of keys to trust.

It is also the family's **reference loco.rs application**. The existing
service crates declare `loco-rs` but run hand-rolled Axum; they will be
converted to idiomatic loco using this crate as the template.

## 2. Scope

In scope: sign up, sign in, sign out — all via magic link; JWT issuance;
JWKS publication; session revocation; the user record.

Out of scope (for now): passwords, OAuth/social login, multi-factor,
roles/permissions/authorization (services authorize locally from claims),
organization/tenant modelling, account self-service beyond sign-in.

## 3. Stakeholders and users

- **Operators** — humans who sign into any Main X front-end.
- **Peer services** — person/worker/place/thing/event/course services
  that accept this service's tokens.
- **Front-end** — `authentication-front-end-with-svelte`.

## 4. Glossary

- **Magic link** — a one-time, short-lived URL containing an opaque
  token that signs a user in without a password.
- **JWKS** — JSON Web Key Set; the public keys at
  `/.well-known/jwks.json` used to verify token signatures offline.
- **jti / jid** — the JWT id; stored as `sessions.jid` to enable
  revocation.
- **pid** — a user's public UUID, carried as the token `sub`.

## 5. Domain model

- **users** — `id`, `pid` (UUID), `email` (unique), `name`,
  `email_verified_at`, magic-link columns, `deleted_at`, audit
  timestamps. `password` exists only to satisfy `NOT NULL` and holds an
  unusable random hash. `deleted_at` (GDPR Art. 17) soft-deletes the
  account: when set, the `email`/`name` are anonymised to a tombstone
  and every read path treats the user as gone.
- **sessions** — `jid` (unique, = token `jti`), `user_pid`,
  `expires_at`, `revoked_at`, `user_agent`. One row per issued token;
  the unit of revocation.
- **auth_events** — `id`, `event`, `email`, `user_pid`, `detail`,
  timestamps. The durable authentication audit trail (T-10). Never
  stores tokens or secrets.

## 6. Functional requirements

1. `POST /api/auth/signup {email, name?}` — create a passwordless
   account and issue a magic link. Always `200` (no enumeration).
2. `POST /api/auth/magic-link {email}` — issue a magic link for an
   existing account. Always `200`.
3. `GET /api/auth/magic-link/{token}` — validate the (unexpired) token,
   mark the email verified, issue an RS256 access token, record a
   session, and return `{token, pid, name, email, is_verified}`.
4. `GET /api/auth/me` (bearer) — return the current user; reject if the
   session has been revoked locally.
5. `POST /api/auth/signout` (bearer) — revoke the current session.
6. `GET /.well-known/jwks.json` — publish the RSA public key(s).

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

9. **GDPR subject rights (account).**
   - `GET /api/auth/account/export` (bearer) — **right of access**
     (Art. 15): a JSON document of everything the service holds about
     the authenticated subject — their `users` row, their `sessions`,
     and their `auth_events` audit trail. No tokens, key material,
     password hash, or api key.
   - `GET /api/auth/account/audit` (bearer) — the subject's own audit
     trail (per-subject counterpart to the open `/audit/recent`).
   - `DELETE /api/auth/account` (bearer) — **right to erasure**
     (Art. 17): soft-delete + anonymise the `users` row (stamp
     `deleted_at`, tombstone `email`/`name`), revoke all the subject's
     sessions, and record an `account_erased` audit row. After erasure
     the bearer token still verifies cryptographically until expiry, but
     `/me` and the export treat the subject as gone (`401`). Idempotent.

## 7. Non-functional requirements

- **RS256, not HS256.** No shared secret; peer services verify offline.
- **Short token TTL** (default 1h) bounds the staleness window of
  offline verification against revocation.
- **No SMTP in dev**: links are logged to the tracing console.
- **Deterministic keys**: a stable committed dev keypair so the JWKS is
  stable across restarts in development.
- **Abuse resistance**: per-email sliding-window rate limiting on
  magic-link issuance (`MAX_REQUESTS` = 5 per `WINDOW` = 5 min) bounds
  email-bombing and account-probing without breaking anti-enumeration.
  Backed by Postgres (the `auth_rate_limits` table) under a per-key
  advisory lock, so the quota is exact and shared across
  horizontally-scaled instances.

## 8. Architecture

loco.rs `App` (`src/app.rs`) registers the `auth` and `jwks`
controllers and the Postgres-backed worker queue. Token crypto is a
self-contained module (`src/auth`) using `jsonwebtoken` (RS256) and
`rsa` (to derive the JWK modulus/exponent). The bearer extractor is a
plain Axum `FromRequestParts`, so peer services can reuse the same
verification approach.

**Key set & rotation.** `auth::AuthKeys` holds a **set** of keys: one
primary signing key plus zero or more additional verify-only public
keys (loaded from `JWT_ADDITIONAL_PUBLIC_KEY_FILES` /
`JWT_ADDITIONAL_PUBLIC_KEY_PEMS`; unset ⇒ just the primary,
backward-compatible). `sign_access_token` signs with the primary and
stamps its `kid`; `verify_token` selects the verifying key by the token
header `kid` from {primary} ∪ {additional}; the JWKS publishes all keys
(primary first). This enables operator-driven, **zero-downtime key
rotation** — see the entity spec §8.4 runbook and
[`config/keys/README.md`](../config/keys/README.md). No auto-rotation
scheduler (follow-up).

## 9. API surface

See §6. Responses are raw loco JSON (no envelope). Errors use loco's
standard error responses (`401` unauthorized, `400` bad request, `429`
too many requests on throttled issuance).

The API is described by a hand-written **OpenAPI 3.0.3** document
(`src/openapi.rs`; the family authors these by hand rather than via
`utoipa`). It is served by the docs controller
(`src/controllers/docs.rs`) at `GET /api-docs/openapi.json`, with a
Swagger UI page at `GET /swagger-ui` (CDN assets). The document covers
every endpoint plus the `SignupParams` / `MagicLinkParams` /
`LoginResponse` / `CurrentResponse` / `Claims` / `Jwks` / `Jwk` /
`AuthEvent` / `AccountExport` (+ `AccountUserExport` /
`AccountSessionExport` / `AccountAuditExport`) schemas, the `429`
rate-limit responses, and a bearer `securityScheme` applied to the `me`
+ `signout` + `account/export` + `account/audit` + `account` (DELETE)
endpoints. Un-gated `spec()` unit tests pin its well-formedness, the
documented paths, the bearer scheme, and the schemas.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations:
`m20220101_000001_users`, `m20220101_000002_sessions`,
`m20220101_000003_auth_events`, `m20220101_000004_users_deleted_at` (the
GDPR-erasure soft-delete column), and `m20220101_000005_auth_rate_limits`
(the magic-link rate-limiter window log). `auto_migrate` is on in
development, off in production.

## 11. Testing strategy

- **Unit (DB-free):** `src/auth` — sign/verify roundtrip, JWKS shape,
  tampered/garbage-token rejection, and **key rotation**: a single-key
  set is byte-for-byte backward-compatible (same `kid`), a multi-key set
  publishes all keys (primary first), a token signed by a now-additional
  (verify-only) key still verifies via `kid` lookup, an unknown `kid` is
  rejected, duplicate additional keys are de-duplicated, and inline-PEM
  splitting works. Built deterministically via the `load_from(...)` test
  constructor (no env mutation). Run with `cargo test --lib`.
- **Request tests:** loco's `tests/requests/auth.rs` exercises the §6
  magic-link surface (signup / magic-link / redeem incl. single-use and
  anti-enumeration / me / signout / JWKS). The HTTP tests require a
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
- **Cross-crate contract test (DB-free):** `tests/sign_verify_contract.rs`
  pins the convention shared with
  [`authentication-verifier`](../../authentication-verifier-rust-crate/index.md):
  a token signed by `auth::sign_access_token` verifies through the
  verifier crate built from this service's published JWKS; the claims
  round-trip and `kid` = base64url(SHA-256(modulus)) holds; a `kid`
  mismatch fails. A **multi-key** case asserts a verifier built from a
  JWKS carrying more than the primary key still verifies a primary-signed
  token and rejects a token whose `kid` is absent from the set.

## 12. Compliance

Email is personal data: minimise, never log tokens alongside avoidable
PII in production, and honour the family's GDPR posture. Sessions give
an audit trail of issuance and revocation; `auth_events` is the durable
authentication audit trail.

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

**Audit gating decision.** The system-wide
`GET /api/auth/audit/recent` is left **unauthenticated** (family
convention; mirrors the sibling care-pathway `/audit/recent`; rows carry
no tokens or secrets). The GDPR right-of-access requirement is met
instead by the bearer-gated per-subject `GET /api/auth/account/audit`,
so a subject's own audit trail (and export) is reachable **only** by
that subject, while the operator-facing system feed stays open.

## 13. Tasks (live work queue)

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
- [ ] Optional Mailpit docker-compose service for realistic dev email.

## 14. Implementation status

Done: real loco scaffold; passwordless magic-link flow; RS256 signing;
JWKS endpoint; sessions + signout; console magic links; Postgres queue;
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
JWKS publishes all keys; `verify_token` selects by `kid`).

## 15. Roadmap

v0.1 (here): core magic-link + RS256/JWKS + signout, reworked request
tests, peer-service verifier + contract test, operator-driven key
rotation (multi-key set). v0.2: Mailpit, auto-rotation scheduler. v0.3:
begin loco conversion of the sibling services using this as the template
(peers adopt `authentication-verifier`).

## 16. Open questions

- Refresh tokens vs. short-lived access tokens only? (Currently access
  only.)
- Should revocation propagate to peers (e.g. a short-TTL deny-list
  endpoint) or stay local + rely on short TTLs?
- Audience model when peers need distinct audiences.

## 17. References

- loco.rs — https://loco.rs/
- RFC 7519 (JWT), RFC 7517 (JWK), RFC 7518 (JWA / RS256).

## 18. Change control

Update this spec in the same PR as any behavioural change. Bump
`CHANGELOG.md` under `[Unreleased]`.
