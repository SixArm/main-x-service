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
  + clippy clean. Follow-ups: worker `employed_by` (LNK-3), `linked`/
  `unlinked` events (LNK-1-style), matcher-partition guard test.
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
