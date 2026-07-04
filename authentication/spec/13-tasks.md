## 13. Tasks

Spec-driven work breakdown for the **entity level** (cross-subproject
contract + documentation). Crate-internal tasks live in each
subproject's own spec §13. Each task has an acceptance criterion; tick
the box when an automated test or clearly described manual check
confirms it. Split larger tasks (`T-5a`, `T-5b`).

- [x] **T-1 — Give the verifier crate its doc set.** *(2026-06-13)*
  - [x] `spec/` in the house §1–§18 shape
    ([spec/index.md](../authentication-verifier-rust-crate/spec/index.md)).
  - [x] `README.md` and `CHANGELOG.md` — `Cargo.toml` declares
    `readme = "README.md"` and lists both in `include`; both now exist
    and `cargo package` works again.
  - [x] `AGENTS.md` / `CLAUDE.md` per the per-crate doc-set convention
    (plus `index.md`).
  - **Acceptance met:** `cargo package --list --allow-dirty` succeeds
    in `authentication-verifier-rust-crate` and lists `README.md` and
    `CHANGELOG.md`.
- [x] **T-2 — Register the verifier in the family indexes.** *(2026-06-13)*
  - [x] Add `authentication-verifier-rust-crate` to the root
    `AGENTS.md` subproject tables and `agents/share/overview.md`. Both
    now carry a new **Library crates** section listing the verifier as
    a peer-side offline RS256 JWT-verification library published to
    crates.io as `authentication-verifier` (0.1).
  - [x] Tick the service spec §13 item "a reusable verifier
    crate/snippet for peer services" — the crate now exists.
    *(2026-06-13: ticked with a pointer; service README/AGENTS also
    point peers at the verifier.)*
  - **Acceptance met:** root `AGENTS.md` + `agents/share/overview.md`
    list the crate (Library crates section); the verifier's own
    `index.md`/`README.md` cross-reference the service; service spec
    §13 is consistent with reality.
- [x] **T-3 — Rework service request tests for the magic-link surface.**
  *(2026-06-13)*
  - [x] Replace the generated password-flow tests in
    `tests/requests/auth.rs` + snapshots with signup / magic-link /
    redeem / me / signout / JWKS coverage.
  - **Acceptance met:** `cargo test -- --ignored` (with Postgres)
    exercises every FR-1…FR-8 endpoint; the Postgres-backed tests are
    `#[ignore]`d so plain `cargo test` stays green without a database.
    (Mirrors service spec §13.)
- [x] **T-4 — Cross-crate contract test (service signs, verifier verifies).**
  *(2026-06-13)*
  - [x] Test that builds a `Verifier` from the service's published
    JWKS document and verifies a token signed by
    `auth::sign_access_token`
    (`authentication-service-with-loco/tests/sign_verify_contract.rs`;
    the verifier is a dev-dependency of the service).
  - **Acceptance met:** the claims round-trip through both crates in
    one DB-free, un-gated test; a `kid` mismatch fails with
    `UnknownKid`; the `kid = base64url(SHA-256(modulus))` thumbprint
    is recomputed independently.
- [x] **T-5 — Key rotation.** *(2026-06-13)*
  - [x] Service: `AuthKeys` is now a **key set** — one primary signing
    key plus zero or more additional verify-only public keys, loaded from
    `JWT_ADDITIONAL_PUBLIC_KEY_FILES` / `JWT_ADDITIONAL_PUBLIC_KEY_PEMS`
    (unset ⇒ single primary, fully backward-compatible). `sign_access_token`
    signs with the primary and stamps its `kid`; `verify_token` selects the
    verifying key by the token header `kid` from {primary} ∪ {additional};
    the JWKS publishes all keys (primary first). `kid` stays
    `base64url(SHA-256(modulus))` for every key. A `load_from(...)` test
    constructor builds deterministic multi-key sets without env mutation.
    Operator rotation runbook documented in service spec §8.4 +
    `config/keys/README.md`.
  - [x] Verifier: already selects by `kid` and documents
    refetch-on-`UnknownKid` (the multi-key JWKS contract is now pinned by
    `tests/sign_verify_contract.rs::multi_key_jwks_verifies_primary_and_rejects_unknown_kid`).
  - **Acceptance met:** un-gated unit tests
    (`src/auth/mod.rs`: backward-compat single key, JWKS publishes all
    keys primary-first, a token signed by a now-additional key still
    verifies, unknown-kid rejected, duplicate-key dedup, PEM splitting)
    plus a contract test that a verifier built from the full multi-key
    JWKS verifies a primary-signed token and rejects an unknown `kid`. The
    grace-window semantics hold by construction: an additional key is
    retired only after the max token TTL elapses, after which its tokens
    have expired and are rejected. No auto-rotation scheduler (follow-up).
