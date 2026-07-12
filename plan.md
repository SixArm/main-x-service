# Main X Index — Improvement Program Plan

> **What this is.** A repo-level program plan for the next wave of
> improvements across capabilities, functionality, documentation,
> tutorials, and examples. It is a **handoff artifact**: written for a
> future Claude (Opus) session to execute, task by task, from the
> companion [tasks.md](tasks.md).
>
> **Relationship to the SDD discipline.** Each crate's `spec/index.md`
> (§13) remains the per-crate single source of truth. The per-crate rule
> "no plan.md / no tasks.md" still holds *per crate* — these two files are
> **repo-root, cross-cutting program docs**. Every task that changes a
> crate's behaviour must still land as a three-part change (spec edit +
> code edit + test edit) inside that crate, with its §13 updated. Do not
> create plan/tasks files inside any crate.

## 1. Current state (as of 2026-07-11, `main` @ 61ed6c67)

Recently completed (this quarter's arc):

- **Auth**: family-wide ABAC (verifier 0.8: attrs claim, policy engine,
  resource/env attributes, obligations, hot-reloadable policy+verifier),
  blanket `/api/*` enforcement wired default-off in all services, CSRF +
  sessions reshape + BFF plumbing, operator attribute UI, DB-backed
  enforcement e2e (case = reference), activation playbook.
- **Durable event bus, Phase 2**: transactional outbox + relay (with a
  `LoggingSink`) in **all ten** services; audit joins the outbox
  transaction; `EventTransport` switch (`memory` default, `outbox`).
  Phase 3 (Fluvio) not started — no crate has a live Fluvio sink.
- **Cross-service linking**: `entity-ref` contract crate; the
  `link-graph-service-with-loco` aggregator (reads, merge-repointing,
  governance concealment + audit + blanket guard, OpenAPI, metrics, lazy
  verify-on-read, reconciliation live over HTTP); write-sides on **case**
  (`subject_of`) and **person** (`same_identity`) with canonical
  bulk-links endpoints the aggregator reconciles.
- **Bulk import/export (person = reference)**: `bulk_jobs` + `bg_pg`
  worker + JSONL import (idempotent upsert-by-stable-key, per-row error
  report) + export (masked-by-default, elevated-authz gate for
  full/soft-deleted, per-export audit).

Known gaps (verified against the tree, not memory):

| Gap | Where |
|---|---|
| Tantivy full-text (ILIKE only) | organization, care-pathway, case, portfolio |
| Privacy module (masking/GDPR/consent) | organization, care-pathway, case, portfolio |
| Fluvio: live sink + consumer | all 10 services (sink), link-graph (consumer); fluvio deps in 5 older crates are dormant |
| Envelope `data` field + `Linked`/`Unlinked` kinds | only case has it; person's link events deferred because its envelope can't carry §4.2 data |
| Worker `same_identity` write-side | worker service (person side landed) |
| Key-rotation refresh, policy hot-reload, enforcement e2e, CI `--include-ignored` | **case only** — the other services still boot-once / boot-only-policy / no e2e / skip `#[ignore]` in CI |
| Bulk I/O steps 2/4/5 | CSV+review-routing, Parquet, other entities, S3 store |
| Pagination | loco services use hard caps (LIST_CAP=100, SEARCH_CAP=50) |
| Front-end merge/link/bulk/review UIs | org/case/portfolio front-ends lack merge; no UI anywhere for links, bulk jobs, or the review queue |
| Stale shared docs | `agents/share/architecture.md` (describes the old person layout), `overview.md` capability list (claims Tantivy/privacy/gRPC family-wide) |
| No tutorials, no sample data, no family compose | repo-wide |

## 2. Themes

### Theme A — Platform capabilities (close the deferred core)

**A-bus. Durable bus Phase 3 (Fluvio).** The outbox/relay seam exists in
all ten services with a `LoggingSink`; the design
(`agents/share/event-bus.md` §5, §8) calls for a feature-gated
`FluvioSink` and per-entity transport flip, plus the link-graph Fluvio
consumer (spec T-6) with `processed_events` idempotency. Do this
**once in a reference service (case) + the aggregator consumer**, prove
it with a compose-based bus-gated test tier, then roll mechanically.
This retires lazy verify-on-read per entity and makes the aggregator
event-driven rather than reconciliation-driven.

**A-link. Finish the linking backbone.** (1) Add the envelope `data`
field + `Linked`/`Unlinked` kinds to the remaining nine services (case is
the pattern; person's deferred emission unblocks). (2) Worker
`same_identity` write-side mirroring person's (symmetric assert; the
aggregator canonicalises). (3) Affiliation write-sides (`works_at`,
`member_of` on person; `employed_by` on worker) — same table, new
permitted kinds. (4) Roadmap: cross-service matcher producing
`matcher_suggested` `same_identity` edges + review workflow (§5.2) —
large; spec it before building.

**A-search. Tantivy in the four newest loco services.** organization,
care-pathway, case, portfolio still use ILIKE. The six older services
have working Tantivy (`src/search/`) to copy from. This also unblocks
"search-blocked dedup candidates" (the check-duplicates scan cap).

**A-privacy. Privacy module in the four newest loco services.**
Masking + GDPR export + consent per `agents/share/privacy.md`, with the
case service's ABAC `mask` obligation as the enforcement hook (already
proven there). Person/worker/place/thing/event/course have `src/privacy/`
to copy from.

