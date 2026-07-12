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

- [ ] **H-2 (M)** Make `agents/share/overview.md` honest: replace the
  "What every crate provides" list with a per-crate **capability matrix**
  (capabilities × crates, ✅ / – / planned). Ground it by grepping the
  tree (Tantivy: only person/worker/place/thing/event/course; privacy:
  same six; FHIR: seven + person under `src/api/fhir`; gRPC: stubs).
  Update as later tasks land.
  *Verify:* every ✅ has a corresponding `src/` module; no overclaims.

- [ ] **H-3 (M)** Rewrite `agents/share/architecture.md`. It still
  describes the pre-loco person layout (FHIR endpoint counts, module
  tree). Describe the current family: loco Hooks services, the two
  internal shapes (case-style vs person-style), outbox/relay, the
  aggregator, auth stack. Keep it one page + one diagram.
  *Verify:* no named file/module in the doc is absent from the tree.

- [ ] **H-4 (M)** Roll CI `--include-ignored` to every service CI.
  Case's `.github/workflows/ci.yaml` test job is the pattern (Postgres
  service + `cargo test --all-features --all -- --include-ignored`).
  Apply to the other nine service crates' workflows (and link-graph).
  Check each service's `config/test.yaml` DB name matches its CI env.
  *Verify:* per-crate workflow lints (yamllint or careful review); the
  DB-gated suites at least compile locally (`cargo test --no-run`).

- [ ] **H-5 (M)** Release hygiene: cut CHANGELOG releases for crates with
  large `[Unreleased]` sections (person, case, care-pathway, organization,
  portfolio, link-graph, authentication-service, authentication-verifier,
  entity-ref); tag (`<crate>-vX.Y.Z`); decide/execute crates.io publish
  for `entity-ref` and `authentication-verifier` 0.8 (both are
  dependency-light and publishable).
  *Verify:* CHANGELOGs have dated release headings; tags pushed to both
  remotes.

## Phase 2 — Capability completion (four newest loco services)

- [ ] **S-1 (L)** Tantivy full-text search in **organization**.
  Reference: any older service's `src/search/` (index, query, engine
  wrapper) — but adapt the index lifecycle to loco (index path from
  config; index writes after create/update/delete/merge; rebuild task).
  Replace ILIKE search handler; add fuzzy + phonetic per
  `agents/share/search.md`; keep the REST contract (`?q=`).
  Also wire **search-blocked candidate selection** into
  check-duplicates (drop the scan-cap warning path).
  *Verify:* green gate; DB-gated search round-trip test; spec §13 +
  CHANGELOG; H-2 matrix updated.

- [ ] **S-2 (L)** Tantivy in **care-pathway** (as S-1). Depends: S-1
  (copy its loco-adapted pattern, not the pre-loco one).
- [ ] **S-3 (L)** Tantivy in **case** (as S-1). Depends: S-1.
- [ ] **S-4 (L)** Tantivy in **portfolio** — note the kind gate: index
  `kind` as a field and filter search/dedup within-kind. Depends: S-1.

- [ ] **P-1 (M)** Privacy module in **organization**: masking
  (`mask_*` copy-adapt from person's `src/privacy/`), masked-view
  endpoint (`GET /{id}/masked`), GDPR export (`GET /{id}/export`),
  consent model if applicable per `agents/share/privacy.md`. Wire the
  ABAC `mask` obligation like case's `GET` handler (case is the
  obligation reference).
  *Verify:* green gate; masked fields pinned in unit tests; spec/CHANGELOG.
- [ ] **P-2 (M)** Privacy in **care-pathway** (as P-1; clinical data —
  mind `compliance-for-healthcare.md`). Depends: P-1.
- [ ] **P-3 (M)** Privacy in **case** — it already honours the `mask`
  obligation; add the masked-view + GDPR-export endpoints on top of the
  existing `mask_case`. Depends: P-1.
- [ ] **P-4 (M)** Privacy in **portfolio** (lower sensitivity; masking of
  owner/person refs). Depends: P-1.

- [ ] **AU-1 (M)** Roll the case-only auth hardening to **person, worker,
  place, thing, event** (axum-style `src/api/rest/auth.rs`): key-rotation
  refresh loop (`ReloadableVerifier` + `spawn_key_refresh` — case
  `src/auth.rs` is the pattern), policy hot-reload
  (`ReloadablePolicy` + `spawn_policy_watcher`), and a per-service
  `tests/enforcement.rs` activation proof (case's is the template; each
  runs in its own test binary).
  *Verify:* per-service green gate; jwt-enforcement.md status updated.
