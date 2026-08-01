# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]
### Added — pagination on the plan collection reads (2026-08-01)

- **`GET /api/plans` and `GET /api/plans/search` take `?limit=` and
  `?offset=`**, reporting `X-Total-Count` / `X-Limit` / `X-Offset`
  (`agents/share/restful.md`). Bodies stay bare arrays. Defaults
  reproduce the old caps (100 / 50), `limit` clamps to 500, and an
  `offset` past 10 000 is a `400`.
- **The `?parent=` scope reaches the count as well as the page**, so a
  child listing's total describes that parent's children rather than
  every plan. `list_paged` and `count_for` build the same predicate for
  exactly this reason — a total counting a different set from the page
  would be worse than no total.

**Still capped, not yet paged:** the operational sub-resource lists
(`automations`, automation runs, the deadline queue, delegations,
approvals) keep their `LIST_CAP` of 200. They are per-plan working
lists rather than the entity collection, and pagination there wants the
front-end screens that consume them; recorded in tasks.md PG-1 rather
than half-done here.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-2, the loco-style half of the rollout (case was the reference; the
five axum-style services landed the same day as AU-1).

- **The verifier and the ABAC policy are now reloadable holders**
  (`ReloadableVerifier` / `ReloadablePolicy`) that the blanket guard
  **and** the bearer extractors read per request. They were boot-only
  `OnceLock` snapshots, so a rotated key set or an edited policy could
  not have reached a running process at all.
- **`spawn_key_refresh`** re-fetches `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` every
  `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`spawn_policy_watcher`** polls `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE`'s mtime every
  15 s and calls `reload_policy()`; a malformed edit falls back to the
  built-in default rather than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, new here and in its
  own binary: `401` without a token, `403` for a valid token the default
  policy denies a write to, `200` for `access=write`.
- New environment variable: `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_REFRESH_SECS`.

### Fixed — the DB-gated suite ran for the first time (2026-08-01)

Both failures were in the tests; the service behaved correctly in each
case.

- **The workflow-automation test indexed an object as an array.**
  `GET /api/plans/{pid}/tasks` answers `{ "tasks": [...], "counts": {...} }`,
  so `moved[0]["assignee_ref"]` was `Null` and the assertion read as "the
  automation never fired". It had: the rule logged an `applied` run and
  the row carried the assignee. Now reads `moved["tasks"][0]`.
- **The burndown test hard-coded a sprint window that drifted into the
  past.** Burndown counts `done_at` stamps falling on or before each day
  in the window; the test completes a task *now*, and once "now" passed
  the fixed `ends_on` (2026-07-26) the completion stopped counting — the
  final point read 2 remaining instead of 1. The window is now relative
  to today (−6 / +7 days), keeping today inside a 14-day sprint.

  Suite: 36/36 green vs Postgres 18; crate enrolled in
  [`ci/db-suites.txt`](../../ci/db-suites.txt).


### Added

- 2026-07-22 — **Collaboration, automation, and prioritisation
  capabilities** (spec §9.4a). Migration
  `m20260722_000001_capabilities` adds `reviews`, `automations`,
  `automation_runs`, `scheduled_actions`, and `notifications`.
  - **Collaborative review** — `POST`/`GET /api/reviews`,
    `/{pid}/respond`, `/{pid}/submit`, `DELETE /{pid}`, and
    `/api/reviews/consensus`. Reviewers are `EntityRef` URNs, never raw
    emails; `reviewer_scope` records the internal/external disclosure
    decision explicitly. Only a reviewer who **accepted** may submit, so
    an unanswered invitation never becomes evidence; consensus requires
    a **strict** majority (a tie or plurality reports none) and always
    reports what is still outstanding.
  - **Assignees** — `POST /api/plans/{pid}/tasks/{t_pid}/assign`
    (`null` unassigns, notifies the assignee) and
    `GET /api/assignees/workload`, which surfaces the `unassigned` pile
    rather than dropping it.
  - **Workflow automation** — `POST`/`GET /api/automations`,
    enable/disable, delete, and `GET /api/automations/runs`. Rules fire
    from a Kanban move and from a submitted plan review. Action shapes
    are validated at write time; a failing rule is logged as a `failed`
    run and never undoes the operator's move; actions are applied
    without re-entering the engine, so automations cannot cascade.
  - **Set and forget** — `POST`/`GET /api/scheduled-actions`,
    `/sweep`, and cancel. The sweep **claims** each due row with a
    conditional update, so a deadline fires exactly once even if the
    optional ticker (`PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES`,
    default off) races a manual sweep. Sweeps are capped and say so.
  - **Data-driven prioritisation** — `GET /api/plans/{pid}/smart-score`
    and `/api/prioritisation`. The Smart Score is a renormalised
    weighted average over seven components with a full per-component
    breakdown; absent evidence is **dropped and disclosed** (`missing`
    + `coverage`), not scored zero, and a plan with no evidence scores
    `null` / `unscored` and sorts last. ROI is computed only within a
    single currency (no FX conversion, ever). Weights are tunable via
    `PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS` (a complete
    10 000-basis-point map; anything else is warned about and ignored).
  - **Bird's-eye visibility** — `GET /api/lifecycle` and
    `/api/plans/{pid}/lifecycle`. Every phase is reported even at zero,
    items in an unresolvable phase are counted separately, and
    readiness is a five-check checklist that names each blocker.
  - Every mutation writes an `audit_logs` row; all four rule modules
    (`collaboration`, `automation`, `prioritisation`, `lifecycle`) are
    pure and DB-free with 56 unit tests, plus six `#[ignore]`d request
    tests and OpenAPI coverage.
  - **Not built:** email / push transport (notifications are in-app
    only), a `votes` Smart Score component (a plan carries no link back
    to its originating idea), and record-level ABAC on the new
    endpoints — they sit behind the blanket guard only.

