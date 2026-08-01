# Main X Index — Improvement Program Tasks

> Companion to [plan.md](plan.md) — read that first (themes, sequencing,
> working agreements, green gate). Execute tasks roughly in phase order;
> within a phase, respect `Depends`. Tick a task only when its gate is
> green **and** it is merged to `main`. Behavioural changes are
> three-part (crate spec §13 + code + tests) plus the crate CHANGELOG.
> Do **not** create plan/tasks files inside any crate.
>
> Sizes: S ≈ one focused session-slice · M ≈ a substantial slice or a
> delegated subagent build · L ≈ multiple slices, split before starting.

---

## Phase 1 — Truth & hygiene

- [x] **H-1 (S)** Pin the Rust toolchain. *(done 2026-07-12)*
  Add a repo-root `rust-toolchain.toml` (stable, pinned minor) so rustfmt
  drift cannot recur. Run `cargo fmt --check` across all crates after
  pinning; fix any residue in one `Style:` commit.
  *Verify:* `cargo fmt --check` clean in every crate listed by
  `find . -name Cargo.toml -not -path '*/target/*'`.
  *Result:* pinned `1.96.1` (+ rustfmt/clippy, minimal profile); all 37
  crates already `fmt --check`-clean, so no `Style:` fixup was needed.

- [x] **H-2 (M)** Make `agents/share/overview.md` honest. *(done 2026-07-15)*
  Replaced the overclaiming "What every crate provides" bullet list with a
  grounded **capability matrix**: a verified common baseline (CRUD, matching,
  merge, audit, in-memory events, REST/OpenAPI, PASETO verify + blanket ABAC,
  observability, Postgres) for the ten entity registries, then a ✅/– matrix
  of the nine capabilities that **vary** by crate, plus a note on the two
  cross-cutting services (auth, link-graph). Every cell grounded by grepping
  the tree for the live `src/` module — Tantivy + privacy: person/worker/
  place/thing/event/course (6); FHIR: those 5 (not course) + org/care-pathway/
  case (8); gRPC: person/worker/event (3); durable outbox: all but course (9);
  boundary normalization: person/worker/place/event (4); record-level ABAC +
  cross-service links: person/worker/case (3); bulk: person (1).
  *Verified:* every ✅ maps to an existing `src/` module (no overclaims).

- [x] **H-3 (M)** Rewrite `agents/share/architecture.md`. *(done 2026-07-15)*
  Replaced the pre-loco person-only layout (stale endpoint counts, a
  person-specific module tree, no loco/outbox/aggregator/auth) with a
  one-page family description: the 12-service family + libraries, one layered
  request-flow diagram, the **two internal shapes** (person-style
  `src/api/rest/` — person/worker/course + place/thing/event mid-conversion;
  loco-style `src/controllers/` matcher-DTO-as-JSONB — organization/
  care-pathway/case/portfolio), the cross-cutting subsystems (PASETO+ABAC
  auth stack, in-memory→outbox→Fluvio event bus, the link-graph aggregator),
  shared design patterns, and the create/merge/link data flows.
  *Verified:* every named file/module (person-style tree, loco-style tree,
  link-graph modules, migration location) checked to exist in the tree —
  fixed one stale reference (migrations are crate-root `migration/`, not
  `src/migration/`, except authentication).

- [x] **H-4 (M)** Roll CI `--include-ignored` to every service CI. *(done 2026-07-12)*
  Case's `.github/workflows/ci.yaml` test job is the pattern (Postgres
  service + `cargo test --all-features --all -- --include-ignored`).
  Apply to the other nine service crates' workflows (and link-graph).
  Check each service's `config/test.yaml` DB name matches its CI env.
  *Verify:* per-crate workflow lints (yamllint or careful review); the
  DB-gated suites at least compile locally (`cargo test --no-run`).
  *Result:* all 12 services now run `--include-ignored` against a Postgres
  service with a matching DB name. Category A (`ci.yaml` + PG: authentication,
  organization, portfolio) got `-- --include-ignored`; care-pathway's two
  test steps consolidated into one `--include-ignored` run; Category B
  (`test.yml`: person, worker, event) had `--test api_integration_test`
  replaced with `--all-features --all -- --include-ignored`; Category C
  (course, place, thing had no PG test job) got a new self-contained
  `test.yml`; link-graph (no workflow at all) got a full `ci.yaml`. All
  workflows YAML-validated; no `[features]` section anywhere so
  `--all-features` enables nothing new.

- [ ] **H-5 (M)** Release hygiene: cut CHANGELOG releases for crates with
  large `[Unreleased]` sections (person, case, care-pathway, organization,
  portfolio, link-graph, authentication-service, authentication-verifier,
  entity-ref); tag (`<crate>-vX.Y.Z`); decide/execute crates.io publish
  for `entity-ref` and `authentication-verifier` 0.8 (both are
  dependency-light and publishable).
  *Verify:* CHANGELOGs have dated release headings; tags pushed to both
  remotes.

## Phase 2 — Capability completion (four newest loco services)

- [x] **S-1 (L)** Tantivy full-text search in **organization**.
  *(done 2026-07-31)* `src/search/{index,mod}.rs` — schema (`pid`
  stored; name / legal name / alternate names / Soundex codes /
  identifiers / keywords / flattened address / url full-text;
  `jurisdiction` + `active` exact) and a `SearchEngine` facade behind a
  process-wide `OnceLock` keyed on `ORGANIZATION_SEARCH_INDEX_PATH`
  (default `data/search-index`). Index writes are wired into
  `src/streaming.rs` — the single seam both the native and the FHIR
  controllers write through — after the DB write is durable and
  best-effort (a failed index write is logged at `ERROR`, never fails a
  committed request). `GET /search` keeps `?q=` and gains
  `fuzzy` / `phonetic`; `check-duplicates` now **blocks** on the index
  (≤ 200 candidates) instead of scanning 1000 rows, so a duplicate's
  reachability depends on similarity, not insertion order. Rebuild:
  `cargo loco task search_reindex` plus an automatic boot rebuild when
  the index is empty and the table is not
  (`ORGANIZATION_SEARCH_BOOT_REINDEX=0` opts out).
  *Verified:* `cargo fmt --check` + `clippy --all-targets -D warnings`
  clean; **127** DB-free tests; **22** DB-gated tests green vs
  Postgres 18. The new DB-gated tests were mutation-checked (disabling
  indexing fails 6 of them), which is how the first version of the
  boot-rebuild test was caught passing vacuously — it raced the boot
  hook's own background rebuild, so the rebuild is now split into an
  awaitable `reindex_if_empty` (tested directly) and a thin `spawn_`
  wrapper the request suite switches off.

  Three decisions worth not re-litigating:
  - **The index is a candidate generator, never a source of truth.**
    Every hit is resolved against Postgres and soft-deleted rows do not
    resolve, so a stale index degrades (a missing hit) rather than
    corrupts (it can never resurrect or leak a deleted record).
  - **A broken index is `503`, not an empty result.** Especially on
    `check-duplicates`: answering "no duplicates" from an unopenable
    index would let a caller create a duplicate believing it had been
    checked.
  - **The `ILIKE` search and its `escape_like` guard were deleted, not
    left dormant.** The crate now issues no `LIKE` query, and an
    escaper with no caller invites a future caller to assume it is
    still wired in. care-pathway / case keep theirs — they still use
    `ILIKE`.

  Not done here: the FHIR `GET /fhir/Organization` search is a
  structured multi-parameter filter over a capped scan, not a free-text
  query; moving it onto the index is a separate item.

- [ ] **S-2 (L)** Tantivy in **care-pathway** (as S-1). Depends: S-1
  (copy its loco-adapted pattern, not the pre-loco one).
- [ ] **S-3 (L)** Tantivy in **case** (as S-1). Depends: S-1.
- [ ] **S-4 (L)** Tantivy in **portfolio** — note the kind gate: index
  `kind` as a field and filter search/dedup within-kind. Depends: S-1.