- [ ] **AU-2 (M)** Same for **organization, care-pathway, course,
  portfolio** (loco-style `src/auth.rs`). Depends: AU-1 (shared doc
  wording once).
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

- [ ] **LNK-1 (M)** Envelope `data` field + `Linked`/`Unlinked` kinds in
  **person** (case's `src/streaming.rs` is the pattern; person's envelope
  is `src/streaming/envelope.rs`). Must stay additive —
  `skip_serializing_if = "Option::is_none"`; existing byte-identical
  wire-shape tests must keep passing. Then emit person's `linked` /
  `unlinked` events from the links handlers (currently deferred — see
  `src/api/rest/links.rs` module doc).
  *Verify:* green gate; the §4.2 `data` matches the aggregator's
  `LinkedEvent` (add a seam unit test like link-graph's).
- [ ] **LNK-2 (M)** **Worker** `same_identity` write-side: mirror
  person's (`entity_links` migration/model, `validate_edge` accepting
  only `same_identity` worker→person, per-record + bulk endpoints,
  canonical `EdgeDetail`). Add worker to the aggregator's reconcile
  entity list + a seam test. Symmetric double-assert is by design.
  *Verify:* worker green gate; aggregator seam test green.
- [ ] **LNK-3 (M)** Affiliation edges: `works_at`/`member_of` on person,
  `employed_by` (with `role`) on worker — same tables, extend each
  `validate_edge` permit set per the §9 registry; bulk endpoints already
  generic. Depends: LNK-2.
- [ ] **LNK-4 (L)** Cross-service `same_identity` **matcher + review
  queue** (design §5.2, roadmap): a job comparing person↔worker records
  (reuse matcher components), emitting `matcher_suggested` edges
  (confidence < 1.0) into a review surface; operator confirm promotes to
  `operator`/1.0. **Spec it first** (link-graph §16 + a new §13 task
  chain) — do not start coding without a spec round. Depends: LNK-1..3.

- [ ] **BLK-1 (M)** Bulk I/O step 2a — **CSV** import/export on person:
  the §5 flattening convention (scalars → columns; nested-single →
  dotted; arrays → JSON-in-cell); document person's exact column set in
  its spec (§10 declaration). Codec unit-tested round-trip.
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

- [ ] **DEP-1 (M)** `examples/compose/`: podman-compose for (a) one
  service + postgres, (b) the full family (10 services + auth +
  link-graph + postgres), (c) the enforced variant (auth on, policies
  mounted, reconciliation configured). Compose is also what tutorials and
  the bus-gated tests build on.
- [ ] **DEP-2 (M)** `agents/share/configuration.md`: the complete env-var
  reference — every `<ENTITY>_*`, `LINK_GRAPH_*`, `AUTH_*` variable, its
  default, effect, and which doc governs it. Generated by sweeping
  `std::env::var` call sites; keep a per-service table.
- [ ] **OPS-1 (M)** Runbooks under `agents/share/runbooks/`: auth
  activation checklist (expand jwt-enforcement.md), key rotation,
  reconciliation-divergence response, event-bus outage/replay, bulk-job
  failure recovery. Each: symptoms → checks → actions → verification.

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

---

## Suggested execution order (flattened)

H-1 → H-4 → H-2 → H-3 → H-5 →
S-1 → P-1 → AU-1 → AU-2 → AU-3 → S-2..S-4 (parallelizable) →
P-2..P-4 (parallelizable) → PG-1 →
LNK-1 → LNK-2 → LNK-3 → BLK-1 → BLK-2 → BLK-3 → BLK-4 → BLK-5 →
BUS-1 → BUS-2 → BUS-3 →
DEP-1 → DEP-2 → OPS-1 → FE-1..FE-4 →
EX-1..EX-4 → TUT-1..TUT-6 → LNK-4 (spec-first, last).

Parallelization note: S-2/S-3/S-4, P-2/P-3/P-4, AU-1's five services, and
BLK-5's two services are good one-subagent-per-crate fan-outs — give each
agent the reference-crate paths and the green gate verbatim, then
re-verify independently before committing (see plan.md §4).