- 2026-07-22 — **Five new `kind` labels: `Practice`, `Process`,
  `Purpose`, `Pathway`, `Proposal`.** `parse_kind_label` accepts the
  singular and plural spellings of each (`practice`/`practices`, …), the
  OpenAPI `Plan.kind` enum lists them, and the `kind_target` /
  `collection` validation messages on `/api/proposals`,
  `/api/ideas/{pid}/convert`, and `/api/reports` name them. Matching is
  unchanged — still kind-agnostic. (The `Proposal` **label** on a plan is
  unrelated to the `proposals` intake pipeline resource.)

### Changed — BREAKING: work items unified into one recursive `plan` collection (2026-07-20)

- **`WorkItem` → `Plan` rename.** The registered entity is now a **plan**.
  The embedded matcher exposes `Plan`, `PlanKind`, `PlanIdentifier`,
  `PlanRelationship`, `PlanStatus`, and `MatchingEngine::match_plans`
  (was `match_work_items`); `Plan::new(name)` leaves `kind` `None`. The
  service DTO, controller (`controllers/plans.rs`), and model
  (`models/plans.rs`) follow.
- **Four collections → one `/api/plans` collection.** The four REST
  collections (`/api/portfolios`, `/api/projects`, `/api/products`,
  `/api/programs`) are replaced by a single `/api/plans` collection; the
  `{collection}` path segment is gone. Plan-scoped sub-resources moved to
  `/api/plans/{pid}/...` (tasks, sprints, milestones, allocations,
  gate-reviews, risks, budget-lines, objectives, benefits, schedule,
  governance, audit).
- **`kind` is now an optional label — the kind gate is removed.** `kind`
  is `Option<PlanKind>`, an optional Portfolio / Project / Product /
  Program descriptive/grouping label; it no longer fixes a collection and
  no longer gates matching. The matcher's hard kind gate ("R-GATE") is
  gone — **any two plans may match** regardless of kind
  (`MatchBreakdown.kind_gate_blocked` remains only as a vestigial,
  always-false field).