- [x] **P-1 (M)** Privacy module in **organization**. *(done 2026-08-01)*
  `src/privacy.rs`: `mask_organization` + `export_organization`, the
  endpoints `GET /{pid}/masked` and `GET /{pid}/export`, and the ABAC
  **`mask` obligation** wired into `GET /{pid}` and the export via new
  `auth::authorize_record` + `auth::organization_resource_attrs`
  (`resource.jurisdiction`, `resource.has_fiscal_id`).
  *Verified:* fmt + clippy clean; 136 DB-free tests; 23 DB-gated green
  vs Postgres 18, including a dedicated `tests/masking.rs` binary —
  mutation-checked (dropping the obligation branch fails it).

  What organization masks is **not** what person masks, and the
  difference is the point: most of an organization record is published
  fact. Redacted are `telephone`, `email` (routinely a named
  individual's line or inbox), the address's `street_address` (for a
  sole trader that is a home address, and there is no `is_sole_trader`
  flag to key on, so the street line goes for every record while
  locality / postcode / country stay), and `TaxId` / `Vat` values.
  **Not** redacted: LEI / DUNS / ROR / ISNI / Wikidata, the names,
  `url`, `jurisdiction` — masking those would break the lookups a
  registry exists for.

  **Consent is refused, not deferred.** The shared model is a *data
  subject* granting a purpose; an organization is not one, and the
  natural persons behind it are the person service's to record. A
  second, unauthoritative home for consent is worse than none. Stated
  in the crate spec §2/§13 so the next pass does not "finish" it.
- [ ] **P-2 (M)** Privacy in **care-pathway** (as P-1; clinical data —
  mind `compliance-for-healthcare.md`). Depends: P-1.
- [ ] **P-3 (M)** Privacy in **case** — it already honours the `mask`
  obligation; add the masked-view + GDPR-export endpoints on top of the
  existing `mask_case`. Depends: P-1.
- [ ] **P-4 (M)** Privacy in **portfolio** (lower sensitivity; masking of
  owner/person refs). Depends: P-1.

- [x] **AU-1 (M)** Roll the case-only auth hardening to **person, worker,
  place, thing, event** (axum-style `src/api/rest/auth.rs`): key-rotation
  refresh loop (`ReloadableVerifier` + `spawn_key_refresh` — case
  `src/auth.rs` is the pattern), policy hot-reload
  (`ReloadablePolicy` + `spawn_policy_watcher`), and a per-service
  `tests/enforcement.rs` activation proof (case's is the template; each
  runs in its own test binary).
  *Verify:* per-service green gate; jwt-enforcement.md status updated.
  **Done 2026-08-01** across all five, with `jwt-enforcement.md` §Status
  rewritten.

  The rollout's finding, worth carrying into AU-2/AU-3: **every one of
  the five had snapshotted the verifier into request state** (worker
  twice over), so a rotated key set could reach the handlers but not the
  guard, or the reverse. The bug was not that rotation was missing — it
  was that adding rotation to a snapshot would have half-worked, which is
  harder to notice than not working at all.

  - [x] **person** *(done 2026-08-01)* — the axum-style reference. All
    three parts landed and verified: fmt + clippy clean, 301 lib tests,
    **40 DB-gated green** vs Postgres 18, and the new
    `tests/enforcement.rs` mutation-checked (forcing the flag off fails
    it).

    The part worth copying carefully: person kept the verifier as an
    `Arc<Verifier>` **snapshot** in `AppState`, *and* the enforcement
    middleware took its own copy. Two snapshots means a rotation could
    only ever update one of them, so the fix was to delete the field and
    have the guard and both extractors read one process-wide
    `ReloadableVerifier` per request. Any service still holding a
    verifier in its state has the same latent split.

  - [x] **worker** *(done 2026-08-01)* — the worst split of the five: the
    verifier was snapshotted in `AppState` **and** captured a second time
    by `apply_enforcement`, so a rotation could have updated one and not
    the other. Both are gone; `apply_enforcement(router, require_auth)`
    reads the holders per request. 33 DB-gated green, enforcement proof
    mutation-checked.

  - [x] **place** *(done 2026-08-01)* — `EnforcementState` now carries
    only the flag. This crate had **no HTTP test harness** (its `tests/`
    are library tests over pure functions), so the activation proof
    brings a minimal one that builds the production router; `serial_test`
    and `tower` joined its dev-dependencies for it. 3 DB-gated green.

  - [x] **thing, event** *(done 2026-08-01)* — both kept `require_auth`,
    the verifier **and** the policy in `AppState`. The verifier and policy
    moved to the holders; only the flag stays on the state, because
    turning enforcement on or off mid-flight is not something to do
    without a restart. Both gained place's in-test router for the proof.
    thing 197 lib + 3 DB-gated, event 152 lib + 7 DB-gated, all green.
- [x] **AU-2 (M)** Same for **organization, care-pathway, course,
  portfolio**. *(done 2026-08-01)* Their verifier and policy were
  **boot-only `OnceLock` snapshots**, so unlike the axum-style five there
  was no split to fix — there was simply no way for a rotation or a
  policy edit to reach a running process. Both are now reloadable
  holders read per request, with `spawn_key_refresh` +
  `spawn_policy_watcher` wired at boot.

  Course turned out to be **axum-style**, not loco-style, despite this
  task's grouping: its auth lives in `src/api/rest/auth.rs` and its state
  held the verifier, policy and flag exactly as thing's did. Same
  treatment, and the grouping in this file was simply wrong.

  Activation proofs: organization, course and portfolio gained a
  `tests/enforcement.rs` (own binary); care-pathway already had one.
  organization's `authorize_record` — added with the privacy layer —
  reads the same holder, so masking decisions follow a reloaded policy.

  *Verified:* fmt + clippy clean in all four; DB-gated green vs
  Postgres 18 — organization 24, care-pathway 44, course 15,
  portfolio 37.
- [ ] **AU-3 (S)** link-graph auth completion: boot-time
  keys-over-HTTP fetch + key-rotation refresh (its `Verifier` is
  currently env-only `OnceLock`; swap to `ReloadableVerifier`), and
  OTLP tracing (spec T-22) + `user_ip` capture (ConnectInfo) on governed
  audits.
  *Verify:* green gate; link-graph spec §13 T-19 note + T-22 checked.

- [ ] **PG-1 (L)** Pagination in the four newest loco services: replace
  `LIST_CAP`/`SEARCH_CAP` with `offset`+`limit` params (bounded maxima),
  returning total-count metadata without breaking existing response
  shapes (additive envelope field or headers — pick one convention,
  document in `agents/share/restful.md`). Update the four sibling
  front-ends in the same task.
  *Verify:* service green gate + front-end `pnpm check/test/build`;
  contract pinned in request tests.

## Phase 3 — Platform

- [ ] **BUS-1 (L)** `FluvioSink` (feature `fluvio`) in **case** (the
  relay's `EventSink` seam exists; see `src/relay.rs`). Topic naming per
  event-bus.md §7 (`mxi.case.events`, partition by pid). Compose file
  with a Fluvio broker for the bus-gated test tier (`#[ignore]` +
  feature-gated test: enqueue → relay → topic → consumed).
  *Verify:* default build untouched (feature off); feature build green;
  bus-gated test compiles (runs only with broker).
- [ ] **BUS-2 (L)** link-graph **Fluvio consumer** (spec T-6):
  per-topic consumers driving the existing `apply_event` seam, with the
  `processed_events` idempotency table (spec §10.3) and per-topic offset
  resume. Retire lazy verify-on-read for entities with a live topic
  (keep it for the rest). Depends: BUS-1.
- [ ] **BUS-3 (M)** Roll `FluvioSink` to the remaining nine services;
  reconcile the five older crates' dormant `fluvio` Cargo deps (use or
  remove). Depends: BUS-1, BUS-2.

- [x] **LNK-1 (M)** Envelope `data` field + `Linked`/`Unlinked` kinds in
  **person**. *(done 2026-07-14)* `EventKind` gained `Linked`/`Unlinked`
  (+ tokens); `Envelope` gained an additive `data: Option<Value>`
  (`skip_serializing_if` — the CRUD/merge wire shape stays byte-identical,
  pinned by `crud_envelope_omits_data_on_the_wire`) carrying the §4.2 edge
  detail, plus a `for_link` constructor. The links handlers now emit:
  `create_link` → `linked`, `delete_link` → `unlinked`, transport-aware —
  under `outbox` the edge mutation + its event commit in one transaction
  (the outbox guarantee), under `memory` the in-memory
  `PersonEvent::Linked`/`Unlinked` (lossy dev signal). *Verified:* the seam
  unit test `for_link_carries_edge_detail_data` (data matches the
  aggregator's `LinkedEvent`) + token/frozen-shape tests + a DB-gated
  `linked_event_is_enqueued_to_the_outbox`; person lib + clippy + fmt clean.
  **Worker mirror landed the same day** (identical change on worker's
  envelope + `WorkerEvent`; worker lib 198 pass + clippy + fmt clean), so
  both person and worker now emit `linked`/`unlinked`.
- [x] **LNK-2 (M)** **Worker** `same_identity` write-side. *(done 2026-07-14)*
  Mirrors person's (`entity_links` migration + `NULLS NOT DISTINCT` upsert
  key, SeaORM entity, `src/db/entity_links.rs` persistence, `validate_edge`
  accepting only `same_identity` **worker → person**, per-record
  `POST`/`GET`/`DELETE /api/workers/{id}/links` + the governed bulk
  `GET /api/workers/links` returning canonical `EdgeDetail`, both router
  surfaces, record-level authz + audit incl. a new `log_export`; depends on
  the shared `entity-ref` crate). Worker added to the aggregator's reconcile
  list (`app.rs` `["case","person","worker"]`) + seam test
  `bulk_response_deserializes_the_worker_same_identity_shape`. Symmetric
  double-assert is by design (aggregator canonicalises the pair). Event
  emission deferred (as on person). *Verified:* worker `cargo test --lib`
  (194 pass, 7 links) + clippy + fmt clean; aggregator lib tests (41 pass)
  + clippy clean. Follow-ups all landed since: worker `employed_by` (LNK-3),
  `linked`/`unlinked` events (LNK-1), and the matcher-partition guard test
  (`links_are_not_a_matcher_signal`, both person + worker, 2026-07-15).
- [x] **LNK-3 (M)** Affiliation edges. *(done 2026-07-14)* `works_at` /
  `member_of` on **person** (→ organization) and `employed_by` (with `role`)
  on **worker** (→ organization). Each `validate_edge` permit set extended
  from `same_identity`-only to include the affiliation kinds (person
  `{same_identity, works_at, member_of}`, worker `{same_identity,
  employed_by}`), relying on the shared `entity-ref` `EdgeKind::permits` for
  the endpoint check; same tables / endpoints / generic bulk pull unchanged.
  Accept/reject matrices unit-tested per crate (affiliation → non-org
  rejected; cross-originated kinds rejected). *Verified:* person + worker
  `cargo test --lib links` (9 each) + clippy clean. Follow-ups (shared with
  LNK-2): `linked`/`unlinked` events + matcher-partition guard test.
- [~] **LNK-4 (L)** Cross-service `same_identity` **matcher + review
  queue** (design §5.2, roadmap): a job comparing person↔worker records
  (reuse matcher components), emitting `matcher_suggested` edges
  (confidence < 1.0) into a review surface; operator confirm promotes to
  `operator`/1.0. **Spec round done 2026-07-15** — link-graph spec §16 OQ-9
  (the resolved design: cross-type `IdentityProbe` comparator reusing the
  matcher primitives; identifier+Soundex/birth-year blocking;
  aggregator-hosted job that POSTs `matcher_suggested` edges to person's
  links endpoint while the aggregator stays read-only-to-the-world;
  per-service review + idempotent promotion) + the §13 task chain T-29–T-33.
  **Coding still gated** on the OQ-9 open sub-questions (block key/threshold,
  review-surface home, aggregator-write posture, scale). Depends: LNK-1..3.

- [x] **BLK-1 (M)** Bulk I/O step 2a — **CSV** codec on person.
  *(codec + spec done 2026-07-15)* `src/bulk/csv.rs` flattens the person wire
  type per §5 (scalars → columns; primary name → dotted `name.*`; arrays →
  JSON-in-cell) and **round-trips losslessly** against JSONL
  (`decode(encode(p)) == p`); columns matched by header (reordered/extra
  tolerated); per-row `Err` on a malformed row (§7). Person's exact column
  set declared in spec §10.6; adds the `csv` crate. Unit-tested: fully-
  populated + sparse round-trip, reordered/extra columns, bad-JSON-cell
  per-row error, multi-row, header. *Verified:* `cargo test bulk::csv`
  (6 pass) + clippy clean. **Remaining wiring (folded into BLK-2):** the
  `bg_pg` worker/export `format` dispatch that makes CSV a usable end-to-end
  import/export format.
- [ ] **BLK-2 (M)** Bulk I/O step 2b — keyless-row → duplicate-detection
  → **review-queue** routing on person import (`provenance = import`),
  reusing the existing matcher + review queue. Depends: BLK-1 optional,
  independent of format.
- [ ] **BLK-3 (S)** Parquet **export** (feature-gated `parquet`;
  arrow/parquet deps only under the feature). Export-only per §12 lean.
- [ ] **BLK-4 (M)** S3-compatible `ArtifactStore` impl (config-driven
  switch local-fs vs S3; mirror the env-var conventions). Feature-gate
  the S3 SDK dep if heavy.
- [ ] **BLK-5 (L)** Roll bulk I/O to **organization** (stable key:
  LEI → DUNS → pid) and **case** (agency-scoped case number → pid),
  declaring each §10 section in their specs. Person's `src/bulk/` is the
  reference; these services are case-style loco (simpler than person).
  Depends: BLK-1..2 (so the rolled version includes CSV + review routing).

## Phase 4 — Surfaces, deployment docs, tutorials, examples

- [ ] **FE-1 (M)** Merge actions in the **organization, case, portfolio**
  front-ends (person/worker/place/thing/event/course have the pattern;
  API: `POST /merge`, `merges/recent`).
  *Verify:* `pnpm check` + `pnpm test` + `pnpm build` per app.
- [ ] **FE-2 (M)** Links panel: person front-end (assert/list/withdraw
  `same_identity` + affiliations), case front-end (`subject_of`),
  worker front-end once LNK-2 lands.
- [ ] **FE-3 (M)** Bulk import/export screen in the person front-end:
  upload JSONL, dry-run toggle, job status polling, error-report and
  export download links (BFF-mediated; no token in browser).
- [ ] **FE-4 (M)** Duplicate review-queue screen (services exposing the
  review API; start with person).

> Note: the **test** database side of this is already done — every
> service crate carries a `compose.test.yaml` driven by
> `scripts/test-db.sh` (see DEP-0 below). DEP-1 is the *demo/dev* stack:
> services plus their databases, wired to each other.

- [ ] **DEP-1 (M)** `examples/compose/`: podman-compose for (a) one
  service + postgres, (b) the full family (10 services + auth +
  link-graph + postgres), (c) the enforced variant (auth on, policies
  mounted, reconciliation configured). Compose is also what tutorials and
  the bus-gated tests build on.
- [ ] **DEP-2 (M)** `agents/share/configuration.md`: the complete env-var
  reference — every `<ENTITY>_*`, `LINK_GRAPH_*`, `AUTH_*` variable, its
  default, effect, and which doc governs it. Generated by sweeping
  `std::env::var` call sites; keep a per-service table.
- [~] **OPS-1 (M)** Runbooks under `agents/share/runbooks/`: auth
  activation checklist (expand jwt-enforcement.md), key rotation,
  reconciliation-divergence response, event-bus outage/replay, bulk-job
  failure recovery. Each: symptoms → checks → actions → verification.
  **Partial (2026-07-27):**
  [`runbooks/integrity-activation.md`](agents/share/runbooks/integrity-activation.md)
  covers the integrity and audit controls — activation order and why it
  is an order, how to verify each step actually took effect, checkpoint
  storage, symptoms→checks→actions, and MAC-key rotation. Written because
  every control in that stack is **default-off**, so a deployment doing
  nothing gets none of it. Still unwritten: PASETO key rotation,
  reconciliation divergence, event-bus replay, bulk-job recovery.

- [ ] **TUT-1 (S)** `tutorials/01-getting-started.md` — run one service +
  front-end (uses DEP-1a). Every command copy-pasteable and verified.
- [ ] **TUT-2 (M)** `tutorials/02-identity-lifecycle.md` — create →
  409-duplicate → check-duplicates → match → merge → audit trail, curl +
  UI.
- [ ] **TUT-3 (M)** `tutorials/03-authentication-abac.md` — magic link
  (console), session cookie, `POST /token`, protected call, 401/403
  matrix, write + hot-reload a policy, `mask` obligation demo.
- [ ] **TUT-4 (M)** `tutorials/04-cross-service-linking.md` —
  `subject_of` + `same_identity` writes → aggregator `neighbors` /
  `single-view` / `freshness`; break-and-reconcile demo (divergence
  metric → repair). Depends: DEP-1b.
- [ ] **TUT-5 (S)** `tutorials/05-bulk-import-export.md` — fixtures
  import (dry-run, error report), idempotent re-import, masked vs full
  export (and the 403 on ungated full). Depends: EX-1.
- [ ] **TUT-6 (S)** `tutorials/06-event-bus.md` — outbox rows, relay,
  `/events/recent`; extend with Fluvio when BUS-1..3 land.

- [ ] **EX-1 (S)** `examples/data/` — synthetic JSONL fixtures: ~50
  persons (with duplicate pairs for the dedup tutorial), ~20
  organizations, ~10 cases with subject links. No real PII; documented
  provenance header in each file.
- [ ] **EX-2 (S)** `examples/policies/` — ABAC cookbook: dept-scoped
  read-deny, closed-case write-deny (`resource.status`), after-hours deny
  (`env.after_hours`), ownership (`$sub`), masked-read obligation,
  machine-peer grant (`svc`). Each policy JSON + a three-line README
  entry; all validated by a small test in the verifier crate that parses
  every example file.
- [ ] **EX-3 (S)** `examples/api/` — per-service request collections
  (`.http` files or curl scripts) for the main endpoints incl. auth
  handshake. Spot-verified against a running compose.
- [ ] **EX-4 (S)** A demo **seed** path: loco task (person + organization
  + case) or documented bulk-import of EX-1 fixtures — pick one,
  reference it from TUT-1/2/4.

## Phase 5 — Security hardening (audit-driven, 2026-07-12)

> From the repo-wide security audit (plan.md Theme F). Severity: 🔴 critical
> · 🟠 high · 🟡 medium · ⚪ low. Each code task is three-part (crate spec
> §13 + code + **security test**) + CHANGELOG; the audit's "recommended
> tests" ARE the test half (this is where the "improve tests — fuzzing,
> races, unverified inputs" mandate lands). Criticals (SEC-A1, SEC-B1,
> SEC-G1, SEC-B5) lead; SEC-I1/I3 are cheap Phase-1 hygiene. File:line
> anchors are from `main` at audit time — re-verify before editing.

