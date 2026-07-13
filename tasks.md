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
- [ ] **SEC-A2 (S) 🟠** Gate `GET /api/auth/audit/recent`
  (`controllers/auth.rs:514-518`, route `:605`) — it is unauthenticated and
  returns raw `auth_events` emails/outcomes (account enumeration). Require
  auth (admin) and/or strip emails. *Test:* unauth ⇒ 401/redacted; pins the
  anti-enumeration contract.
- [ ] **SEC-A3 (S) 🟠** Magic-link token logging (`controllers/auth.rs:127-132`)
  emits the full login URL+token at `info` in every env. Log the token only
  in `development` (env/level gate). *Test:* tracing-capture asserts no token
  at `info` in production.
- [ ] **SEC-A4 (S) 🟡** Atomic single-use magic-link consume: replace
  SELECT-then-clear (`controllers/auth.rs:284-298`, `models/users.rs:254-283`)
  with `UPDATE … WHERE magic_link_token=$1 RETURNING`. *Test:* two concurrent
  redemptions ⇒ exactly one 200, one 401 (DB-gated).
- [ ] **SEC-A5 (S) 🟡** Constant-work signup: always run one Argon2 hash so
  existing-vs-new email latency does not distinguish (`controllers/auth.rs:167-197`).
  *Test:* both paths perform equivalent hashing work.
- [ ] **SEC-A6 (S) 🟡** Rate-limit email canonicalization + case-consistent
  `find_by_email` (`rate_limit.rs:60-62`, `models/users.rs:438-471`) —
  plus-address/dot variants bomb one inbox and spawn duplicate accounts.
  *Test:* `victim+1@…`/`v.ictim@gmail.com`/`Victim@…` collapse to one bucket.
- [ ] **SEC-A7 (S) 🟡** GDPR erasure completeness (`models/users.rs:606-614`):
  scrub the subject email from `auth_events` and `sessions.user_agent`, not
  just `users`. *Test:* after `DELETE /account`, email absent everywhere.
- [ ] **SEC-A8 (M) 🟡** Privilege-revocation latency: `POST /token` mints
  from a session `attrs` snapshot (`controllers/auth.rs:430-437`,
  `models/sessions.rs:224-229`) never refreshed on admin revoke. Re-read
  attrs (or invalidate sessions) on privilege change. *Test:* clearing
  `access=admin` stops the session minting admin tokens.
- [ ] **SEC-A9 (M) ⚪** Store only **hashes** of magic-link token / session
  `jid` / CSRF token (migrations `_000001`/`_000002`, `sessions.data.csrf`) —
  they are bearer-equivalent secrets at rest today. *Test:* DB holds no
  usable plaintext credential.
- [ ] **SEC-A10 (S) ⚪** CSRF origin backstop: warn/deny when
  `AUTH_ALLOWED_ORIGINS` unset in production; reject a legacy no-`csrf`
  session on `POST /token` (`controllers/auth.rs:377-410`). *Test:* no-csrf
  session cannot bypass both CSRF and origin checks.

### F-authz — verifier & ABAC (authentication-verifier)

- [ ] **SEC-V1 (S) 🟡** `from_paseto_keys_url` (`src/lib.rs:460-474`): enforce
  `https://`, a request timeout, and a response-size cap (MITM key injection
  / boot-hang / OOM). *Test:* `http://` rejected; oversized/hanging body errors.
- [ ] **SEC-V2 (M) 🟡** Vacuous-negation escalation (`src/abac.rs:223-293`):
  a `!`-negated `resource.`/`env.` condition matches on the coarse
  `evaluate` path because the namespace is absent there. Treat absent
  namespace as non-matching for negated conditions (fail-closed), or warn at
  policy-load when a rule negates a `resource.`/`env.` value. *Test:*
  `{allow,[write],when:{"env.network":["!untrusted"]}}` is **denied** on the
  coarse path (currently allows).
- [ ] **SEC-V3 (S) ⚪** Key-set load resilience (`src/lib.rs:279-300`): a
  single malformed Ed25519 entry aborts the whole load; skip+warn instead,
  and define dup-`kid` policy. *Test:* mixed valid/invalid entries still load
  the valid keys.
