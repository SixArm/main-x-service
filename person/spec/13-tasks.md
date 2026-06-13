## 13. Tasks

Entity-level work queue: tasks that span subprojects or govern the
integration contract. Single-subproject work belongs in that
subproject's spec §13 (service) / §23 (matcher) / §13 (front-end) —
entries here may *reference* those tasks but not duplicate them. Each
task has an acceptance criterion; tick the box when an automated test
or clearly described manual check confirms it. Split oversized tasks
(`E-1a`, `E-1b`).

- [ ] **E-1 — SSO end-to-end via the authentication entity.**
  - [ ] Service: JWT-validator extractor on `/api/*`, verifying RS256
    against the authentication service's JWKS (service §13 T-1).
  - [ ] Front-end: sign-in redirect + bearer-token attachment in
    `ApiClient`; signed-out state on `401`.
  - [ ] This spec: record the verified-claims contract in §8.3.
  - **Acceptance:** golden-path integration suite runs signed-in;
    an unauthenticated `POST /api/persons` returns `401`.
- [ ] **E-2 — Unblock the front-end live integration suite.**
  - [ ] Diagnose the pre-existing service issue blocking
    `tests/integration/golden-paths.spec.ts` (front-end §16 OQ-5).
  - [ ] Land the service fix with its own three-part PR.
  - **Acceptance:** `bin/e2e` passes 9/9 against a locally running
    service.
- [ ] **E-3 — Durable event bus for downstream agencies.**
  - [ ] Service: production publisher behind a feature flag
    (service §13 T-2); document failover behaviour.
  - [ ] This spec: promote FR-15 from in-memory to durable wording.
  - **Acceptance:** integration test publishes `PersonCreated`
    end-to-end through a real broker.
- [ ] **E-4 — Complete the data-subject-rights path in the UI.**
  - [ ] Masked-view toggle on detail (front-end §13 T-19).
  - [ ] GDPR-export download button (front-end §13 T-20).
  - **Acceptance:** operator can export and view-masked a person
    without leaving the front-end; e2e test covers both.
- [ ] **E-5 — Consent management UI.**
  - [ ] Front-end routes for viewing / granting / revoking consent
    against the service's consent model.
  - [ ] Resolve service §16 OQ-3 (query-layer enforcement) first —
    the UI shape depends on it.
  - **Acceptance:** consent grant → revoke round-trip visible in the
    UI and in the audit log.
- [ ] **E-6 — Resolve SVAR DataGrid licensing for governmental use.**
  - [ ] Evaluate GPL-3.0 free tier vs Pro/Enterprise for a public
    deployment (front-end §13 T-21 / §16 OQ-1).
  - **Acceptance:** decision recorded here and in the front-end spec;
    grid swapped or license procured accordingly.
- [ ] **E-7 — Operator-UI localization.**
  - [ ] i18n scaffolding in the front-end; extract strings; ship at
    least one non-English locale from
    [`agents/share/locales.md`](../../agents/share/locales.md).
  - **Acceptance:** locale switch renders the persons list and create
    form fully translated; e2e test asserts no hard-coded English in
    those routes.
- [ ] **E-8 — Audit adapter coverage of the matcher's 42 schemes.**
  - [ ] Table in §5.3 (or `adapter.rs` rustdoc) enumerating which
    identifier `system` URIs route to which matcher scheme slot, and
    which schemes are unreachable from service data today.
  - [ ] Bridge tests for each newly routed scheme.
  - **Acceptance:** documented routing table matches
    `to_matcher_person` behaviour; `cargo test --test
    duplicate_detection` covers every routed scheme family.
- [x] **E-9 — Repair repo-root links broken by entity nesting.**
  - [x] Crate docs still link `../../agents/share/…` and
    `../../AGENTS.md`, which after nesting resolve inside `person/`
    and dangle; sibling-entity links (e.g.
    `../../worker-service-rust-crate/`) dangle likewise.
  - **Acceptance:** a link-checker pass over `person/**/*.md` reports
    no broken relative links. *(Done 2026-06-13: repo-root links
    re-pointed `../`→`../../` / `../../`→`../../../`, cross-entity
    links re-pointed under their entity dirs, `@agents/share/…`
    includes re-pointed, renamed shared docs updated
    (`rust-loco-stack.md`, `rust-tracing-opentelemetry-stack.md`,
    `loco.md`). Link-checker over the three subprojects: 288 relative
    links resolve; the only remaining broken links are pre-nesting
    rot in `person-service-rust-crate/README.md` + `index.md`
    pointing at never-committed files — LICENSE / LICENSE-MIT /
    LICENSE-APACHE / ARCHITECTURE.md / API_GUIDE.md / task-10.md —
    plus a pre-existing dangling `@AGENTS/architecture.md` include in
    the service `CLAUDE.md`; left for a service-level task.)*
- [ ] **E-10 — Regeneration / drift-check story for
  `person-service-schema.sql`.**
  - [ ] Decide whether the entity-root schema file is generated
    (e.g. `pg_dump --schema-only` after `migrate up`) or stays
    hand-maintained with a CI drift check against the migrations.
  - [ ] Reconcile known divergences: `person_consents` (schema file
    only; no migration), "PostgreSQL 15+" header vs the family
    standard PostgreSQL 18 (§10.2).
  - **Acceptance:** a documented, repeatable command reproduces the
    file from the migrations (or a CI check fails on drift), and the
    divergences above are either migrated or removed from the file.