- **General containment via `parent_ref` + cycle check.** `portfolio_ref`
  is renamed `parent_ref`: any plan may contain any other plan (a
  recursive tree). A `parent_ref` that points a plan at itself or at one
  of its descendants is now rejected `422` (new containment-cycle check).
- **Schema.** One `plans` table (was `work_items`) with a **nullable**
  `kind` column and a `parent_pid` column (was `portfolio_pid`);
  migration `..._000001_plans`.
- **Merge is unscoped.** Merge is no longer collection/kind-scoped — any
  two plans may merge; it returns `422` only on a self-merge (equal pids)
  and `404` on an unknown pid (the former cross-kind rejection is gone).
- **Proposals.** The proposal `kind_target` is now an optional descriptive
  label (blank = none), validated via the `parse_kind_label` helper.

### Added — engineering moderate fits (2026-07-20)

- Story points on tasks + `GET .../velocity` (team-local; real
  completions only); env-configured WIP limits enforced on board moves
  (`PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS`); sprint retro/feedback
  notes with once-only action/feedback → task conversion; DevOps event
  ingest (`/api/devops/events`) + DORA-style metrics derived only from
  ingested events (MTTR over linked pairs; declared-cause change
  failure) + the deploy-event release register.

### Added — engineering-team features (2026-07-20)

- The spec-§13 **tasks** sub-resource (Kanban statuses, PATCH board
  move with true flow stamps — `status_changed_at` per move, first
  `done_at` kept; PUT refuses status changes), **sprints**, and the
  honest **burndown** (real completions only, derivation served).
- The last-24h **standup digest** (audit-derived) and the estate
  views: blocked-work aging, the `moscow:<band>` scope cut, the
  delivery-links panel (external tracker identifiers), and the
  milestone calendar (`milestones.kind`:
  milestone/demo/release/checkpoint).
- Migration `m20260720_000001_engineering`; tasks/sprints never feed
  the matcher (the partition rule).

### Added — oversight areas: board / auditor / compliance / CRO / CISO / regulator (2026-07-20)

- Thirteen derived-view endpoints (`controllers/oversight.rs`) + the
  `insight_snapshots` table: the period board pack + investments +
  stored trend snapshots (explicit POST or env-gated ticker), the
  audit-trail explorer + segregation-of-duties findings + evidence
  pack (JSON/CSV), compliance/security risk registers + conformance
  findings, the CRO heatmap (posture, concentration, hygiene, declared
  risk appetite or an honest absence), and the deliberately coarse
  regulator extract honouring the ABAC `mask` obligation.

### Added — executive moderate fits (2026-07-19)

- **Stage-gated funding tranches**: `budget_lines.gate` + `released_at`
  (migration `m20260719_000002`); a gated line is held (actuals `422`)
  until the work item's stage reaches the gate and the new
  `POST …/budget-lines/{line_pid}/release` succeeds (fail-closed
  `gate_reached`; audited). `financials/exposure` reports per-currency
  `held_minor`.
- **Technical-debt register**: `risks.category` (validated closed set)
  + `GET /api/technology/debt` — `tech_debt` risks, exposure-sorted.
- **Delivery-flow metrics**: `milestones.done_at` stamped on complete +
  `GET /api/technology/flow` — throughput/month + median lead days;
  pre-stamp completions counted but never timed.
- **Strategic-alignment coverage**: `GET /api/executive/alignment` —
  aligned/unaligned per collection, unaligned spend per currency,
  ranked unaligned items (largest single-currency planned; disclosed
  heuristic).
- **Scenario comparison**: `GET /api/scenarios/compare?a=&b=` — two
  live evaluations side-by-side with per-currency deltas (b−a).

### Added — executive insight areas: CEO / CFO / CTO (2026-07-19)

- Seven read-only derived views over existing tables (no new
  migrations), ETag-conditional with `as_of`:
  `/api/executive/health` (per-portfolio RAG briefing),
  `/api/executive/decisions` (gate reviews, scenario commits, decided
  proposals, merges), `/api/executive/benefits` (per-currency target vs
  realized; honest null ratios), `/api/financials/variance` (by
  collection / category / portfolio; minor units; currencies never
  merged), `/api/financials/exposure` (per-currency totals, no FX),
  `/api/technology/dependency-risk` (fan-out / cross-portfolio /
  red-predecessor edges), `/api/technology/radar`
  (`tech:<name>[:<ring>]` tag convention, majority ring vote).