- [ ] **SEC-V4 (M) 🟠 (tests)** Forgery + fuzz + policy-property suite:
  cross-key forgery (valid sig, honest `kid`) ⇒ Err; missing-`exp` ⇒
  Malformed; `exp==now` reject / `nbf==now` accept; token-parser fuzz (never
  panics); policy proptest (first-match, negation involution, default
  decision, `resource.`/`env.` disjointness). Adds `proptest` +
  `cargo-fuzz` targets to the crate.

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
- [ ] **SEC-G2 (M) 🟠** Case FHIR read/search (`case/src/controllers/fhir.rs:90,248`)
  take no caller — apply record-level ABAC + the `mask` obligation like
  native `get_one`. *Test:* masked/denied read on `/fhir/Task/{id}` + search.
- [ ] **SEC-G3 (M) 🟠** Masking-on-every-read: case `list`/`search`/
  `check_duplicates` (`cases.rs:307,324,370`) and person `search_persons`
  (`api/rest/handlers.rs:427-465`, currently client-param driven) must honour
  record-level authz/mask, not just single GET. *Test:* a mask-only policy
  redacts on **every** read path (get/list/search/check-dup/FHIR/export).
- [ ] **SEC-G4 (S) 🟡** Add `escape_like` to the three repo-based searches
  (person `db/repositories.rs:1181`, worker `:1388`, event `:1218`) — raw
  `%{q}%` allows `LIKE`-wildcard injection/DoS. *Test:* `q="%"`/`"_"`/`"a\%b"`
  match literally (port the loco `escape_like` test).
- [ ] **SEC-G5 (M) 🟡** Guard-all for event/thing/course (`auth.rs` prefix-gate
  → deny-unless-public), plus a guard-bypass matrix test across all services
  (trailing slash, `%2e`, case, `//`, `/../`). *Test:* guard decision and the
  actually-routed handler agree; no reachable path the guard treats as public.
- [ ] **SEC-G6 (S) 🟡** Destructive-action classification robust to path
  variants: `derive_action` uses `path.ends_with("/merge"|…)` — a trailing
  slash downgrades to `write`. Normalize before matching. *Test:*
  `POST /api/cases/merge/` (and any router-accepted variant) ⇒ `destructive`
  ⇒ `access=write` gets 403.
- [ ] **SEC-G7 (S) ⚪** Bound person `search_persons` `offset`
  (`handlers.rs:436-441`) — unbounded `offset+limit` forces the index to
  materialise arbitrarily many hits. *Test:* large offset clamped/rejected.
- [ ] **SEC-G8 (S) 🟡** Default-off exposure pin: an explicit per-service test
  documenting that with `<ENTITY>_REQUIRE_AUTH` off, audit / bulk-links / PII
  reads are open — so activation is a **tracked release gate**, not an
  accident (feeds OPS-1 runbook).

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
- [ ] **SEC-B2 (M) 🟠** person bulk import caps: byte cap + row cap + true
  streaming (`bulk/handlers.rs:175`, `bulk/jsonl.rs:57-65`, `pipeline.rs:188`)
  — currently 3× resident, unbounded ⇒ OOM DoS; export `limit` also uncapped.
  *Test:* oversized upload ⇒ 413/422 pre-materialisation; fuzz `parse_line`
  (random bytes / truncated UTF-8 / giant line) never panics.
- [ ] **SEC-B3 (M) 🟠** person bulk upsert idempotency race: SELECT-then-
  INSERT with no `ON CONFLICT` and no `UNIQUE(system,value)`
  (`pipeline.rs:216,236-246`, `db/repositories.rs:883-890`) ⇒ two workers
  duplicate a record. Add the unique index + `ON CONFLICT`/advisory lock.
  *Test:* two concurrent imports of one stable key ⇒ exactly one row.
- [ ] **SEC-B4 (M) 🟠** person bulk artifact hardening (`bulk/store.rs:75-99`,
  `db/bulk_jobs.rs:94`, `handlers.rs:403-471`): set + enforce `expires_at`
  TTL; ownership check on job GET (IDOR/BOLA); confine store path (reject
  `..`/absolute `file://`). *Tests:* cross-actor job GET ⇒ 403/404; `..`
  rejected; expired artifact unreadable.
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
- [ ] **SEC-B6 (M) 🟠** Relay exactly-once: `SELECT … WHERE published_at IS
  NULL` has no `FOR UPDATE SKIP LOCKED` (case `relay.rs:91`, person `:93`)
  and no consumer `event_id` dedupe (person consumer is `todo!()`), so >1
  instance double-ships. Add `SKIP LOCKED` + a `processed_events`
  idempotency table (aligns with BUS-2). *Test:* two concurrent drains send
  each row once; replayed `event_id` ignored.
