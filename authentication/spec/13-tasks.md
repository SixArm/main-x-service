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
- [ ] **T-2 — Register the verifier in the family indexes.**
  - [ ] Add `authentication-verifier-rust-crate` to the root
    `AGENTS.md` subproject tables and `agents/share/overview.md`
    (today neither mentions it). *(Root docs are outside this
    entity's write scope — pending a root-level pass.)*
  - [x] Tick the service spec §13 item "a reusable verifier
    crate/snippet for peer services" — the crate now exists.
    *(2026-06-13: ticked with a pointer; service README/AGENTS also
    point peers at the verifier.)*
  - **Acceptance:** root docs list the crate; service spec §13 is
    consistent with reality (second half met).
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
- [ ] **T-6 — Rate limiting / abuse resistance for magic-link issuance.**
  - [ ] Per-email and per-IP issuance limits; backoff on repeated
    requests.
  - **Acceptance:** integration test shows the N+1th request inside
    the window is throttled while the always-`200` anti-enumeration
    shape is preserved.
- [ ] **T-7 — Localise user-facing emails and UI.**
  - [ ] Mailer templates (`magic_link`, `welcome`) and front-end
    strings per [`agents/share/locales.md`](../../agents/share/locales.md).
  - **Acceptance:** a locale switch produces a translated magic-link
    email and UI.
- [ ] **T-8 — OpenAPI documentation for the service API.**
  - [ ] Document FR-1…FR-8 endpoints (sibling services ship Swagger).
  - **Acceptance:** an OpenAPI 3.0 document describing all six
    endpoints is served or committed.
- [ ] **T-9 — GDPR subject-rights workflow for accounts.**
  - [ ] Export (Art. 15) and erasure (Art. 17) paths for `users` +
    `sessions`.
  - **Acceptance:** documented endpoints or runbook; erasure removes
    or anonymises the email.
- [ ] **T-10 — Authentication event audit trail.**
  - [ ] `audit_log`-style records (or event streaming) for sign-in
    attempts, redemptions, and signouts, aligned with
    [`agents/share/auditability.md`](../../agents/share/auditability.md).
  - **Acceptance:** a redeemed magic link produces a queryable audit
    record with user, timestamp, and outcome.
- [ ] **T-11 — Front-end test suite.**
  - [ ] Vitest unit tests (`ApiClient`, `AuthRepository`); playwright
    smoke for the four routes. (Mirrors front-end spec §13.)
  - **Acceptance:** `pnpm run test` and `pnpm run test:e2e` pass in CI.