- [x] **T-6 — Rate limiting / abuse resistance for magic-link issuance.**
  *(2026-06-13)*
  - [x] Per-email issuance limit with a monotonic-clock sliding window
    (`src/rate_limit.rs`): at most `MAX_REQUESTS` = 5 issuance requests
    per `WINDOW` = 5 minutes, keyed by a normalised (trimmed,
    lowercased) email. `Instant`-based (no wall-clock / env coupling);
    a `check_at(key, now)` core makes it deterministically testable and
    a `reset()` helper clears the process-wide store between tests.
  - [x] Wired into `POST /api/auth/signup` + `POST /api/auth/magic-link`
    *before* any account lookup or token issuance: over the limit
    returns `429` (`Error::CustomError(TOO_MANY_REQUESTS,
    ErrorDetail::new("rate_limited", …))`) with no mail sent; the
    success path keeps the always-`200` anti-enumeration shape because
    the `429` is keyed on request volume, not account existence.
  - **Acceptance met:** un-gated unit tests (8) prove allow-up-to-N /
    reject-N+1 / window-reset / sliding-window / per-key isolation /
    normalised-key sharing / non-consuming rejection; a DB-gated request
    test (`magic_link_issuance_is_rate_limited`) asserts the
    `(MAX_REQUESTS+1)`th magic-link POST for one email returns `429`
    after `MAX_REQUESTS` `200`s. Documented in service spec §6/§7.
- [ ] **T-7 — Localise user-facing emails and UI.**
  - [ ] Mailer templates (`magic_link`, `welcome`) and front-end
    strings per [`agents/share/locales.md`](../../agents/share/locales.md).
  - **Acceptance:** a locale switch produces a translated magic-link
    email and UI.
- [x] **T-8 — OpenAPI documentation for the service API.** *(2026-06-13)*
  - [x] Hand-written OpenAPI 3.0.3 document (`src/openapi.rs`, no
    `utoipa`, mirroring the care-pathway/case pattern) served by a docs
    controller (`src/controllers/docs.rs`) at
    `GET /api-docs/openapi.json` + `GET /swagger-ui` (CDN assets),
    registered in `app.rs`. Documents all six endpoints (signup,
    magic-link request, magic-link redeem, me, signout, JWKS) with the
    `SignupParams` / `MagicLinkParams` / `LoginResponse` /
    `CurrentResponse` / `Claims` / `Jwks` / `Jwk` schemas, the `429`
    rate-limit responses, and a bearer `securityScheme` applied to `me`
    + `signout`.
  - **Acceptance met:** the OpenAPI document is served at
    `/api-docs/openapi.json`; un-gated `spec()` unit tests (5) assert it
    is well-formed, documents every endpoint, carries the bearer scheme,
    and exposes the core schemas. Documented in service spec §9.