### F-authn — token & session integrity (authentication-service)

- [x] **SEC-A1 (S) 🔴** DEV_SEED prod guard. *(done 2026-07-12)* `load_seed()`
  (`src/auth/mod.rs`) silently fell back to the committed `DEV_SEED` →
  forgeable tokens. Now `dev_seed_fallback(is_production)` refuses the
  fallback when `LOCO_ENV`/`RUST_ENV` = `production`, so `load_keys()`
  errors and `keys()` boot-panics with guidance; dev/test still boot on
  `DEV_SEED`. Unit test `dev_seed_fallback_refused_in_production`
  (env-free, pure helper — edition-2024 + `forbid(unsafe)` rules out
  `set_var` in tests). Green: lib tests + clippy + fmt.
- [x] **SEC-A2 (S) 🟠** Gate `GET /api/auth/audit/recent`. *(done 2026-07-13)*
  Was unauthenticated + returned `auth_events` emails/outcomes (an
  enumeration oracle via timing). Now requires a PASETO bearer with
  `access=admin` (`401`/`403`); `recent_audit` takes `AuthUser` +
  `claims_have_admin`. Unit test `recent_audit_requires_admin`; DB-gated
  request test pins the `401`; spec §12 decision superseded.
- [x] **SEC-A3 (S) 🟠** Magic-link token logging. *(done 2026-07-13)*
  `deliver_magic_link` logged the full verify URL (embedding the live token)
  at `info` in every env. Now gated to `Environment::Development` only via
  pure `log_magic_link_url`; other envs log the issuance without the token.
  Unit test `magic_link_url_logged_only_in_development`.
