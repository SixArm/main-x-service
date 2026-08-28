## 13. Tasks

Live entity-level work queue. Tasks that belong to one subproject's
internals should migrate into that crate's spec §13; they are listed
here while the crate specs are thin. Each task has an acceptance
criterion; tick the box when an automated test or clearly described
manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [x] **T-1 — Stand up the trio (CRUD + matching).**
  - [x] matcher: `Case` type + enums; deterministic + probabilistic
    matching with per-component breakdown; presets.
  - [x] service: loco.rs chassis, `cases` table, CRUD with soft delete,
    `/match`, `/check-duplicates`.
  - [x] front-end: `/`, `/new`, `/[pid]`, `/[pid]/edit` over the REST
    API.
  - **Acceptance:** create → read → match → check-duplicates round-trip
    works end to end.
- [x] **T-2 — Validation (`422`).**
  - [x] Blank `title` → `422` on create and update (family convention).
  - [x] `opened_date` ISO-format check; blank identifier `value`; blank
    `subjects` / `keywords` entries. All problems reported in one
    `422` (`src/validation.rs`).
  - **Acceptance:** unit tests + request tests post each bad shape and
    get `422`; `400` stays for malformed bodies.
- [x] **T-3 — Audit log + event streaming.**
  - [x] `audit_logs` table + best-effort row per create/update/delete/
    merge (action + JSON snapshot + `actor`); read at
    `/audit/recent`, `/{pid}/audit`.
  - [x] In-memory `CaseEvent` ring buffer (cap 1 000); `created`/
    `updated`/`deleted`/`merged` published; read at `/events/recent`.
  - **Acceptance:** integration test creates + updates + deletes a case
    and reads back the audit rows and events; streaming pinned un-gated
    by `streaming::publish_and_read_back`.
- [x] **T-4 — Request-level integration tests (PostgreSQL).**
  - [x] loco testing harness over CRUD, `/search`, `/match`,
    `/check-duplicates`, `/merge`, audit/events, `whoami`, OpenAPI.
  - **Acceptance:** `cargo test -- --ignored` with a Postgres URL
    covers every endpoint, including a stored near-duplicate round-trip.
    (`#[ignore]`-gated so default `cargo test` stays DB-free.)
- [x] **T-5 — Front-end tests.**
  - [x] vitest units for `ApiClient` + `CaseRepository` (incl. a
    `check-duplicates` path regression).
  - [x] Playwright smoke over the four routes (API stubbed, runs on
    `vite preview`).
  - **Acceptance:** both suites run and fail on a broken endpoint
    contract.