- Pure derivations live in `src/insights.rs` with DB-free unit tests;
  the RAG derivation is shared with `/at-a-glance`.

### Added

- 2026-07-18 — **PPM Phase C: strategy** (T-PPM-C; PPM-2/4/5/11).
  The idea funnel (capture / vote / dismiss / convert into a draft
  proposal, `provenance=idea` — completing idea → proposal → work
  item); what-if scenarios evaluated over live budgets, open risk
  exposure, and OKR alignment (per-currency saturating sums, budget
  cap + must-include violations; **infeasible commits refused**, the
  committed evaluation audited); the OKR objective registry with
  weighted (1–5) per-pair-upserting item mappings and
  per-collection alignment rollups; benefits with minor-unit
  financial targets or non-financial notes, accumulate-realize, and
  per-currency **ROI in basis points** against recorded budget
  actuals. Pure rules in `src/strategy.rs`; 3 unit + 4 DB-gated
  request tests vs Postgres 18.

- 2026-07-18 — **PPM Phase B: visibility** (T-PPM-B; PPM-6/7/8/9).
  Cross-item finish-start dependencies (cycle-refusing) + the
  portfolio schedule view (violations, memoised critical path,
  undated members); milestones with overdue flags; resource
  allocations over `person:`/`worker:` URNs + the per-person
  capacity rollup (summed percent over a window, > 100 % flagged);
  saved report definitions run synchronously as JSON or CSV
  (RFC-4180 escaping, row cap 1000); the ETag-conditional
  `/api/at-a-glance` dashboard (per-collection RAG — documented
  heuristic over materialised risks / overdue targets / budget
  overrun / exposure / schedule violations — stage distributions,
  and site tiles). Pure rules in `src/visibility.rs`; 7 unit + 5
  DB-gated request tests vs Postgres 18.

- 2026-07-18 — **PPM Phase A: the governance core** (T-PPM-A;
  PPM-1/3/10/12 from the entity roadmap). Work-intake `proposals`
  pipeline with matcher-backed duplicate-demand detection and
  promote-to-work-item (`provenance=intake`); strictly ordered
  phase-gate reviews (g0_concept…g5_benefits) advancing an
  operational `work_items.stage`, gate-lockable via the new
  `resource.stage` record-level ABAC (`auth::authorize_record`);
  risks (1–5 × 1–5 exposure, escalation); budget lines in integer
  minor units + ISO-4217 with per-currency planned/actual/variance;
  the per-item `/governance` summary. Pure rules in
  `src/governance.rs`; every mutation audited; OpenAPI `governance`
  tag; 4 unit + 5 DB-gated request tests, verified against
  Postgres 18.


### Fixed

- 2026-07-18 — **Unknown-pid reads returned 500, not 404.** loco 0.16's
  `IntoResponse` catch-all maps an unmapped `ModelError::EntityNotFound`
  to a 500, so `GET /…/{pid}` with an unknown pid crashed instead of
  404ing (the organization service was immune — its `http_err` helper
  already mapped it; the copy-adaptors dropped it). Controller lookups
  now route through a `model_not_found` mapping. Family-wide fix with
  per-crate request-test pins.


### Changed

- 2026-07-18 — **Subproject renamed**: `portfolio` →
  `project-portfolio-management` (directory, crate/package name, lib
  ident, env-var prefix `PORTFOLIO_*` → `PROJECT_PORTFOLIO_MANAGEMENT_*`,
  database names). The **domain language is unchanged**: the work-item
  kinds (portfolio / project / product / program), the `work_items`
  table, the API routes, and the matcher's `WorkItem` type keep their
  names — the rename repositions the *subproject* as a project
  portfolio management (PPM) product; see the feature roadmap in
  `../spec/15-roadmap.md`.