- [x] **SEC-A4 (S) 🟡** Atomic single-use magic-link consume. *(done 2026-07-13)*
  Replaced SELECT-then-clear with `Model::consume_magic_token` — one
  `UPDATE … WHERE magic_link_token=$1 AND not-expired RETURNING *` (via
  `query_one` + `FromQueryResult`), so concurrent redemptions can't both win.
  DB-gated `concurrent_magic_link_redemptions_only_one_wins` (exactly one 200,
  one 401). Green: lib (71) + `test --no-run` + clippy + fmt.
- [x] **SEC-A5 (S) 🟡** Constant-work signup. *(done 2026-07-14)*
  `create_passwordless` returns `EntityAlreadyExists` before its Argon2 hash,
  so only the new-account path paid the deliberately-slow hash — a timing
  oracle for enumeration despite the always-`200` response. The
  existing-email branch now runs one equivalent Argon2 hash
  (`constant_work_hash`, discarded), so both paths perform one hash and
  signup latency is indistinguishable between new and existing. Unit test
  pins that a real `$argon2` hash is performed (fresh per call).
- [x] **SEC-A6 (S) 🟡** Rate-limit email canonicalization + case-consistent
  `find_by_email`. *(done 2026-07-14)* `rate_limit::normalize_key` folds the
  throttle bucket aggressively — trim + lowercase, strip `+tag`, Gmail/
  `googlemail` dot-folding — so `victim+1@gmail.com` / `v.ictim@gmail.com` /
  `Victim@…` collapse to one bucket (throttle-only; never loosens the quota).
  `users::find_by_email` + `create_passwordless` are now case-insensitive
  (`LOWER(email)` compare + `normalize_email` store), so a case variant is
  the same account, not a duplicate. **Deliberately case-only for identity**
  — `+tag`/dot folding is confined to the throttle bucket, not account
  identity (the security-clean subset; provider-specific folding must not
  merge distinct accounts). Pure key-collapse tests + a DB-gated
  case-variant signup test (one lowercased account).
- [x] **SEC-A7 (S) 🟡** GDPR erasure completeness. *(done 2026-07-13)*
  Erasure now scrubs the subject's email from `auth_events`
  (`AuthEvent::scrub_subject_email`, pid OR normalised-email match) and
  `sessions.user_agent` (`scrub_user_agent_for_user`), and writes
  `account_erased` without the email. Extended `account_erasure_…` request
  test asserts no `auth_events` row retains the email + user_agent scrubbed.
- [x] **SEC-A8 (M) 🟡** Privilege-revocation latency. *(done 2026-07-13)*
  The admin attribute API (`PUT …/attributes`) and the `user_attributes` CLI
  task now `sessions::Model::revoke_all_for_user` after a change, so a
  session that snapshotted the old attrs can't keep minting stale-attribute
  tokens until its absolute TTL — the next login copies fresh attrs. Extended
  admin request test asserts the target's sessions are revoked after the PUT.
- [x] **SEC-A9 (M) 🟡** Hash bearer-equivalent secrets at rest.
  *(done 2026-07-14)* Magic-link token, session `jid` (cookie / PASETO `sid`),
  and CSRF token (`sessions.data.csrf`) now store only a one-way SHA-256 hash
  (new `secret_hash` module; fast unsalted hash — high-entropy tokens, so
  deterministic lookup-by-hash, not Argon2). `create_magic_link` returns the
  plaintext in-memory (email/log) but persists the hash; `consume_magic_token`
  / `find_by_jid` / CSRF compare hash the presented value. Migration `_000009`
  enables `pgcrypto` + hashes existing rows in place
  (`encode(digest(x,'sha256'),'hex')`, guarded `length <> 64`) so live
  links/sessions survive. *Test:* `secret_hash` vectors + DB-gated
  `session_secrets_are_hashed_at_rest` / `magic_link` assert the DB holds no
  usable plaintext credential while presented-plaintext lookups still resolve.
- [x] **SEC-A10 (S) 🟡** CSRF origin backstop. *(done 2026-07-14)* Pure
  `csrf_token_gate(is_production, origin_ok, session_csrf, provided_csrf)` in
  `controllers/auth.rs`: a token-carrying session must echo `X-CSRF-Token`; a
  legacy no-`csrf` session must prove same-origin (`AUTH_ALLOWED_ORIGINS`) and
  is refused in production without it, so it can no longer bypass both checks.
  Unset allow-list in production warns once (`warn_missing_allowed_origins`).
  *Test:* `csrf_gate_matrix` — no-csrf session cannot bypass both CSRF and
  origin checks in production.

### F-authz — verifier & ABAC (authentication-verifier)

- [x] **SEC-V1 (S) 🟡** `from_paseto_keys_url` hardening. *(done 2026-07-13)*
  Now refuses non-`https://` URLs, forbids redirects (no https→http bounce),
  sets a 10 s timeout, and reads the body under a 64 KiB cap (MITM key
  injection / boot-hang / OOM). Test `non_https_keys_url_is_refused`.
- [x] **SEC-V2 (M) 🟡** Vacuous-negation escalation. *(done 2026-07-13)*
  A `!`-negated `resource.`/`env.` condition matched vacuously when the
  namespace was absent. An absent namespaced attr now biases by effect to
  the safe outcome: `allow` rules do NOT match (no silent grant), `deny`
  rules still match (fail-closed); subject-attr negation unchanged. Test
  `negated_allow_does_not_match_vacuously_when_namespace_absent` (+ existing
  deny-rule test preserved).
- [~] **SEC-V3 (S) ⚪** Key-set load resilience — **deferred**. Skipping a
  malformed Ed25519 entry (vs the current deliberate fail-fast on a
  malformed key set) contradicts a stated design decision + spec and is Low
  severity; left as an open call (fail-fast surfaces misconfiguration;
  skip-and-continue favours availability).
- [x] **SEC-V4 (M) 🟠 (tests)** Forgery + robustness suite. *(done 2026-07-13)*
  Added the previously-missing **cross-key forgery** test (attacker sig +
  honest `kid` ⇒ `Paseto` Err), `token_missing_exp_is_rejected`, and
  `malformed_tokens_never_panic` (arbitrary/truncated/oversized input ⇒ only
  ever `Err`, never panics). Example-based (no new deps); a full `proptest`/
  `cargo-fuzz` policy-property + parser-fuzz layer folds into SEC-I2.

### F-guard — read-path masking & guard consistency (entity services)