- [x] **T-9 — GDPR subject-rights workflow for accounts.** *(2026-06-13)*
  - [x] **Right of access (Art. 15) — account export.**
    `GET /api/auth/account/export` (bearer) returns a JSON document of
    everything the service holds about the authenticated subject: their
    `users` row (`pid`, `email`, `name`, `email_verified_at`,
    timestamps), their `sessions` (jid, issuance/expiry/revocation,
    user_agent), and their `auth_events` audit trail (matched by pid
    *or* email). Excludes the password hash, api key, and any token /
    key material (`models/users::find_active_by_pid`,
    `sessions::find_all_by_user_pid`, `auth_events::for_subject`;
    `views/auth::AccountExport`).
  - [x] **Right to erasure (Art. 17) — account deletion.**
    `DELETE /api/auth/account` (bearer): **soft-delete + anonymise** —
    new `users.deleted_at` column (migration
    `m20220101_000004_users_deleted_at`), `email`→`deleted+<pid>@invalid`
    tombstone (keeps `UNIQUE(email)`, RFC 2606 unroutable),
    `name`→`"deleted user"`, magic-link material cleared; **all sessions
    revoked**; an `account_erased` `auth_events` row written. The row
    survives so referential history + the audit trail keep integrity.
    Post-erasure the bearer token still verifies cryptographically until
    `exp`, but `/me` and the export treat the subject as gone (`401`,
    via `find_active_by_pid`). Idempotent.
  - [x] **Per-subject audit (T-10 follow-up).** Decision: leave the
    system-wide `GET /api/auth/audit/recent` **open** (family convention,
    mirrors care-pathway; rows carry no secrets) and add a bearer-gated
    per-subject `GET /api/auth/account/audit` returning only the caller's
    own events. So a subject's export + own audit are reachable **only**
    by that subject; the operator-facing system feed stays open.
  - **Acceptance met:** un-gated unit tests — anonymisation/tombstone
    transform (`models/users` tests, 3), export-document assembly +
    secret-exclusion (`views/auth` tests, 2), OpenAPI `spec()` for the
    new paths/schemas/bearer (`openapi.rs` tests, 3). DB-gated request
    tests (`tests/requests/auth.rs`): export returns the caller's data,
    erasure soft-deletes + anonymises + revokes sessions + writes the
    audit row, post-erasure `/me` + export are `401`, unauthenticated
    export/audit/delete are `401`. OpenAPI updated; spec §6/§9/§12/§14 +
    AGENTS updated. `cargo test` green (un-gated), clippy clean.
- [x] **T-10 — Authentication event audit trail.** *(2026-06-13)*
  - [x] Durable `auth_events` table (migration
    `m20220101_000003_auth_events`): `(id, event, email, user_pid,
    detail, created_at)`, aligned with
    [`agents/share/auditability.md`](../../agents/share/auditability.md).
    SeaORM entity (`models/_entities/auth_events.rs`) + model
    (`models/auth_events.rs`) with best-effort `record` /
    `record_best_effort` (never fails the request) and `recent`.
  - [x] Wired into signup, magic-link request (records the
    `rate_limited` / `unknown_email` / `issued` outcome without leaking
    which to the caller), redeem (`ok` vs `invalid_or_expired`), and
    signout. Anti-enumeration preserved: the audit row distinguishes
    outcomes, the HTTP response does not. No tokens or secrets stored.
  - [x] Read endpoint `GET /api/auth/audit/recent` (newest 100), left
    unauthenticated to mirror care-pathway's `/audit/recent` (noted in
    spec §12; bearer-gating tracked with T-9). OpenAPI documents the
    endpoint + the `AuthEvent` schema.
  - **Acceptance met:** un-gated unit tests (`normalise_email`; OpenAPI
    `documents_the_audit_endpoint_and_schema`); a DB-gated request test
    (`auth_events_are_recorded_and_queryable`) asserts a signup and an
    unknown-email magic-link request write the expected `auth_events`
    rows and that `/audit/recent` returns them. Documented in spec
    §6/§9/§10/§12.