- [ ] **SEC-B7 (S) 🟡** link-graph reconcile peer trust
  (`reconcile.rs:94-155`): `LINK_GRAPH_RECONCILE_TOKEN` is optional (unauth
  pull) and returned edges are applied directly. Require the token for a
  remote source; validate each edge's endpoint types via `EdgeKind::permits`
  before `apply_linked`. *Test:* remote source without a token refused;
  ill-typed edge rejected.
- [ ] **SEC-B8 (S) 🟡** Bulk audit gaps: import runs with
  `AuditContext::default()` (no actor/ip) and writes no job-level audit row
  (`bulk/worker.rs:86-121`); export audit is best-effort and delivers the
  artifact even if `log_export` fails (`worker.rs:199-206`). Thread real
  actor context; write a job-level import audit; make export audit block
  delivery. *Tests:* import/export audited with actor; export fails closed on
  audit error.
- [ ] **SEC-B9 (S) 🟡** Wire the idempotency key (`db/bulk_jobs.rs:48,61`
  hardcode `None`; the `UNIQUE(entity,kind,idempotency_key)` never fires):
  read a client key and dedupe a retried submit. *Test:* re-submit with the
  same key returns the same job, no re-run.
- [ ] **SEC-B10 (S) 🟡** person merge audit in-tx (`repositories.rs:1082,1108-1128`
  writes post-commit) — match case's in-tx threading so a crash after commit
  cannot lose the merge audit. *Test:* merge audit present atomically.
- [ ] **SEC-B11 (S) ⚪** link-graph `freshness` authz (`controllers/graph.rs:353-367`
  — unauth liveness oracle) + non-redirecting probe client + host allowlist
  (`probe.rs:98` SSRF-via-redirect). *Tests:* freshness gated; probe refuses
  a redirect.

### F-input — unverified input, false matches & fuzzing (validators + matchers)

- [ ] **SEC-M1 (M) 🟠** Input-size caps: per-field length + array-cardinality
  caps in every service `validate`/`problems` → `422` **before** persist;
  set `limit_payload` on the five loco services that set none (course, org,
  care-pathway, case, portfolio) and lower the 5 MB cap. Closes the O(n·m)
  Jaro-Winkler/Levenshtein/Jaccard DoS amplified ×scan-cap. *Tests:*
  over-length field / over-cardinality array ⇒ 422 before the matcher runs.
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
- [ ] **SEC-M5 (S) 🟡** organization identifier validation
  (`controllers/organizations.rs:90-98` validates only `name`): enforce
  LEI/DUNS/GLN/VAT check-digit/length before store, since they drive the
  matcher's deterministic short-circuit. *Tests:* bad check digit ⇒ 422.
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

- [ ] **SEC-I1 (M) 🟠** Roll `security.yml` (`cargo audit` + `cargo deny` +
  dependency-review; person's is the pattern) to the **9** services + the
  matcher/library crates that lack it, and add a repo-root `deny.toml`
  (advisories + licenses + source/ban rules). *Verify:* each workflow lints;
  `cargo deny check` passes locally.
- [ ] **SEC-I2 (M) 🟡** `cargo-fuzz` scaffolding: a `fuzz/` crate per matcher
  (+ auth-verifier token parser + person bulk `parse_line`) with the SEC-M6
  targets; a short CI smoke run + an optional nightly longer run. Depends:
  SEC-M6 targets.
- [ ] **SEC-I3 (S) ⚪** Add `#![forbid(unsafe_code)]` to the three crate roots
  missing it (care-pathway-matcher `src/main.rs`, case-folder `src/lib.rs` +
  `src/bin/main.rs`). *Verify:* builds clean; grep shows full coverage.
- [ ] **SEC-I4 (M) 🟡** `agents/share/security.md`: the audit summary, the
  cross-cutting invariants (never-panic / masking-on-every-read / fail-closed
  authz / secret-handling / no-secret-in-logs), the `*_REQUIRE_AUTH`
  activation gate, and the threat model. Wire into `agents/share/index.md`
  and the compliance docs. Feeds OPS-1 runbooks.

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