- [x] **SEC-G1 (M) 🔴/🟠** Governed bulk-links leak. *(done 2026-07-12)*
  `GET /api/cases/links` dumped every `subject_of` edge (and the person
  twin `GET /api/persons/links` every `same_identity` edge) with only the
  coarse gate + no audit. Both handlers now authorise the cross-record dump
  as a privileged governed read (`authorize_record(Action::Destructive,…)`
  — default policy admits only `svc`/`admin`) and audit each surfacing.
  Case: DB-gated `bulk_links_requires_elevated_authority` (401/403/200 in
  `tests/enforcement.rs`). Person: unit test pins the `Destructive`
  classification (shares case's e2e-proven gate). Green: both crates' lib
  tests + clippy + fmt (+ case `test --no-run` DB-gated).
- [x] **SEC-G2 (M) 🟠** Case FHIR read/search authz + mask. *(done 2026-07-13)*
  Both now take a caller and apply record-level ABAC + the `mask` obligation
  like native `get_one`: FHIR `read` → `403` on deny + masked Task on the
  `mask` obligation; FHIR `search` omits denied cases + masks the rest.
  DB-gated `tests/masking.rs` (denied caller gets `403` on `/fhir/Task/{id}`).
- [x] **SEC-G3 (M) 🟠** Masking-on-every-read. *(case done 2026-07-13; person
  done 2026-07-14)* Case `list`/`search`/`check_duplicates` **omit** cases the
  caller may not read (concealment) via the shared `auth::read_visibility`;
  DB-gated `tests/masking.rs` proves the list conceals a denied case. Person
  `search_persons` (`api/rest/handlers.rs`) now runs `auth::read_visibility`
  on every hit too: a denied record is omitted (concealed), a `mask`
  obligation masks even without the client `mask_sensitive` param (closing the
  bypass), and the param still masks on request; no-op when
  `PERSON_REQUIRE_AUTH` is off. Pure `search_result_disposition` unit test
  pins the omit/mask/full matrix. (Person `check_duplicates`/`match` return
  match candidates, not a record dump; a concealment pass there is an optional
  follow-up if a deployment needs it.)
- [x] **SEC-G4 (S) 🟡** `escape_like` in the repo-based searches. *(done 2026-07-13)*
  person / worker / event `db/repositories.rs::search` built `format!("%{}%",
  query.to_lowercase())` with no escaping (raw `%`/`_` = wildcard
  injection / scan-everything DoS; already a bound param so not SQLi). Each
  now escapes `\`/`%`/`_` via a per-crate `escape_like` helper before the
  contains-pattern; unit test `escape_like_neutralises_wildcards` in all
  three (ports the loco `escape_like` test). Green: lib test + clippy
  `-D warnings` + fmt per crate.
- [x] **SEC-G5 (M) 🟡** Guard-all for event/thing/course. *(done 2026-07-13)*
  Their `enforce` was **allow-unless-in-prefix** (any non-`/api`/non-`/fhir`
  path unguarded); now **deny-unless-public** via a `is_public_path`
  allow-list (matching the case reference), and the dead prefix consts/helpers
  removed. Guard-bypass test per crate: enforcement on + no token ⇒ `401` for
  `/`, `/admin`, `/secret`, `/foo/bar`. (The other 8 services are already
  guard-all. A percent-encoded/normalisation matrix vs the router is a
  deeper follow-up.)
- [x] **SEC-G6 (S) 🟡** Destructive-action classification robust to a
  trailing slash. *(all 10 services done 2026-07-13)* `derive_action` now
  `trim_end_matches('/')`-normalises the path before the destructive-suffix
  check, so `POST …/merge/` stays `Destructive` (was downgraded to `Write`,
  which an `access=write` non-admin caller could exploit). Rolled to all ten
  services (case, event, thing, course, care-pathway, portfolio, place,
  organization, worker, person) with a trailing-slash test per crate.
- [x] **SEC-G7 (S) ⚪** Bound person `search_persons` `offset`. *(done
  2026-07-14)* `GET /api/persons/search` rejects `offset > MAX_SEARCH_OFFSET`
  (10 000) with `400 OFFSET_TOO_LARGE` before asking the index for
  `offset + limit` hits (unbounded offset ⇒ index materialises arbitrarily
  many hits; the add could also overflow — now `saturating_add`). Pure
  `search_offset_within_bound` unit test + DB-gated `400` integration test.
- [x] **SEC-G8 (S) 🟡** Default-off exposure pin. *(done 2026-07-14)* Added a
  named unit test
  (`default_off_exposes_sensitive_reads_activation_is_a_release_gate`) to the
  two services the audit flagged for the bulk-links + audit exposure — **case**
  (PII / audit / governed `subject_of`) and **person** (PII / GDPR export /
  audit / `same_identity`) — pinning that with `<ENTITY>_REQUIRE_AUTH` off
  those reads are open without a token, so activation is a **tracked release
  gate** (framed in `agents/share/security.md` §4 from SEC-I4). The generic
  flag-off `enforce` pin already exists family-wide; this adds the explicit,
  sensitive-path-named form on the flagged services. Feeds OPS-1 runbook.

### F-data — bulk / linking / concurrency integrity

- [x] **SEC-B1 (M) 🔴** link-graph reconcile cross-entity scoping. *(done 2026-07-12)*
  It diffed the **global** read-model (`all_edge_ids`) against **one**
  entity's edges, so each entity pass deleted the others' edges and the
  graph never converged. `AuthoritativeSource` now declares `entity()`;
  `reconcile` diffs only `edges::Model::edge_ids_from_entity(source.entity())`
  (exact `<entity>:` `from_ref` prefix — correct for `subject_of` from=case
  and canonical `same_identity` from=person). DB-gated
  `reconcile_is_scoped_to_the_source_entity` (case pass leaves the person
  edge intact) + pure `from_ref_scoping_*` unit tests. Green: lib tests +
  `test --no-run` (DB-gated) + clippy + fmt.
- [~] **SEC-B2 (M) 🟠** person bulk import caps. *(caps + fuzz done
  2026-07-13; true streaming deferred)* Import upload read chunk-by-chunk
  and rejected `413` past `MAX_IMPORT_BYTES` (64 MiB) **before**
  materialisation (`read_field_capped`/`exceeds_cap`); pipeline rejects a
  load over `MAX_IMPORT_ROWS` (1M) via `split_lines_capped`; export `limit`
  clamped to `MAX_EXPORT_ROWS` (1M) via `clamp_export_limit` (worker mapping
  + pipeline listing path). proptest fuzzes `parse_line`/`split_lines`/
  `split_lines_capped` (random bytes / truncated UTF-8 / 2 MiB line never
  panic); boundary `exceeds_cap` unit-tested incl. saturating-add overflow.
  **Deferred:** true end-to-end streaming (never buffering the whole file,
  so the caps can rise) — the caps make the current buffered path safe.
- [x] **SEC-B3 (M) 🟠** person bulk upsert idempotency race. *(done
  2026-07-13)* The per-row find→create/update runs under a
  transaction-scoped advisory lock on the stable key
  (`pg_advisory_xact_lock(hashtext(key))`, `import_upsert_locked`), so two
  concurrent importers of one key produce exactly one record (the second
  upserts the first's). **Chose advisory lock over `UNIQUE(system,value)`**:
  the registry permits duplicate identifiers by design (dedup is a
  workflow), so a hard uniqueness constraint would reject legitimate data.
  DB-gated test: two concurrent imports of one SSN key ⇒ one distinct owner,
  one create + one upsert; plus a pure lock-key collision test.
- [~] **SEC-B4 (M) 🟠** person bulk artifact hardening. *(store confinement +
  IDOR + TTL done 2026-07-13; object-store sweep deferred)* (1) store `get`
  **confined** to the canonicalised base + `is_safe_key` on `put`/`get`
  (rejects `..`/absolute/`file://`-escape → closes arbitrary-file read);
  (2) job-status GET returns `404` unless the caller **owns** the job
  (`is_job_owner`, `actor == sub`) or is elevated (`access=admin`/`svc=true`)
  → closes IDOR/BOLA on status + download URL; (3) `create` stamps
  `expires_at = created_at + BULK_ARTIFACT_TTL_SECS` (7 days), status handler
  `404`s an expired job (`artifact_expired`). Pure cores unit-tested incl.
  the outside-the-base `file://` refusal. **Deferred:** physical artifact
  deletion (object-store TTL sweep) — the expiry gate stops the reference
  being handed out.
- [x] **SEC-B5 (M) 🔴/🟠** Merge TOCTOU + self-merge. *(done 2026-07-13)*
  person `POST /merge` had **no self-merge guard** (merged a record into
  itself → tombstoned + data loss) — now `422` before any fetch
  (`test_merge_into_self_is_rejected`; case already had the guard).
  Both merges read main+duplicate **unlocked** before the write tx; the
  person repository `merge` and the case `outbox` `merge_and_emit` now lock
  both participant rows `FOR UPDATE` (id-ordered, deadlock-free) and
  re-check the duplicate is still active before writing, so concurrent
  merges of the same duplicate can't both apply (loser fails closed).
  Green: person lib (186) + integration compile + clippy + fmt; case lib
  (96) + no-run + clippy + fmt. *(Residual: a deterministic concurrent-race
  integration test, and the case `memory` (dev, non-transactional)
  path — both follow-ups.)*
- [x] **SEC-B6 (M) 🟠** Relay exactly-once. *(all 10 services done 2026-07-13)*
  `drain_once` now runs in a transaction and `unpublished` claims rows with
  `FOR UPDATE SKIP LOCKED`, so >1 relay instance can't double-ship (a second
  instance skips the locked rows; lock releases on commit). Rolled to **all
  ten** loco/axum services (case, person, worker, place, thing, event,
  course, organization, care-pathway, portfolio) — each green (lib + clippy
  + fmt). Consumer-side `event_id` dedupe (`processed_events`) is the
  aggregator's job — folds into BUS-2; a deterministic two-concurrent-drain
  test needs a dual-connection harness (follow-up).
- [x] **SEC-B7 (S) 🟡** link-graph reconcile peer trust. *(done 2026-07-13)*
  `HttpAuthoritativeSource::from_env_for` refuses an **unauthenticated
  remote** source — a non-loopback URL requires `LINK_GRAPH_RECONCILE_TOKEN`
  (`source_auth_ok`/`is_loopback_url`, fail-closed on an unparseable URL);
  only a loopback URL may be token-less. Before `apply_linked`, `reconcile`
  validates each edge via `edge_valid_for_source`: it must originate from the
  source's own entity AND its endpoint types must be permitted for its kind
  (`EdgeKind::permits`), so a compromised/buggy source can't inject a
  cross-typed or foreign-origin edge (ill-typed edges skipped, stay as
  divergence). Pure helpers unit-tested (remote-needs-token, loopback-ok,
  ill-typed + foreign-origin rejected).