**A-bulk. Bulk I/O steps 2–5.** CSV (import+export, §5 flattening
convention), keyless-row→review-queue routing, Parquet export
(feature-gated), S3-compatible artifact store, then roll to organization
and case first (their stable keys are already designed: LEI/DUNS/pid,
agency case number/pid).

### Theme B — Functionality & operational hardening

**B-auth. Family rollout of the case-only auth hardening.** Key-rotation
refresh loop (`ReloadableVerifier` + `spawn_key_refresh`), policy
hot-reload (`ReloadablePolicy` + file watcher), a DB-gated
`tests/enforcement.rs` activation proof, and CI `--include-ignored` — to
the other eight entity services and the link-graph. All four patterns are
documented in the case crate; this is mechanical replication with
per-service verification.

**B-page. Pagination.** Replace the loco services' hard list/search caps
with offset+limit (matching the older services' §dataflow contract), and
update front-ends. Keep response shapes backward compatible.

**B-fe. Front-end functionality.** Merge actions where missing
(organization, case, portfolio); a links panel (person↔worker,
case→person) in person/worker/case front-ends; a bulk import/export
screen (upload, dry-run, error-report download) in the person front-end;
a duplicate-review queue screen for services exposing it.

**B-obs. Observability completion.** link-graph OTLP tracing (spec T-22)
+ `user_ip` capture on governed audits; confirm every service's
`/metrics.prom` inventory matches its AGENTS doc.

### Theme C — Documentation (make the docs true, then make them good)

**C-truth. Fix stale shared docs first** — they mislead every future
agent session. `agents/share/architecture.md` still describes the
pre-loco person layout; `overview.md`'s "What every crate provides"
overclaims (Tantivy/privacy/gRPC are not family-wide). Replace the
overclaim with an honest per-crate **capability matrix** (rows =
capabilities, columns = crates, ✅/–/planned) that Theme A updates as it
lands.

**C-deploy. Deployment & configuration reference.** One doc: every env
var in the family (the `<ENTITY>_*`, `LINK_GRAPH_*`, `AUTH_*` sets) with
defaults and effects; plus podman-compose files (single service; full
family + auth + link-graph; enforced variant).

**C-ops. Runbooks.** Auth activation (expand the jwt-enforcement.md
playbook into a checklist), key rotation, reconciliation divergence
response, event-bus outage/replay, bulk-job failure recovery.

**C-rel. Release hygiene.** Cut CHANGELOG releases (Unreleased sections
are large), tag, decide crates.io publishes (`entity-ref` 0.1,
`authentication-verifier` 0.8). Add `rust-toolchain.toml` to pin the
toolchain (a rustfmt version drift caused repo-wide fmt churn this
quarter — pin to prevent recurrence).

### Theme D — Tutorials (task-oriented, runnable end-to-end)

Six tutorials under a new top-level `tutorials/` directory, each a single
markdown file with copy-pasteable commands verified against the compose
files from C-deploy:

1. **Getting started** — run one service + its front-end locally.
2. **Identity lifecycle** — create → duplicate-detect (409) → match →
   merge → audit, via curl and the UI.
3. **Authentication & ABAC** — magic link → session → PASETO → protected
   call; write and hot-reload a policy; obligations (masked read).
4. **Cross-service linking** — subject_of + same_identity →
   `neighbors` / `single-view`; watch reconciliation repair a divergence.
5. **Bulk import/export** — JSONL fixtures, dry-run, error report,
   masked vs full export.
6. **Event bus** — outbox rows, the relay, `/events/recent`; (Fluvio
   once A-bus lands).

### Theme E — Examples & sample assets

- `examples/data/` — realistic-but-synthetic JSONL fixtures (persons,
  organizations, cases; no real PII), import-ready, used by tutorial 5.
- `examples/policies/` — an ABAC policy cookbook: dept-scoped read,
  closed-case write-deny, after-hours deny, ownership (`$sub`),
  masked-read obligation, machine-peer (`svc`) grants; each with a
  one-line "what it demonstrates".
- `examples/compose/` — the podman-compose files from C-deploy.
- `examples/api/` — per-service `.http`/curl scripts exercising the main
  endpoints (complementing Swagger).
- A `seed` loco task (or SQL) loading the sample data for demos.

### Theme F — Security hardening & assurance

Driven by a **repo-wide security audit (2026-07-12)** that read the auth
stack, the offline verifier + ABAC engine, every entity service's guard /
masking / query layer, the matcher libraries + validators, and the bulk /
cross-service-linking / concurrency / secrets surfaces. The full findings
are enumerated as **Phase 5 (SEC-\*)** in [tasks.md](tasks.md); the shape:

- **F-authn. Token & session integrity.** The single worst finding is a
  **committed dev signing seed** (`DEV_SEED`) that `load_seed()` falls back
  to with no environment guard — a prod deploy that forgets
  `TOKEN_PRIVATE_KEY_SEED` signs PASETOs anyone can forge (`attrs:{access:
  [admin]}`). Plus: an unauthenticated `/api/auth/audit/recent` leaking
  registered emails, the magic-link token logged unconditionally, a
  non-atomic (racy) single-use magic-link consume, timing-based account
  enumeration, rate-limit email-canonicalization bombing, incomplete GDPR
  erasure (email survives in `auth_events`), and stale-privilege token
  minting (session `attrs` snapshot never re-read on admin revoke).
- **F-authz. Verifier & ABAC edges.** `from_paseto_keys_url` accepts
  `http://` with no timeout/size cap (MITM key injection); a **vacuous-
  negation** escalation where a `!`-negated `resource.`/`env.` condition
  matches on the coarse (no-record) guard path because the namespace is
  absent there; malformed-key-entry load abort. These process attacker-
  controlled tokens, so they need a forgery + fuzz + policy-property suite.
- **F-guard. Read-path masking & guard consistency.** Masking/record-level
  authz is enforced on single-record GET but **bypassed** on `list` /
  `search` / `check_duplicates` / FHIR / bulk paths (case, person). Two
  bulk-links endpoints (**case `subject_of`**, person `same_identity`) dump
  every high-sensitivity edge with only the coarse gate. `LIKE`-wildcard
  injection in the three repo-based searches (person/worker/event lack the
  `escape_like` the loco services have). Prefix-gated guards
  (event/thing/course) vs the safer deny-unless-public shape. All authz is
  inert while `<ENTITY>_REQUIRE_AUTH` defaults off — so the shipped default
  is wide open; activation must be a tracked release gate.
- **F-data. Bulk / linking / concurrency integrity.** A **critical
  reconcile bug** (link-graph diffs the *global* read-model against *one*
  entity's edges → each entity pass deletes the others' edges; the graph
  never converges). Bulk import with no byte/row caps (OOM DoS), a
  SELECT-then-INSERT upsert race defeating idempotency, artifact IDOR + no
  TTL + unconfined `file://` paths, merge TOCTOU (unlocked read before the
  write tx) and a person self-merge that tombstones the record, and a relay
  that double-ships (no `FOR UPDATE SKIP LOCKED`, no consumer `event_id`
  dedupe). Mostly correctness-and-integrity work with a security blast
  radius.
- **F-input. Unverified input, false matches & fuzzing.** No length or
  array-cardinality caps anywhere → unbounded O(n·m) Jaro-Winkler /
  Levenshtein / Jaccard DoS (five loco services set no body limit at all).
  A systemic **false-deterministic-match** class: short-circuits that key
  on a post-normalization string with no empty guard let two records
  sharing only blank/punctuation values score a spurious `1.0` identity
  (passport, place name+postcode, thing URL/sameAs, national-ID sentinels,
  …). One real `i64` overflow **panic** in portfolio date math. These pure
  functions are ideal fuzz/property targets (never-panic, score ∈ [0,1],
  symmetric, identical⇒1.0, no-spurious-identity).
- **F-assurance. Supply-chain & test infrastructure.** `cargo audit` /
  `cargo deny` / CodeQL run in only **3 of 12** services and there is no
  repo `deny.toml`; `proptest` covers only the 5 older matchers and there
  is **no `cargo-fuzz`** anywhere; three crate roots miss
  `#![forbid(unsafe_code)]`. Roll dependency-scanning family-wide, add a
  fuzz harness, and write `agents/share/security.md` (audit summary +
  invariants + secret-handling rules + the activation gate).

**Priority:** the four critical/near-critical items — F-authn `DEV_SEED`
guard, F-data reconcile scoping, and the F-guard governed-edge leak — lead,
ahead of the rest of the program where a deployment is (or is about to be)
exposed to untrusted callers. Every SEC-\* code fix is a three-part change
(crate spec §13 + code + security test) with the crate CHANGELOG, same as
the rest of the program; the audit's "recommended tests" become the test
half of each task, satisfying the "improve tests (fuzzing, races, unverified
inputs)" mandate directly.

