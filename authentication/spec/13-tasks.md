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
    (`authentication-service-rust-crate/tests/sign_verify_contract.rs`;
    the verifier is a dev-dependency of the service).
  - **Acceptance met:** the claims round-trip through both crates in
    one DB-free, un-gated test; a `kid` mismatch fails with
    `UnknownKid`; the `kid = base64url(SHA-256(modulus))` thumbprint
    is recomputed independently.
- [ ] **T-5 — Key rotation.**
  - [ ] Service: publish multiple JWKS entries (`kid` already stamped)
    with a grace window ≥ max token TTL.
  - [ ] Verifier: document (or implement) refetch-on-`UnknownKid`.
  - **Acceptance:** tokens signed with the previous key verify during
    the grace window; after it, they are rejected.
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
- [ ] **T-9 — GDPR subject-rights workflow for accounts.**
  - [ ] Export (Art. 15) and erasure (Art. 17) paths for `users` +
    `sessions`.
  - **Acceptance:** documented endpoints or runbook; erasure removes
    or anonymises the email.
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