- [~] **SEC-B8 (S) 🟡** Bulk audit gaps. *(job-level audit + fail-closed
  export + actor threading done 2026-07-13; per-row actor deferred)* A
  successful import now writes a job-level `IMPORT` audit row (`log_import`)
  with the actor + reconciled counts; the export audit is written **before**
  `finish_export` and its error **propagates**, so a failed audit marks the
  job `failed` and never surfaces `download_url` (fail-closed delivery). The
  actor is threaded into both rows (fallback `system` only when the job had
  no caller). Pure `import_audit_summary`/`export_audit_summary` unit-tested.
  **Deferred:** threading the real actor into each **per-row** create/update
  audit — needs a `PersonRepository::create/update` signature change (they
  build a default `system` `AuditContext` today).
- [x] **SEC-B9 (S) 🟡** Wire the idempotency key. *(done 2026-07-13)* Both
  submit handlers read an `Idempotency-Key` header;
  `create_or_get_idempotent` returns the original job (no re-store /
  re-enqueue) when the key already names one, backstopped by the existing
  `UNIQUE(entity,kind,idempotency_key)` on the check-then-insert race (no
  migration needed — the constraint already existed, just never fired). Blank
  key ⇒ absent; key-less ⇒ always creates. DB-gated same-key/keyless tests +
  pure key-trim test.
- [x] **SEC-B10 (S) 🟡** person merge audit in-tx. *(done 2026-07-13)* The
  merge `UPDATE` (survivor) + `DELETE` (duplicate) audit rows are written on
  the merge transaction (new connection-generic `log_update_on`/
  `log_delete_on`) **before** commit, so a crash after commit cannot lose the
  merge audit and an audit failure rolls the whole merge back (was
  best-effort post-commit). DB-gated test asserts both rows present after a
  merge.
- [x] **SEC-B11 (S) ⚪** link-graph `freshness` authz + non-redirecting probe.
  *(done 2026-07-13)* The probe now uses a shared **non-redirecting** reqwest
  client (`redirect::Policy::none()`); a `3xx` ⇒ `Unknown`
  (`outcome_from_status`), closing SSRF-via-redirect — the only host
  contacted is the operator-configured `LINK_GRAPH_PROBE_URL_<ENTITY>`
  template, which *is* the host allow-list (no separate list needed once
  redirects are off). Freshness was already behind the blanket guard (not in
  `is_public_path`); added a regression test pinning it stays guarded (`401`
  when enforcement on) so it can't be mistaken for a public health probe.
  Pure status-mapping + freshness-guard tests.

### F-input — unverified input, false matches & fuzzing (validators + matchers)