- [x] **T-7 — Token verification (partial).**
  - [x] Verify tokens offline against the auth-service's published key via
    the embedded `authentication-verifier` (`src/auth.rs`), built from
    `CASE_PASETO_KEYS` / `CASE_TOKEN_ISSUER` / `CASE_TOKEN_AUDIENCE`.
    `AuthUser` (required) and `MaybeAuthUser` (optional) extractors;
    `/whoami` protected; audit / merge `actor` stamped from the token.
  - **Acceptance:** no token → `401`; valid signed token → `2xx`
    (un-gated crypto unit tests mint a token + matching key in-process).
  - [x] Switch the credential from RS256 JWT to PASETO v4 public
    (Ed25519) per
    [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
    (source of truth; supersedes the RS256-JWT + JWKS model): verifier
    consumes the auth-service's published Ed25519 key(s)
    (`Verifier::from_paseto_keys_value` / `from_paseto_keys_url`); same
    `Claims` shape (`kid`/`iss`/`aud`/`exp`; footer carries `kid`);
    un-gated unit tests mint a real PASETO v4 public token + matching
    Ed25519 key in-process.
  - [ ] *Follow-up:* blanket enforcement on every `/api/*` route (awaits
    the coordinated family SSO rollout; the front-end BFF attaches the
    bearer token server-side) and paseto-keys-over-HTTP fetch from the
    auth service at boot (currently injected via env).
- [x] **T-8 — Record merge.**
  - [x] `POST /merge` folds a duplicate into a survivor (union list
    fields, former-title alias, soft-delete the duplicate,
    `merge_records` history + snapshot, `Merged` + `Deleted` events);
    equal pids → `422`, unknown → `404`; `/merges/recent` history.
    Pure `src/merge.rs`.
  - **Acceptance:** integration test merges two stored cases and
    verifies survivor contents + soft-deleted duplicate; merge logic
    pinned un-gated.
  - [ ] *Follow-up:* a front-end merge action from the duplicates list
    (T-5 / T-11 territory).
- [x] **T-9 — OpenAPI / Swagger.**
  - [x] Hand-written `src/openapi.rs` (the matcher's `Case` shape is the
    API DTO and is dependency-light, so the schema is authored by hand,
    not utoipa-derived) served at `/api-docs/openapi.json` + `/swagger-ui`.
  - **Acceptance:** Swagger UI serves every documented endpoint;
    `openapi::spec` unit tests assert well-formedness + endpoint
    coverage.
  - [ ] *Follow-up:* deeper validation — docket / case-number format
    checks, status-transition rules, terminology checks.

### Delivered since the last pass (2026-08 professionalization audit)

The three tasks below were found still marked open even though the
crate CHANGELOG shows them shipped weeks earlier. Closed here with
their actual landing dates and implementing files, grounded against
`case/case-service-with-loco/src/` and `CHANGELOG.md`, not against the
stale prose that used to sit in this file.

- [x] **T-6 — Search + candidate blocking.** *(done 2026-08-02)*
  - [x] Name/title search endpoint. **Superseded, not just done:** the
    original `GET /api/cases/search?q=` (Postgres `ILIKE`) was replaced
    2026-08-02 by Tantivy full-text/fuzzy/phonetic search — see below.
  - [x] Make the `check-duplicates` in-memory scan cap a named,
    documented const (`CHECK_DUPLICATES_SCAN_CAP` = 1 000) with a
    `tracing::warn!` on hit. *(superseded — see next line)*
  - [x] Tantivy full-text / fuzzy search over the JSONB payload.
    **Done 2026-08-02** (repo `tasks.md` S-3) — new `src/search/`
    (`CaseIndexSchema`/`CaseIndex`/`SearchEngine`: `search` /
    `fuzzy_search` / `phonetic_search` / `search_page` / `candidates`);
    `GET /api/cases/search?q=` is Tantivy-backed with `?fuzzy=true` and
    `?phonetic=true`; `503` (never a silent empty result) when the
    index is unavailable. `tasks/search.rs` adds a `search_reindex` CLI
    task plus rebuild-if-empty on boot.
  - [x] Replace the 1 000-row in-memory scan in `check-duplicates` with
    search-blocked candidates (NFR-1 / NFR-2; OQ-2). **Done
    2026-08-02** — `POST /api/cases/check-duplicates` now scores a
    blocked candidate set (fuzzy title, exact identifier, phonetic
    title — up to 200) from the Tantivy index instead of the capped
    in-memory scan, closing the scale cliff where a duplicate past row
    1000 was unreachable.
  - **Acceptance:** DB-gated `search_reaches_secondary_fields_and_tolerates_typos`
    and `check_duplicates_blocks_on_identifier_alone` pin the index-backed
    behaviour. *(The literal "1,000,000 stored cases" latency benchmark
    from the original acceptance line was never run as a discrete test;
    what changed is that `check-duplicates` no longer scans all stored
    rows, so it no longer scales with total row count the way the old
    capped scan did — see `benches/service_bench.rs`'s `search` group
    for the measured indexing/retrieval cost instead.)*

- [x] **T-10 — Privacy: per-field masking + GDPR data-subject export.**
  *(done 2026-08-02; case data is personal data, §12)*
  - [x] Masked-view endpoint (`GET /api/cases/{pid}/masked`) applying
    per-field masking rules. **Done** — always-masked view via
    `mask_case` (`src/controllers/cases.rs`), redacting `subjects` /
    `identifiers` / `same_as` / `case_number`. (`mask_case` itself, and
    its wiring to the ABAC `mask` obligation on the native
    `GET /{pid}`, actually predates this task closure — it landed with
    the record-level ABAC work, 2026-07-05; what was missing until
    2026-08-02 was this always-masked view and the export below.)
  - [x] GDPR data-subject export (`GET /api/cases/{pid}/export`) and a
    subject-scoped export across cases sharing a `subjects` id.
    **Single-case export done, multi-case subject-scoped export not
    built:** `export_case` (`src/controllers/cases.rs`) returns
    `{entity, pid, exported_at, masked, record, note}`, masked when the
    record-level ABAC decision carries the `mask` obligation, and
    audited via `disclosure::action::EXPORT` (HIPAA §164.528
    accounting). A dedicated "every case sharing this `subjects` id"
    export is not implemented; the bulk-export machinery (BLK-5, §8.7
    of the crate spec) exports by list/search filter, not by subject id
    specifically — left as a documented gap rather than silently
    dropped.
  - [x] A GDPR-erasure path layered on soft delete (retention policy).
    **Exists, landed under a different task/date:**
    `POST /api/cases/{pid}/erase` (GDPR Art. 17, `access=admin`,
    `src/compliance/erasure.rs`) shipped 2026-07-25..27 as part of the
    compliance suite (crate spec §12.0/§12.0.1), not originally tracked
    under T-10 — ticked here because the capability is real, not
    because it was delivered under this task ID.
  - **Acceptance:** masked view hides the configured fields (pinned,
    `tests/masking.rs`); export returns a complete, machine-readable
    record set for the subject of one case (pinned,
    `tests/export_masking.rs`,
    `tests/requests/cases.rs::masked_view_and_export_are_served`) —
    **not** pinned for the cross-case "all cases sharing a subject"
    shape, which is not built; erasure is auditable
    (`src/compliance/erasure.rs`, DB-gated tests).

- [x] **T-12 — Durable event bus.** *(done: Phase 1 pre-existing,
  Phase 2 outbox 2026-07-09, Phase 3 real-broker sink 2026-08-03)*
  - [x] Replace the in-process ring buffer with a durable broker so peer
    registries and analytics can subscribe across replicas. **Done** —
    `src/streaming.rs` (versioned `Envelope` + `EventPublisher`/
    `EventSink` seam), `models/event_outbox.rs` (Phase 2 transactional
    outbox; the `audit_logs` write joins the same transaction as the
    entity mutation and the outbox row as of 2026-07-09, so the three
    can never disagree), `src/relay.rs` (Phase 3 relay loop +
    `FluvioSink`; default-off via `CASE_EVENT_TRANSPORT=memory` and the
    feature-gated `fluvio` Cargo feature, landed 2026-08-03, BUS-1).
  - **Acceptance:** events survive a replica restart and are delivered
    cross-replica. **Partially met, honestly:** the outbox row is
    durable in Postgres (survives a replica restart) and `FluvioSink`
    is wired to deliver cross-replica once a broker is configured; the
    live-broker delivery path itself has no automated execution in this
    repo's CI — `tests/fluvio_relay.rs` is a `#[ignore]`d round-trip
    verified by compiling under `--features fluvio`, not by running
    against a real broker (`compose.fluvio.yaml` provisions one for
    opt-in manual runs only). No deployment yet points
    `CASE_FLUVIO_ENDPOINT` at a live broker (family capability matrix,
    `agents/share/overview.md`).

### Open / deferred

- [ ] **T-11 — Front-end search box + audit / event views.**
  - [ ] A search box on `/` calling `GET /api/cases/search?q=`.
  - [ ] Audit-trail and event views (consume `/{pid}/audit`,
    `/audit/recent`, `/events/recent`).
  - **Acceptance:** the UI surfaces search results and a case's audit
    trail.
- [x] **T-13 — Thicken crate docs.** *(closed as won't-do, 2026-08
  professionalization audit)*
  - [x] ~~Add a service `agents/` reference set (`models.md`,
    `matching.md`, `restful.md`, `testing.md`,
    `spec-driven-development.md`) matching the sibling shape~~ —
    **won't do.** The root `AGENTS.md` "Subprojects" section states
    this as a deliberate decision, not an oversight: the six original
    entity crates carry an `agents/` reference set, but "[t]wenty-two
    newer subprojects do not, and that is a decision rather than a
    gap — those files restate what the spec already says, and a
    restatement that nobody regenerates is exactly the drift the SDD
    discipline exists to prevent." Confirmed
    `case/case-service-with-loco/` has no `agents/` directory today
    (only the single `AGENTS.md` entry point, which is the newer
    pattern) — that absence is correct per policy, and adding the
    directory this task asked for would be the regression, not the fix.
  - [x] ~~split any single-file crate specs into the numbered
    layout~~ — **moot.** The crate spec
    (`case/case-service-with-loco/spec/index.md`) already carries the
    full §1–§18 numbered structure; it is one file by choice, and
    root `AGENTS.md`'s "Two spec shapes exist" table fixes the
    numbering as what matters for this shape, not a one-file-per-section
    split.
  - **Acceptance (superseded):** N/A — task rescoped to won't-do rather
    than closed by a file that was never going to be added.