## 3. Sequencing

**Phase 1 — Truth & hygiene (do first, cheap, unblocks everything):**
C-truth, C-rel (toolchain pin + release cut), B-auth's CI
`--include-ignored` rollout. Rationale: stale docs and skipped test
suites actively mislead the sessions executing the rest.

**Phase 2 — Capability completion in the four newest services:**
A-search (Tantivy), A-privacy, B-auth remainder, B-page. These are
copy-adapt jobs with existing in-repo references; they make the
capability matrix honest by making it true.

**Phase 3 — Platform:** A-bus (Fluvio reference + consumer), A-link
(envelope data family-wide, worker same_identity, affiliations), A-bulk
(CSV/review-routing, Parquet, S3, org+case rollout).

**Phase 4 — Surfaces & enablement:** B-fe, C-deploy, C-ops, then D and E
(tutorials/examples last, so they document the improved system and their
commands actually run).

Docs tasks inside each phase may interleave with code tasks; a tutorial
must not be written against behaviour that Phase 2/3 is about to change.

**Phase 5 — Security hardening (Theme F) interleaves, criticals first.**
The audit-driven SEC-\* tasks are not a strict fifth wall after Phase 4:
the four critical/near-critical fixes (SEC-A1 `DEV_SEED` prod guard, SEC-B1
reconcile scoping, SEC-G1 governed-edge leak, SEC-B5 merge TOCTOU/self-
merge) should land **before** any enforced/exposed deployment and ahead of
new-capability work that would build on the affected surfaces. The
supply-chain infra (SEC-I1 dependency scanning, SEC-I3 `forbid(unsafe)`) is
cheap and should land in Phase 1 alongside the other hygiene tasks. The
remaining SEC items slot next to the capability work they touch (F-input
with A-search/A-privacy; F-data with A-bulk/A-link; F-guard with B-auth).