- [~] **SEC-M1 (M) 🟠** Input-size caps. *(case + care-pathway + portfolio
  validators done 2026-07-13; residuals below)* Per-field length +
  array-cardinality caps in `validate`/`problems` → `422` **before** persist,
  closing the O(n·m) Jaro-Winkler/Levenshtein/Jaccard DoS. **Done** (shared
  caps `MAX_TEXT_LEN=1024` chars / `MAX_ARRAY_LEN=256` entries /
  `MAX_ITEM_LEN=512` chars, incl. struct-array inner strings; false/oversized
  unit tests + within-caps pin; each crate green): case, care-pathway,
  portfolio, **organization** *(new `src/validation.rs`, done 2026-07-13)*,
  **course** *(caps woven into `validate_course`/`validate_instance`, done
  2026-07-14)*, and the **5 older axum services**
  (person/worker/place/thing/event `validation/mod.rs` — each `<entity>_size_caps`
  woven into `validate_<entity>`, done 2026-07-14). **Remaining:** only the
  coarse `limit_payload` body-cap backstop on the uncapped loco configs (+
  lower the others' 5 MB) — the config change carries loco-boot risk best
  validated by running the app.
- [x] **SEC-M2 (M) 🟠** False-deterministic-match empty guards. *(done 2026-07-13)*
  A post-normalization empty/trivial-value guard added to every string-keyed
  deterministic short-circuit across **all 9 matchers**, each with a
  false-match unit test (two different records sharing only a
  blank/punctuation/trivial value MUST NOT match) + preserved positive test:
  person/worker passport `passport_books_share_pair` + demographic fallback
  (non-empty normalised names); place `name_and_postcode_match`; thing
  `same_canonical_url` + `shares_same_as` (skip empty); event
  `name_and_start_date_match`; course + care-pathway R-1 provider-scoped
  code (require non-empty normalised code); case R-0 identifier
  (`is_trivial_identifier`: empty / `"0"` / all-zeros UUID) + R-2 `same_as`
  `"/"`; portfolio R-2 `same_as` `"/"`. Each crate re-verified independently
  green (test + clippy `-D warnings` + fmt).
- [x] **SEC-M3 (S) 🟠** Reject sentinel national IDs (all-zeros). *(done 2026-07-13)*
  `parse_ie_ihi`/`parse_es_tsi`/`parse_dk_cpr` in both the person- and
  worker-matcher `identifiers.rs` now reject an all-zeros placeholder (via
  a shared `is_sentinel_zeros` helper), matching the `nl_bsn` posture, so a
  `"0000000"` sentinel shared by two records cannot short-circuit to 1.0.
  Unit test `format_only_parsers_reject_all_zeros_sentinels` in both crates.
- [x] **SEC-M4 (S) 🟡** portfolio `days_from_civil` overflow. *(done 2026-07-12)*
  `iso_date_to_days` parsed `year` as unbounded `i64`, so a crafted date
  overflowed `era*146_097` (panic debug / wrap release) via the timeframe
  component. Year now bounded to ISO `0..=9999`; out-of-range ⇒ `None`.
  Test `iso_date_year_is_bounded_and_never_overflows` (incl. `i64::MAX`
  year). Green: lib tests + clippy + fmt.
- [x] **SEC-M5 (S) 🟡** organization identifier validation. *(done
  2026-07-14)* `validation::problems` (`identifier_problem`) validates the
  deterministic schemes before store: **LEI** (ISO 17442, 20 alnum + ISO
  7064 MOD 97-10), **GLN** (13 digits + GS1 mod-10 check digit), **DUNS**
  (9 digits — no public check digit), **VAT** (2-letter country prefix +
  2–13 alnum; per-country check digits deferred). A bad value ⇒ field-scoped
  `422`; non-deterministic schemes unconstrained. Pure check-digit helpers
  unit-tested with hand-verifiable values (GS1 `5901234123457`; ISO 7064
  synthetic `…098`).
- [x] **SEC-M6 (M) 🟠 (tests/infra)** Matcher property harness. *(proptest done
  2026-07-13; cargo-fuzz = SEC-I2, still pending)* Added `proptest = "1.11"`
  (dev-dep) + property tests to the five newer matchers (course,
  organization, care-pathway, case, portfolio — the older five already had
  it). Invariants pinned per crate: **never panics** (engine + pure helpers
  on arbitrary UTF-8), **score ∈ [0,1]** & finite, **symmetric**,
  **identical ⇒ is_match / ≥ threshold**, Soundex shape `[A-Z][0-9]{3}`/None;
  portfolio also pins the **kind gate** (cross-kind ⇒ 0.0) and an
  `iso_date_to_days` no-overflow property (reinforces SEC-M4). The symmetry
  property surfaced a **real bug** in course `provider_score` (asymmetric on
  a one-sided empty `provider_id`) — fixed to require both sides non-empty
  (three-part). Each crate independently re-verified green. cargo-fuzz
  targets remain as SEC-I2.

### F-assurance — supply-chain & test infrastructure

- [x] **SEC-I1 (M) 🟠** Dependency-scanning CI + `deny.toml`. *(done 2026-07-13)*
  Added a per-crate `deny.toml` (advisories + a permissive-license allow-list
  + `private.ignore` for local crates + bans/sources = warn) and a
  `Security Audit` `security.yml` (single `cargo deny check` job, on
  push/PR/weekly) to **all 25 Rust crate roots** (services + matchers +
  libs) — previously only 3 services had any dep-scanning. Consolidated on
  `cargo deny` (same RUSTSEC DB as `cargo audit` but honours the ignore
  policy). **All 25 pass `cargo deny check` locally** (verified). The scan
  surfaced **real transitive advisories** in the shared loco-rs tree
  (quick-xml namespace-decl DoS via opendal, protobuf recursion,
  unmaintained async-std/instant/paste, …); these are not fixable at the
  service level, so they're `ignore`-listed **with justification** and to be
  revisited on the next loco-rs bump — see the note below. Matcher/library
  crates (small trees) pass clean with no ignores needed.
- [~] **SEC-I2 (M) 🟡** `cargo-fuzz` scaffolding. *(all 9 matchers done
  2026-07-14; non-matcher roll + CI pending)* Each matcher has a standalone
  `fuzz/` cargo-fuzz crate (not a workspace member, so it never touches the
  stable build) with libFuzzer targets mirroring the SEC-M6 invariants:
  `match_<entity>` (JSON deserialize → engine; finite score ∈ [0,1], both
  orders) plus the pure-helper targets that crate exposes; `fuzz/README.md`
  documents run + roll-out. **Done — all 9:** person (reference),
  **worker / place / thing / event** (3 targets: match + `normalizer` +
  `scorer`), and **course / organization / care-pathway / case / portfolio**
  (2 targets: match + `normalize` — these expose their similarity primitives
  only through the engine, no public `Scorer`). Each verified `cargo +nightly
  fuzz build` (cargo-fuzz 0.13.2, nightly) + short campaigns run clean
  (millions of execs each, no panics/crashes; e.g. place `match_places` 2.7M,
  worker `match_workers` 3.35M, case `match_cases` 2.37M). **Also done:** the
  **auth-verifier** `fuzz/` crate — `verify` (the PASETO `v4.public` token
  parser: header / footer `kid` / signature over an arbitrary token) and
  `policy` (`Policy::from_json` + `evaluate_with_context` — the ABAC parser +
  rule evaluator), both pinning golden rule #5 (no panics); verified clean at
  `verify` 11.1M / `policy` 6.6M execs. Plus the **person bulk** `parse_line`
  target (`bulk::jsonl` split + per-line JSON parse over attacker-supplied
  upload bytes; verified clean, 173k execs). **Remaining:** only a short CI
  smoke run (all fuzz *targets* are now in place — every matcher, the
  auth-verifier, and the bulk parser).
- [x] **SEC-I3 (S) ⚪** Add `#![forbid(unsafe_code)]` to every crate root
  missing it. *(done 2026-07-14)* The three named roots (care-pathway-matcher
  `src/main.rs`, case-folder `src/lib.rs` + `src/bin/main.rs`) **plus** the 12
  SeaORM `migration/src/lib.rs` roots — the only remaining gaps a full grep
  surfaced. Now **every** `src/lib.rs` / `src/main.rs` / `src/bin/main.rs` in
  the workspace forbids `unsafe`. Builds clean; grep shows full coverage.
- [x] **SEC-I4 (M) 🟡** `agents/share/security.md`. *(done 2026-07-14)*
  Written: provenance (2026-07-12 audit), the audit summary by theme
  (F-authn/authz/guard/data/input/assurance with per-item status), the 10
  cross-cutting invariants (fail-closed secrets / never-panic / bound-input /
  no-spurious-identity / masking-on-every-read / fail-closed-authz /
  trusted-source-verify / concurrency-integrity / no-secret-in-logs /
  least-authority-artifacts), the `<ENTITY>_REQUIRE_AUTH` activation gate,
  secret-handling, the threat model, and a status snapshot. Wired into
  `agents/share/index.md`, the root `AGENTS.md` `@`-includes, and both
  compliance docs. Feeds OPS-1 runbooks.

---

## Suggested execution order (flattened)

**Security criticals FIRST** (before any exposed/enforced deployment):
SEC-A1 → SEC-B1 → SEC-G1 → SEC-B5 → then the F-authz forgery/fuzz proof
SEC-V4. Cheap security hygiene lands with Phase 1: SEC-I1, SEC-I3.

H-1 → H-4 → **SEC-A1 → SEC-B1 → SEC-G1 → SEC-B5** → SEC-I1 → SEC-I3 →
H-2 → H-3 → H-5 →
S-1 → P-1 → AU-1 → AU-2 → AU-3 → S-2..S-4 (parallelizable) →
P-2..P-4 (parallelizable) → PG-1 →
(F-input with search/privacy: SEC-M1 → SEC-M2 → SEC-M3 → SEC-M4 → SEC-M5 →
SEC-M6 → SEC-I2) →
(F-guard with B-auth: SEC-G2..G8) → (F-authn remainder: SEC-A2..A10) →
(F-authz remainder: SEC-V1..V3) →
LNK-1 → LNK-2 → LNK-3 → BLK-1 → BLK-2 → BLK-3 → BLK-4 → BLK-5 →
(F-data with A-bulk/A-link: SEC-B2 → SEC-B3 → SEC-B4 → SEC-B6 → SEC-B7 →
SEC-B8 → SEC-B9 → SEC-B10 → SEC-B11) →
BUS-1 → BUS-2 → BUS-3 →
DEP-1 → DEP-2 → OPS-1 (+ SEC-I4, SEC-G8) → FE-1..FE-4 →
EX-1..EX-4 → TUT-1..TUT-6 → LNK-4 (spec-first, last).

Parallelization note: S-2/S-3/S-4, P-2/P-3/P-4, AU-1's five services,
BLK-5's two services, and the per-matcher SEC-M2/M6 + SEC-I1 rollouts are
good one-subagent-per-crate fan-outs — give each agent the reference-crate
paths and the green gate verbatim, then re-verify independently before
committing (see plan.md §4).

## Found 2026-07-18 (while fixing the family-wide EntityNotFound→500 bug)

- [x] **QA-CASE-MASK (M)** *(fixed 2026-07-18)* — the test was **born
  failing** (reproduced at its birth commit c4e34443): its
  subject-only deny rule (`dept=blocked`) matched at the coarse
  blanket guard, 403ing the surface before the record-level
  concealment it meant to pin could run. Contract clarified in the
  test doc: a subject-only deny **belongs to the coarse guard**
  (defense-in-depth); SEC-G3 concealment is the property of callers
  who pass the guard and are denied on *specific records* via
  `resource.*` conditions (which, per SEC-V2, never match on the
  coarse no-record path). The test now uses a resource-scoped deny
  (`dept=blocked` + `resource.case_type=investigation`) on an
  Investigation-typed case — list concealment, native-GET 403, and
  FHIR 403 all actually exercise the record-level pass. Green vs
  Postgres 18. Bonus: the same order-dependence class was found and
  fixed in case's shared requests binary
  (`blanket_enforcement_gates_api_but_not_public_paths` duplicated
  `tests/enforcement.rs` and only passed when it ran first —
  removed; the dedicated binary owns that pin).
- [x] **QA-CP-FLAKE (S)** *(fixed 2026-07-18)* — moved to its own
  `tests/enforcement.rs` binary (the case / patient-flow pattern):
  the flag is now set before the process's only boot, so the pin is
  order-independent. Full care-pathway DB-gated suite green (1 + 22
  + 1 across the three binaries).

## Done 2026-08-01 — every service's DB suite now runs

- [x] **QA-SWEEP (M)** *(done 2026-08-01)* — ran all eight remaining
  unenrolled DB-gated suites through their new containers (DEP-0). Five
  were **already green and had simply never been run**:
  contact-relationship-management (8 tests), link-graph (16),
  patient-flow (9), place (2), thing (2). Three were red; all three are
  fixed, and **all 17 service crates are now enrolled** in
  `ci/db-suites.txt`.

  **course — 6 of 14 failing, two stacked causes.**
  - `POST /api/courses` stored an explicit all-zeros `id` verbatim.
    `Course::id` mints via `#[serde(default)]`, which only applies to an
    *absent* field — so the first create claimed the nil UUID and every
    later one died on the primary key with a `500`. The handler now mints
    on nil, matching the event service. *(Product fix.)*
  - The fixtures fought the duplicate detector: names were
    `Integration <suffix> <micros>`, and consecutive microsecond stamps
    share nearly every leading digit, scoring ~0.92 on Jaro-Winkler.
    Swapping in a UUID was not enough — the constant `Integration `
    prefix held the score at ~0.88 via the prefix bonus. Names now lead
    with the random token. The detector was right; the fixtures were
    wrong. Now 14/14.

  **event — 1 failing, a product defect.** `POST /api/events` *required*
  `created_at` / `updated_at`, which the repository sets on insert and
  refreshes on update — it demanded values it then discarded, answering
  `422 missing field created_at`. Both are now `#[serde(default)]`. The
  test also now reads the body before asserting the status, which is what
  turned "422 != 201" into a one-run diagnosis. Now 6/6.

  **portfolio — 2 failing, both test bugs; the service was right.**
  - The automation test read `moved[0]["assignee_ref"]` from
    `GET /tasks`, which answers `{ "tasks": [...], "counts": {...} }` —
    indexing an object gave `Null`, reading as "the automation never
    fired". It had: an `applied` run was logged and the row carried the
    assignee.
  - The burndown test hard-coded a July sprint window. Burndown counts
    `done_at` stamps within the window, the test completes a task *now*,
    and once now drifted past `ends_on` the completion stopped counting.
    The window is now relative to today. Now 36/36.

  Worth stating plainly: of the four defects this sweep found across the
  family (counting authentication's), **two were in shipped product code**
  and two were tests that had rotted against changes nobody could have
  noticed — because the suites had never run. The cost of a DB-gated
  suite that never runs is not zero; it is the illusion of coverage.

- [ ] **QA-SERVER-FIELDS (S)** — `POST /api/places` (and, on the same
  evidence, `thing`) **requires the fields the server owns**: `id`,
  `active`, `created_at`, `updated_at`, `keywords` and any other field
  the model declares without a serde default. Omit one and the JSON
  extractor answers `422 missing field …` before a handler runs, for a
  value the repository then overwrites. This is exactly the event-service
  defect fixed on 2026-08-01 (`missing field created_at`), and the fix is
  the same: `#[serde(default)]` on the server-managed fields. Found while
  writing place's activation proof, whose payload is built by serializing
  `Place::new` rather than by hand for precisely this reason. Not bundled
  into the auth commit — a payload-contract change is its own three-part
  change.

## Found 2026-08-01 (first run of the authentication DB suite)

- [x] **QA-AUTH-DB (M)** *(fixed 2026-08-01)* — `authentication-service`'s
  DB-gated suite had **never been run**; the containerised database
  (DEP-0) made it a one-liner, and it came up 16 pass / 22 fail. Three of
  the four causes were test rot; one was a **production defect**.

  - **Every `LOWER(email)` lookup failed against Postgres.**
    `Expr::cust_with_values("LOWER(email) = ?", …)` emits the `?`
    verbatim — a MySQL placeholder where Postgres wants `$n` — so the
    driver sent `… WHERE LOWER(email) = ? LIMIT $1` and Postgres rejected
    it with `syntax error at or near "LIMIT"`. That is `find_by_email` and
    the duplicate-account guard: **signup and magic-link sign-in returned
    500**. Replaced with sea-query's typed `LOWER()`.
  - `src/fixtures/users.yaml` never gained the ABAC `attributes` column;
    loco seeds by deserializing into the entity, so every model test
    aborted on `missing field 'attributes'`.
  - Two request tests redeemed the magic-link token they read back out of
    the database — but SEC-A9 stores only its hash. The helper now issues
    a link through `create_magic_link` and uses the plaintext it returns.
  - One test asserted the decommissioned `/.well-known/jwks.json`. It now
    asserts the PASETO key set, plus that nothing serves a key set at the
    old path — checking the *body*, because loco's fallback middleware
    answers unmatched routes with `200` and a status check would pass
    either way.

  38/38 green vs Postgres 18; crate enrolled in `ci/db-suites.txt`.

- [ ] **QA-CUST-SQL (S)** — the same `cust_with_values` footgun may be
  latent in **person**, **worker**, and **event**
  (`src/db/repositories.rs`: `LOWER(family|name) LIKE $1`). Those spell
  the placeholder `$1` rather than `?`, so they may well be fine — but
  the repository `search()` they sit in is **not called by any handler**
  (the handlers search Tantivy) and **not covered by any test**, so
  nothing would notice either way. Decide per crate: exercise it or
  delete it. Not touched here, because each needs its own verification
  against a database and this was not the change to bundle it into.

## Done 2026-08-01 — a containerised test database per service

- [x] **DEP-0 (M)** *(done 2026-08-01)* — **`compose.test.yaml` in all 17
  service crates**: one `postgres:18-alpine` container each (Podman, not
  Docker), providing exactly what that crate's DB-gated suite needs and
  matching what CI provides (`.github/workflows/ci.yml` `test-db`):
  superuser `loco`/`loco`, port 5432, the database its `config/test.yaml`
  names. Driven by the new **`scripts/test-db.sh`**
  (`up`/`down`/`psql`/`logs`/`url`/`status`/`down-all`), which waits on
  the container healthcheck instead of sleeping. Extensions come from one
  shared init script (`ci/postgres-init/`, mounted read-only into every
  container) that enables them in **`template1`**, so the `ci_*` databases
  `ci-check.sh` creates per crate inherit them. PGDATA is on **tmpfs**:
  every `up` is a fresh `initdb`, and a test database that accumulates
  state is the difference between a real failure and a stale one.

  `scripts/ci-check.sh test-db` gained **`DB_SUITES_FORCE=1`**, which runs
  an unenrolled crate anyway — the missing half of the `ci/db-suites.txt`
  rule that a crate is enrolled *once observed green*. Together these are
  what unblocks enrolling the nine services still outside that allowlist.

  *Verified:* all 17 compose files parse, and all 17 containers were
  started, reported healthy, and served the 5 expected extensions. Four
  suites were then run end to end through a container —
  **organization 22/22, person 38/38, link-graph 16/16, place 2/2** — and
  side-by-side operation (`TEST_DB_PORT=5434`), `status`, and `down-all`
  were exercised.

  Three findings worth keeping:
  - **A healthy container is not a reachable one.** On macOS podman
    publishes on IPv6 `*`, so a Postgres already holding IPv4
    `127.0.0.1:<port>` answers `localhost` first: the container is
    healthy, the connection succeeds, and the error is "database does not
    exist" — which reads like a broken container. Hit while testing the
    second-port path on this machine. `test-db.sh up` now probes the
    published port from outside and says so.
  - **The old `docker-compose.test.yml` files (person / worker / event)
    were wrong in three ways** and are removed: credentials
    (`test_user`/`test_password`) matched neither CI nor `config/test.yaml`;
    the tmpfs mount was at `/var/lib/postgresql/data`, which the 18 image
    does not use (PGDATA is `/var/lib/postgresql/18/docker`), so it
    silently did nothing; and their `test-runner` service built a
    `Dockerfile.test` pinned to Rust 1.93 against a repo pinned to 1.96.1,
    running a `cargo test --test api_integration_test` command that no
    longer describes the suite. The three `Dockerfile.test` files went
    with them.
  - **Six crates' `config/test.yaml` disagreed with everything else.**
    person / worker / event / place / thing defaulted to
    `postgres://localhost/<db>` (no credentials — implicitly the
    developer's OS user), and case-folder to `postgres:postgres`. All six
    now default to `loco:loco@localhost:5432`, so config, container, and
    CI finally name the same connection.

  **Found here, fixed next** (see QA-AUTH-DB above):
  `authentication-service`'s DB-gated suite was **red — 16 pass, 22
  fail** — the first time it had ever been run. One of the causes was a
  production defect in the signup / sign-in path.

## Found 2026-07-31 (during the doc harmonization pass)

- [x] **FE-LILY-RENAME (M)** *(done 2026-07-31)* — Lily renamed its
  two helper packages upstream: `-locale-select` → `-locale-picker`
  and `-theme-select` → `-theme-picker`, components likewise
  `LocaleSelect`/`ThemeSelect` → `LocalePicker`/`ThemePicker`. All 15
  older front-ends declared the old `file:` paths, which no longer
  exist, so a fresh `pnpm install` failed in every one of them. They
  built only because pnpm's store still held a copy.

  **All 16 front-ends are now on the pickers** (the 15 plus the CMS
  client, which was already there). Verified: `pnpm install` resolves
  the new packages everywhere with no stale `-select` directory left;
  `svelte-check` clean in all 16; **467 unit tests** pass; all 16
  build; **15 of 16 Playwright suites pass** — case-folder's is
  skipped for the pre-existing reason that its pre-flight needs the
  live Rust backend.

  It was not a pure rename, as expected. Three things had to change
  beyond the identifiers:
  - **Chrome CSS.** The old component rendered a `<select>` inside a
    `.theme-select` / `.locale-select` root. The picker renders a
    button plus a `ul` listbox, so every `:global(select)`,
    `:global(.theme-select)` and `:global(.locale-select)` rule was
    styling nothing. Repointed at `.theme-picker-button` /
    `.locale-picker-button` in 11 layouts.
  - **Playwright.** Two suites drove `select.locale-select` with
    `.selectOption()`, which cannot operate a listbox. Both now use a
    `chooseLocale` helper.
  - **case-folder's tracked `package-lock.json`** carried the old
    paths alongside its `pnpm-lock.yaml`.

  Two findings worth keeping, both discovered by running the thing
  rather than reading it:
  - The options carry **no `lang` attribute** and the **theme picker
    renders `li[role="option"]` too**, so a bare
    `li[role="option"][lang="de"]` selector matches nothing while
    `li[role="option"]` matches 58 elements. The list must be scoped
    (`ul.locale-picker-list`) and the option matched by its label.
  - **The picker stays open after a pointer selection**, so a test
    that clicks the button again to make a second choice *closes* it.
    The helper opens only when collapsed, which is correct either way.
    Reported upstream; Lily shipped a fix on 2026-07-31 for the
    related **effect re-entrancy** (`onChange` writing reactive state
    re-entered the apply effect, hitting
    `effect_update_depth_exceeded` and freezing the component with a
    stale `aria-expanded`). That fix is now in every front-end — see
    FE-LILY-REFRESH — and the freeze is gone; the list still being
    expanded after a click persists, and the helper is agnostic to
    it.

- [x] **FE-LILY-REFRESH (S)** *(done 2026-07-31)* — refreshed all 16
  front-ends onto Lily's fixed pickers after the upstream repair of
  the apply-effect re-entrancy (`onChange` writing reactive state
  re-entered the effect, hitting `effect_update_depth_exceeded` and
  freezing the listbox with a stale `aria-expanded`; the fix guards on
  an `appliedValue` so re-applying is idempotent).

  Two mechanical notes for the next time upstream changes:

  - **`dist/` is what consumers get.** The package `exports` point at
    `./dist/index.js`, and the fix landed in the package-root source
    only, so the front-ends would have kept the old behaviour. Run
    `npm run build` in `lily-design-system-svelte-helpers` first;
    upstream's own suite (211 tests) passing does not mean `dist` was
    rebuilt.
  - **`pnpm install --force` is not always enough** for a `file:`
    dependency: pnpm reuses the content-addressed store entry, and
    three apps kept the stale copy until `node_modules` was removed
    and reinstalled. Verify by diffing the installed
    `dist/LocalePicker.svelte` against upstream's rather than trusting
    the install to have done it.

  Verified: all 16 installed copies byte-identical to upstream's
  `dist` for both pickers; `svelte-check` clean; 467 unit tests pass;
  15/16 Playwright suites pass (case-folder needs its live backend);
  no `effect_update_depth_exceeded` in the browser.