- [x] **T-11 — Front-end test suite.** *(2026-06-13)*
  - [x] Vitest unit tests (`tests/unit/client.test.ts` +
    `tests/unit/auth.test.ts`, 16): `ApiClient` request shaping +
    bearer-token attachment + `ApiError` classification + raw-JSON /
    empty-body / non-JSON handling; `AuthRepository` exact path/verb/body
    for signup / magic-link request / verify (URL-encoded token) / me /
    signout. (Mirrors front-end spec §11/§13.)
  - [x] Playwright smoke (`tests/e2e/smoke.spec.ts`, 7) stubbing the auth
    API via `page.route` and loading sign-up / sign-in / verify /
    signed-in + signed-out home; `playwright.config.ts` runs against
    `vite preview` (build+preview, port 4173) per the care-pathway
    pattern. Also fixed a scaffold artifact (`src/app.html` meta
    description named the Course Service).
  - **Acceptance met:** `pnpm test` (16) and `pnpm test:e2e` (7) pass;
    `pnpm run check` 0/0; authored files prettier-clean.
- [ ] **T-12 — Pivot off JWT-for-sessions → cookie sessions + PASETO.**
  *(spec'd 2026-06-17; supersedes the RS256 JWT + JWKS model)* Adopts
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  across all three subprojects. **Core landed** (a–b, d–f); remaining:
  T-12c (full CSRF) and the T-12a sessions-table reshape:
  - [x] **T-12a — Service: cookie sessions (core).** Magic-link
    redemption creates a session row and sets the
    `__Host-mxi_session` cookie (HttpOnly/Secure/SameSite/`Path=/`);
    signout sets `revoked_at` + clears the cookie (shared §3, §7).
    *Remaining refinement:* reshape `sessions` to the shared-§3 schema
    (`sid` / `data` JSONB / `last_seen_at` / `idle_expires_at` /
    `absolute_expires_at`; today `sid` = the legacy `jid` column),
    idle-TTL sliding on `/me`, and `sid` rotation on privilege change;
    then drop the transitional PASETO body from redemption.
  - [x] **T-12b — Service: PASETO minting + key publication.**
    `POST /token` exchanges a valid session for a short-lived
    (~5 min) PASETO **v4.public** (Ed25519, claims §5.3, footer `kid`);
    the Ed25519 public key set is published at
    `/.well-known/paseto-keys`; seed loading via `TOKEN_PRIVATE_KEY_SEED`
    / `TOKEN_PRIVATE_KEY_FILE` (built-in dev seed otherwise). Crate:
    `rusty_paseto` + `ed25519-dalek` (`#![forbid(unsafe_code)]` holds).
  - [ ] **T-12c — Service: CSRF.** Per-session CSRF token
    (synchroniser / double-submit) on
    cookie-authenticated `POST`/`PUT`/`PATCH`/`DELETE` incl. `POST /token`,
    signout, `DELETE /api/auth/account` (shared §4). The
    `Origin`/`Referer` allow-list backstop (`AUTH_ALLOWED_ORIGINS`) is
    already in place.
  - [x] **T-12d — Service: remove RS256/JWKS.** RS256 signing,
    `GET /.well-known/jwks.json`, and the `jsonwebtoken`/`rsa` stack are
    removed; OpenAPI updated (shared §9 step 6).
  - [x] **T-12e — Verifier: PASETO support.**
    `Verifier::from_paseto_keys_value` / `from_paseto_keys_url` replaced
    the RS256 `from_jwks_*`; same `Claims`; footer-`kid` selection;
    `VerifyError` taxonomy updated (`Keys` / `Paseto`). Shipped as
    `authentication-verifier` 0.2.0 on crates.io.
  - [x] **T-12f — Front-end: BFF + remove `localStorage`.**
    Session-holding moved to the SvelteKit server (BFF:
    `hooks.server.ts` + server loads);
    browser holds only the cookie;
    `mxi.auth.token` / `mxi.auth.user` / `mxi_access_token` dropped
    (shared §6). *Remaining:* browser→BFF CSRF token (with T-12c) and
    restating the front-end test suites to the BFF model.
  - **Acceptance:** magic-link redemption returns `Set-Cookie:
    __Host-mxi_session` and no token; `POST /token` mints a PASETO that a
    verifier built from `/.well-known/paseto-keys` accepts and a peer
    verifies offline; signout sets `revoked_at`; no `/.well-known/jwks.json`
    and no RS256 path remain; the cross-crate contract test signs PASETO
    and verifies through the verifier; the front-end never stores a
    credential in JS.