## 4. Working agreements (for the executing session)

- **Green gate per task** (non-negotiable): `cargo build` (0 warnings),
  `cargo test --lib`, `cargo clippy --all-targets --all-features` = 0,
  migration-crate clippy = 0 where present, `cargo test --no-run`
  (DB-gated `#[ignore]` suites must compile), `cargo fmt --check` clean
  on touched crates. Front-ends: `pnpm check` + `pnpm test` + `pnpm build`.
- **Workflow**: branch from `main` per task (or small task group) →
  verify → commit (`Co-Authored-By` trailer per repo convention) → push
  (origin mirrors to Codeberg + GitHub) → `--no-ff` merge to `main` →
  delete the branch. Never commit directly to `main`.
- **Three-part changes**: behavioural change = spec §13 edit + code +
  tests, in the same commit/branch; plus the crate CHANGELOG.
- **Reference-service pattern**: prove a cross-cutting change in one
  service (usually case or person), document the pattern in the shared
  doc, then roll — verifying each service independently (don't trust a
  bulk edit without per-crate gates).
- **Subagents**: large per-crate builds parallelise well (one agent per
  crate), but give each the reference implementation paths and the green
  gate verbatim, and independently re-verify before committing.
- **DB-gated tests** run in CI's Postgres job; locally they compile only
  (no Postgres assumed). Don't mark a task done on compile-only when the
  task is test-behaviour — say so explicitly in the commit.
