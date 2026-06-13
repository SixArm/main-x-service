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
  `email_verified_at`, magic-link columns, audit timestamps. `password`
  exists only to satisfy `NOT NULL` and holds an unusable random hash.
- **sessions** — `jid` (unique, = token `jti`), `user_pid`,
  `expires_at`, `revoked_at`, `user_agent`. One row per issued token;
  the unit of revocation.

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
   The window is measured with a monotonic `Instant` clock (no wall-clock
   / env coupling); `src/rate_limit.rs` is pure and unit-testable via a
   clock-injecting `check_at(key, now)` and a `reset()` test helper.

8. `GET /api-docs/openapi.json` + `GET /swagger-ui` — the hand-written
   OpenAPI 3 document and a Swagger UI page.

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
  In-memory, process-wide (single-instance MVP); a shared store
  (Redis/DB) would be needed for horizontal scaling.

## 8. Architecture

loco.rs `App` (`src/app.rs`) registers the `auth` and `jwks`
controllers and the Postgres-backed worker queue. Token crypto is a
self-contained module (`src/auth`) using `jsonwebtoken` (RS256) and
`rsa` (to derive the JWK modulus/exponent). The bearer extractor is a
plain Axum `FromRequestParts`, so peer services can reuse the same
verification approach.

## 9. API surface

See §6. Responses are raw loco JSON (no envelope). Errors use loco's
standard error responses (`401` unauthorized, `400` bad request, `429`
too many requests on throttled issuance).

The API is described by a hand-written **OpenAPI 3.0.3** document
(`src/openapi.rs`; the family authors these by hand rather than via
`utoipa`). It is served by the docs controller
(`src/controllers/docs.rs`) at `GET /api-docs/openapi.json`, with a
Swagger UI page at `GET /swagger-ui` (CDN assets). The document covers
all six endpoints plus the `SignupParams` / `MagicLinkParams` /
`LoginResponse` / `CurrentResponse` / `Claims` / `Jwks` / `Jwk` schemas,
the `429` rate-limit responses, and a bearer `securityScheme` applied to
the `me` + `signout` endpoints. Un-gated `spec()` unit tests pin its
well-formedness, the documented paths, the bearer scheme, and the
schemas.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations:
`m20220101_000001_users`, `m20220101_000002_sessions`. `auto_migrate` is
on in development, off in production.

## 11. Testing strategy

- **Unit (DB-free):** `src/auth` — sign/verify roundtrip, JWKS shape,
  tampered/garbage-token rejection. Run with `cargo test --lib`.
- **Request tests:** loco's `tests/requests/auth.rs` exercises the §6
  magic-link surface (signup / magic-link / redeem incl. single-use and
  anti-enumeration / me / signout / JWKS). The HTTP tests require a
  Postgres instance (standard loco) and are `#[ignore]`d so plain
  `cargo test` stays green; run them with `cargo test -- --ignored`.
  DB-free route-table and params-contract assertions always run.
- **Rate-limit unit tests (DB-free):** `src/rate_limit.rs` — allow up to
  `MAX_REQUESTS`, reject the next, window reset, sliding-window single-slot
  release, per-key isolation, normalised-key sharing, non-consuming
  rejection (clock injected via `check_at`, serialised over the shared
  store). The DB-gated `magic_link_issuance_is_rate_limited` request test
  asserts the `(MAX_REQUESTS+1)`th magic-link POST returns `429`.
- **OpenAPI unit tests (DB-free):** `src/openapi.rs` `spec()` — well-formed,
  documents every endpoint, the bearer scheme is present + applied, core
  schemas exist. The docs route table is asserted in
  `tests/requests/auth.rs`.
- **Cross-crate contract test (DB-free):** `tests/sign_verify_contract.rs`
  pins the convention shared with
  [`authentication-verifier`](../../authentication-verifier-rust-crate/index.md):
  a token signed by `auth::sign_access_token` verifies through the
  verifier crate built from this service's published JWKS; the claims
  round-trip and `kid` = base64url(SHA-256(modulus)) holds; a `kid`
  mismatch fails.

## 12. Compliance

Email is personal data: minimise, never log tokens alongside avoidable
PII in production, and honour the family's GDPR posture. Sessions give
an audit trail of issuance and revocation.

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
- [ ] Key rotation: support multiple JWKS entries (`kid` already
      stamped) and a grace window.
- [ ] Optional Mailpit docker-compose service for realistic dev email.

## 14. Implementation status

Done: real loco scaffold; passwordless magic-link flow; RS256 signing;
JWKS endpoint; sessions + signout; console magic links; Postgres queue;
green `cargo build`, clippy clean, DB-free unit tests passing;
magic-link request tests (Postgres-gated); peer-service verifier crate
(`../authentication-verifier-rust-crate/`) with a DB-free cross-crate
contract test; per-email rate limiting on magic-link issuance (`429`);
hand-written OpenAPI 3 + Swagger UI; durable `auth_events` audit trail
(`GET /api/auth/audit/recent`).

## 15. Roadmap

v0.1 (here): core magic-link + RS256/JWKS + signout, reworked request
tests, peer-service verifier + contract test. v0.2: key rotation,
Mailpit. v0.3: begin loco conversion of the sibling services using this
as the template (peers adopt `authentication-verifier`).

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