### Fixed

- 2026-07-18 — **Fresh-database `db migrate` failure.** The
  `…_000004_event_outbox` migration used the loco `create_table`
  helper, which pluralizes table names (`event_outbox` →
  `event_outboxes`); its own index DDL then failed and rolled back
  the entire fresh migrate (zero tables). Rewritten as explicit SQL
  creating exactly `event_outbox`; verified against a fresh
  Postgres 18 (all migrations apply, correct table names). Family-wide
  fix (case, care-pathway, organization, portfolio; patient-flow
  shipped with the explicit-SQL form).


### Security

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Security — SEC-M1: input-size caps on the validation entrypoint (2026-07-13)

- `src/validation.rs` now rejects oversized `WorkItem` payloads before the
  record is stored or matched, closing a CPU/memory denial-of-service
  vector: the matcher runs `O(n·m)` string similarity (Jaro-Winkler /
  Soundex) and Jaccard over the payload's text fields and arrays, so a
  single huge string or huge array is a DoS (amplified by the
  check-duplicates scan). New named caps enforced (all problems collected,
  never aborting early, surfaced as `422`): `MAX_TEXT_LEN = 1024`
  Unicode scalar values per scalar text field (`name`, `code`,
  `owner_org_id`, `owner_org_name`, `lead_ref`, `portfolio_ref`,
  `start_date`, `target_date`, `in_language`); `MAX_ARRAY_LEN = 256`
  entries per array (`alternate_names`, `goals`, `keywords`, `tags`,
  `identifiers`, `same_as`, `relationships`); `MAX_ITEM_LEN = 512` per
  string entry inside an array (each entry, plus `goals[i].title`,
  `identifiers[i].value`, `relationships[i].work_item_id`). The `kind`
  discriminator is an enum, not free text, so it is not capped. New unit
  tests cover oversized single field, oversized array, oversized array
  item, and a within-caps large-but-valid record.

### Changed — event bus: audit now joins the outbox transaction (2026-07-09)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and the `work_items` controller no longer audits
  separately. New DB-gated `tests/outbox_audit.rs` drives `create_and_emit`
  under `outbox` and asserts entity + event + audit all commit together.
  (The `merge_records` history row stays a best-effort side channel — it
  is merge metadata, not the §3 audit trail.)

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`; matched on path suffix across
  all four collections), and the policy is evaluated over the token's
  new `attrs` claim, first-match-wins, defaulting to allow-read /
  deny-mutation.
- New env vars `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` (inline JSON) and
  `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. `require_auth_mw` in `app.rs`
  passes the request method and `auth::policy()`. DB-free unit tests
  pin the family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added

