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
- [x] **E-8 — Audit adapter coverage of the matcher's national-ID schemes.**
  - [x] Table in §5.3.1 + `adapter.rs` rustdoc enumerating which
    identifier `system` URIs route to which matcher scheme slot, and
    which schemes are unreachable from service data today.
  - [x] Bridge tests for each newly routed scheme.
  - **Acceptance:** documented routing table matches
    `to_matcher_person` behaviour; `cargo test --test
    duplicate_detection` covers every routed scheme family.
  - *(Done 2026-06-13: the matcher exposes 26 national-ID builder
    slots; `route_identifier` reaches 14 (UK NHS, US SSN, BR CPF,
    FR NIR, ES TSI, IN Aadhaar, JP My-Number, MX CURP, SE
    personnummer, DE KVNR, NL BSN, NZ NHI, AU/IE IHI) via system-URI
    fast paths + the `tax_id`/SSN/TAX → us_ssn defaults; 12 remain
    unreachable (uk_hc_number, uk_chi_number, uk_nino, it_cf, bg_egn,
    es_dni, hr_oib, no_fnr, pl_pesel, ro_cnp, si_emso, cn_rrn).
    Routing table added to spec §5.3.1 and the adapter module rustdoc;
    `tests/duplicate_detection.rs` grew +3 tests
    (`routable_identifier_systems_reach_their_matcher_slot`,
    `ihi_disambiguates_au_vs_ie_by_digit_count`,
    `shared_cpf_system_uri_is_deterministic_match`) → 17 pass, 0
    fail.)*
  - *(Follow-up 2026-06-13: the 12 previously-unreachable slots are now
    routed via `system`-URI fast paths — `uk_hc_number` (`hc-number`/
    `health-and-care`), `uk_chi_number` (`chi-number`/`:chi`/`/chi`),
    `uk_nino` (`nino`/`national-insurance`), `it_cf` (`codice`/`it-cf`/
    `:cf`), `bg_egn` (`egn`), `es_dni` (`dni`), `hr_oib` (`oib`),
    `no_fnr` (`fnr`/`fodselsnummer`), `pl_pesel` (`pesel`),
    `ro_cnp` (`cnp`), `si_emso` (`emso`), `cn_rrn` (`rrn`). All **26**
    matcher slots are now reachable; the routing table moved to spec
    §5.3.1 and is pinned by a new bridge test
    `all_national_id_schemes_route_to_their_slot` that asserts each
    scheme routes **and** deterministic-matches on a shared well-formed
    value → `cargo test --test duplicate_detection` 18 pass, 0 fail.)*
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
    the service `CLAUDE.md`; left for a service-level task. **Resolved
    2026-06-13:** license links re-pointed to the `Cargo.toml` SPDX
    expression (no LICENSE files exist; crate is multi-licensed),
    `ARCHITECTURE.md`→`spec/08-architecture.md`,
    `API_GUIDE.md`→`AGENTS/restful.md`, `task-10.md`→`spec/13-tasks.md`,
    and the `CLAUDE.md` include re-pointed to
    `../../agents/share/architecture.md`. Link-checker over
    `person/**/*.md`: 508 relative links resolve, zero dangling.)*
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