- **Update this program**: tick tasks in `tasks.md` as they merge; if a
  task is re-scoped, edit its entry rather than silently diverging.

## 5. Risks & notes

- **Fluvio (A-bus)** needs a running broker even for tests — budget for a
  compose-based test tier and expect CI changes; keep it feature-gated so
  default builds never require it.
- **Tantivy in loco services (A-search)**: the older services' Tantivy is
  pre-loco code; adapting the index lifecycle to loco's boot/worker model
  is the real work, not the query code.
- **Pagination (B-page)** touches response contracts front-ends rely on —
  change service + front-end in one task to avoid a broken window.
- **Envelope `data` rollout (A-link)** must stay additive
  (`skip_serializing_if`) — CRUD event wire shapes are frozen; there are
  pinned byte-identical tests that will catch violations.
- The five older services carry **dormant `fluvio` Cargo deps** from the
  pre-loco era; A-bus should reconcile these (use or remove) rather than
  leaving two generations of dependency.
- `same_identity` symmetric writes (person and worker both asserting) are
  by design (aggregator canonicalises + dedupes) — don't "fix" the
  duplication at the write side.
- **Security (Theme F)**: most authz/masking controls are gated behind a
  default-**off** `<ENTITY>_REQUIRE_AUTH` flag, so the audit's findings are
  reachable with no token at all in the default config — treat "enforcement
  off" as the threat model, not an excuse. Several F-data items (reconcile
  scoping, merge TOCTOU/self-merge, upsert race) are **correctness** bugs
  with a security blast radius; fix them as bugs even independent of the
  auth posture. The `DEV_SEED` fallback (SEC-A1) is the one finding that is
  catastrophic *regardless* of any flag — do it first.