- **Boot-time paseto-keys-over-HTTP fetch** (the spec §13 follow-up, done
  2026-07-04). New optional env var `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`: when set
  (non-blank), `auth::init` — called from `App::after_routes`, before the
  app serves traffic — fetches the auth-service's published Ed25519 key
  set once over HTTP via `Verifier::from_paseto_keys_url` (the
  `authentication-verifier` crate's `fetch` feature, now enabled). On
  success the fetched key set **wins** over the `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS`
  env key set (`tracing::info!`); on failure the service logs a
  `tracing::warn!` and falls back to the env path, so it **always
  boots**. Unset/blank ⇒ prior behaviour unchanged (env key set, else
  empty reject-all). Fetch is once-at-boot only — no refresh loop
  (rotation-triggered refetch is tracked in spec §16). The seeding is
  idempotent (`OnceLock`), and the fetch-or-fallback helper
  (`auth::fetch_or`) is dependency-injected (URL / issuer / audience /
  fallback passed in) so tests cover it without the process global: a
  `#[tokio::test]` local ephemeral-port HTTP listener proves a token
  signed by the served key verifies via the fetch-built verifier, and a
  fast-failing URL (`http://127.0.0.1:1/`) proves fallback without
  panic. Existing env-key auth tests unchanged and green.

## [0.1.0] - 2026-06-18

### Added

- **Inaugural spec scaffold (spec-only — no code yet).** Documentation
  set for the loco.rs work-item registry **and** project-management tool:
  - `spec/index.md` — the §1–§18 single-source-of-truth service spec,
    mirroring the care-pathway service shape. Defines the **four matchable
    collections** (`portfolios`, `projects`, `products`, `programs`) — one
    JSONB row table per kind, sharing one parameterised controller core
    (the API DTO **is** `project_portfolio_management_matcher::WorkItem`, persisted verbatim,
    matched with no adapter); **within-kind matching only** (the matcher's
    R-GATE makes a project never match a product); the umbrella hierarchy
    (Projects / Products / Programs carry a `portfolio_ref` to their parent
    portfolio); the operational sub-resources (goals, tasks, issues) in
    their own tables keyed by the parent `(kind, pid)` and **excluded from
    the matcher payload** (goal titles bridge via `data.goals[]`); the
    derived timeline / burndown read views; CRUD + soft-delete + audit;
    embedded probabilistic + deterministic matching (`POST /match` /
    `/check-duplicates` / `/deduplicate`); real-time create duplicate
    detection (`409`) + review queue; record merge (`Replaces` link +
    transferred snapshot + `Merged` event, same-kind only); `ILIKE` name
    search; event streaming (durable-bus Phase 1 envelope); OpenAPI/Swagger;
    per-collection Prometheus metrics; offline PASETO v4.public verification
    + blanket `/api/*` enforcement (off by default, gated by
    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`); cross-service entity links (write side); and
    bulk import/export (deferred).
  - `README.md` — user-facing intro, route table, quick start, status.
  - `CLAUDE.md` — one-line `@AGENTS.md` include.
  - `AGENTS.md` — agent guide (what this is, API surface, MVP scope,
    golden rules incl. four-kinds-one-core, within-kind matching, and the
    matcher-partition rule, intended layout).
  - `index.md` — documentation index + worked flow.
- **Auth model is PASETO v4.public + cookie sessions (spec-only).** The
  intended auth design is **server-side cookie sessions** for the human
  session plus **offline PASETO v4.public** verification for peers
  (verified against the auth-service's published **Ed25519 key**), and a
  **BFF** for the front-end so the browser holds no token. The
  `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` flag + enforcement semantics follow the family
  contract. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (RS256/JWKS not used).
- **Adopts the cross-service-linking contract.** Portfolio is a
  participating service with an `entity_links` write-side table and
  `POST`/`GET`/`DELETE /api/{collection}/{pid}/links` emitting `linked`
  / `unlinked`; a work item / goal / task / issue can link to **any** index
  entity. Cross-service links are **not** a matcher signal (separate from
  within-payload `relationships`). Contract:
  [`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md).
- **Adopts the bulk-import/export contract** (deferred §13). Async
  `bg_pg` jobs, JSONL/CSV/Parquet, the five endpoints under
  `/api/{collection}/*`; stable upsert key = a deterministic external PM
  identifier (Jira / Asana / Trello / MS Project / GitHub Project / Linear /
  URI / UUID) or owner-scoped `code` or `pid`; keyless rows → dedupe →
  review queue (within-collection). Lead / person refs are personal data →
  export audited. Contract:
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md).

### Notes

- No Rust / Cargo crate has been generated; every `spec.md §13` task is
  unchecked. Next step is `loco new` (stripped of the auth starter) plus
  the four work-item tables + the shared CRUD MVP.
- The canonical `WorkItem` domain model is owned by the
  [portfolio entity spec §5](../spec/index.md); this crate spec references
  it.
- Copy-adapted from the (deleted) `plan` service template; the headline
  differences are the **four distinct matchable kinds** (vs plan's single
  `plan_type` field), the within-kind match **gate** (R-GATE), and the
  dropped `posts` / `comments` / `members` sub-resources (now deferred
  roadmap).

[Unreleased]: #unreleased
[0.1.0]: #010---2026-06-18
</content>
