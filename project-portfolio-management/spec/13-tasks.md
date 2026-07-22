## 13. Tasks

Live entity-level work queue. (Historical header — the trio has been
**implemented** since 2026-06-19; per-crate work now lives in each
subproject's own spec §13. The unchecked items below are the original
build-out backlog kept for trace; the PPM feature catalogue and its
delivery state live in [15-roadmap.md](15-roadmap.md).) Tasks that belong to one subproject's internals
migrate into that crate's spec §13 once the crate is scaffolded; they
are listed here while the trio is being stood up. Each task has an
acceptance criterion; tick the box when an automated test or clearly
described manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [x] **2026-07-22 — Collaboration, automation, and prioritisation
  capabilities (§6.4a / §9.2a / §10.5).** Service: migration
  `m20260722_000001_capabilities`, four pure rule modules
  (`collaboration` / `automation` / `prioritisation` / `lifecycle`,
  56 unit tests), three controllers, the automation engine firing from
  a board move and a submitted plan review, the claim-based
  set-and-forget sweep + optional ticker, OpenAPI, six `#[ignore]`d
  request tests. Front-end: `CapabilityClient` + the `/prioritisation`,
  `/lifecycle`, `/reviews`, and `/automations` pages (English-first).
  Not built: email / push transport, a `votes` Smart Score component,
  record-level ABAC on the new endpoints, and a notifications page.

- [ ] **T-1 — Scaffold the trio.**
  - [ ] Create `project-portfolio-management-matcher-rust-crate/`,
    `project-portfolio-management-service-with-loco/`, and
    `project-portfolio-management-front-end-with-svelte/` from the care-pathway / plan
    siblings (copy-adapt; drift accepted — repo decision 2026-06-02).
  - [ ] Each subproject ships its own `spec/` (matcher §1–§25; service
    + front-end §1–§18) referencing this entity spec's §5 as the
    canonical domain model rather than redefining it.
  - [ ] Add the entity `AGENTS/` reference set (`index.md`,
    `models.md`, `matching.md`, `restful.md`, `testing.md`,
    `subprojects.md`, `spec-driven-development.md`).
  - [ ] Register the trio in the root `AGENTS.md`, `agents/share/overview.md`,
    and the front-end table.
  - **Acceptance:** every link in this entity spec resolves to a real
    file or section.
- [ ] **T-2 — Matcher crate: domain model + matching.**
  - [ ] The canonical `Plan` type + optional `PlanKind` label +
    `Goal` + all enums (§5.1–§5.4), serde round-trip, NFKC-folding
    diacritic-preserving normalisation.
  - [ ] **Kind-agnostic matching**: `kind` is an optional descriptive
    label that neither gates nor scores; any two plans may match
    regardless of their labels (`MatchBreakdown.kind_gate_blocked` is a
    vestigial always-`false` field).
  - [ ] Deterministic short-circuits: R-0 (each deterministic
    identifier scheme — `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
    `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`;
    owner-scoped `Code`/`LocalId`/`Custom` excluded), R-1 (same
    `owner_org_id` + equal normalised `code`), R-2 (`same_as`
    overlap) → 1.0.
  - [ ] Probabilistic components + weights per §6.8 (name JW + Soundex
    bonus, goal-title Jaccard, code, owner org, parent-plan exact,
    timeframe date-proximity, keywords, relationships typed-set
    Jaccard, tags), renormalised over present components; `status`
    and the optional `kind` label never scored; presets
    strict/default/lenient.
  - **Acceptance:** `cargo test` green; the FR-8 weight table sums to
    1.00 in a test; kind-agnostic matching, each rule, and each
    component have a unit test; public-API + doctest suites pass.
- [ ] **T-3 — Service crate: chassis + thin-record CRUD + matching.**
  - [ ] loco 0.16 / Axum 0.8 / SeaORM 1.1 chassis; the `plans`
    migration (nullable `kind`, `parent_pid`); `cargo loco start`;
    config yamls; port 5150.
  - [ ] The controller over the one plans collection: CRUD over the
    thin `Plan` (JSONB `data`), name search (`ILIKE`), `/match`,
    `/check-duplicates` (all kind-agnostic), validation (§FR-1a incl.
    `parent_ref` shape + containment-cycle rejection).
  - **Acceptance:** DB-free matcher-embedding + JSON round-trip tests
    green; blank-name / malformed-`EntityRef` / malformed-`parent_ref` /
    containment-cycle / malformed-deterministic-id → `422`.
- [ ] **T-4 — Service crate: operational sub-resources + derived views.**
  - [ ] Tables + CRUD for tasks, issues (keyed by `parent_pid`); goals
    via `data.goals[]` mutation (the §5.3 bridge).
  - [ ] Derived `timeline` + `burndown` read endpoints; `task_snapshots`
    feeding burndown.
  - [ ] Real-time `409` duplicate detection on create; record merge
    (any two plans) that **re-homes sub-resources** to the survivor;
    the child roll-up (`?parent=` filter + `parent_pid` column).
  - **Acceptance:** DB-gated request tests cover sub-resource CRUD,
    the goals bridge (a goal write changes a subsequent match score),
    `409` on create, merge re-homing, and the roll-up filter; the
    partition test (no sub-resource field in any `data`; a non-goal
    sub-resource write does not change the match score) passes.
- [ ] **T-5 — Auditability + security.**
  - [ ] `audit_logs` (plan + sub-resource actions) + read endpoints;
    in-memory `PlanEvent` stream + `…/events/recent`.
  - [ ] Offline PASETO v4 public verification (`authentication-verifier`);
    switch `src/auth.rs` per
    [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
    (supersedes the RS256-JWT model). `AuthUser`/`MaybeAuthUser`; `whoami`
    protected; audit / merge `actor` stamped from the token.
  - **Acceptance:** create + update + delete a plan and a task →
    audit rows + events read back; no token → `401`, valid token →
    `2xx`.
  - [x] *Follow-up (delivered):* blanket `/api/*` enforcement +
    paseto-keys-over-HTTP fetch + **ABAC** write authorisation over
    the token's `attrs` claim (per
    [`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md);
    supersedes the earlier role-based sketch). Default-off via
    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`; activation awaits the coordinated family
    SSO rollout (the front-end must attach the bearer token first).
- [ ] **T-6 — Front-end: routes + sub-resource workspaces + tests.**
  - [ ] The `/plans`, `/plans/new`, `/plans/[pid]`,
    `/plans/[pid]/edit` routes over the thin record; sub-resource
    workspaces (`…/[pid]/{goals,tasks,issues}`) and derived views
    (`…/[pid]/{timeline,burndown}`); the child roll-up.
  - [ ] vitest units (`ApiClient` + `PlanRepository`) +
    Playwright smoke; `pnpm run check` strict 0/0 + production build.
  - **Acceptance:** both suites green; a contract-drift in any
    endpoint path fails a test.
- [ ] **T-7 — Cross-service links (write-side).**
  See §9.5 and
  [cross-service-linking.md](../../agents/share/cross-service-linking.md).
  - [ ] `entity_links` migration (`UNIQUE (from_pid, kind, to_ref,
    valid_from)`); `POST`/`GET`/`DELETE …/{pid}/links`; `linked` /
    `unlinked` events; the `EntityRef` value type (copied per project).
  - [ ] The partition rule (§7 there): links are never stored in
    `relationships` and never fed to the matcher.
  - **Acceptance:** a link create emits `linked`, a delete emits
    `unlinked`, and a test asserts no link ever reaches the matcher.
- [ ] **T-8 — Bulk import / export.**
  See §9.6, §10.4 and
  [bulk import/export](../../agents/share/bulk-import-export.md).
  - [ ] `bulk_jobs` migration (shared doc §3 schema, with
    `UNIQUE (entity, kind, idempotency_key)`; `entity` is the one
    `plans` collection).
  - [ ] The five endpoints on the plans collection (§9.6); `bg_pg`
    worker draining `queued → running → completed |
    completed_with_errors | failed`.
  - [ ] JSONL (lossless reference) + CSV (flattening per §9.6: every
    repeated / nested field a JSON-in-cell) codecs; Parquet
    **export-only**, feature-gated.
  - [ ] Per-row pipeline reusing the single-create validators +
    matcher + review queue: upsert by stable key (deterministic
    external-PM identifier, `(owner_org_id, code)`, or `pid`, §9.6);
    keyless / unmatched rows → duplicate detection → review queue with
    `provenance = import`; events + audit not bypassed.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`); one bad row
    never aborts the load; counts reconcile.
  - [ ] Export masking + audit: `masking_profile` (masked default —
    people references hidden; full gated), `include_soft_deleted`
    gated, every export audited (even zero-row).
  - **Acceptance:** integration tests cover idempotent re-import,
    the per-row error report, a keyless dedupe-to-review row
    (`provenance = import`), masked vs full export of a plan with
    people references, and that a zero-row export still writes an
    audit record.
- [ ] **T-9 — OpenAPI / Swagger + richer validation.**
  - [ ] OpenAPI 3 schema (hand-written, dependency-light, same
    approach as the organization / care-pathway services) + Swagger UI
    at `/api-docs/openapi.json` · `/swagger-ui`.
  - [ ] Validation of deterministic identifier shapes (UUID,
    external-PM-id patterns), `EntityRef` syntax, `parent_ref` (a valid
    plan `pid`, no containment cycle), `in_language` (BCP-47), and
    relationship integrity (no self-reference, inverse-consistency,
    acyclicity — §5.8); `422` on failure.
  - **Acceptance:** Swagger UI serves the documented endpoints; a
    malformed-identifier / self-referencing-relationship /
    containment-cycle test returns `422`.
- [x] **T-10 — Unify the four work-item kinds into one recursive
  `Plan`.** ✅ *Delivered 2026-07-20 (built + tested green across
  matcher, service, and front-end).* The former Portfolio / Project /
  Product / Program "work item" kinds were collapsed into **one
  recursive entity**, a **plan**:
  - The matcher type is renamed `WorkItem` → `Plan`
    (`PlanKind`/`PlanIdentifier`/`PlanRelationship`/`PlanStatus`);
    `kind` becomes `Option<PlanKind>` — an **optional descriptive
    label** that no longer gates or scores. `Plan::new(name)` defaults
    `kind` to `None`.
  - The hard **kind gate (R-GATE)** is **removed**: any two plans may
    match regardless of their labels. `MatchBreakdown.kind_gate_blocked`
    is kept as a vestigial, always-`false` field for wire compatibility.
  - The four `/api/{portfolios,projects,products,programs}` REST
    collections collapse into one **`/api/plans`** collection; the four
    per-kind tables collapse into one **`plans`** table (nullable
    `kind`, nullable `parent_pid`).
  - Containment becomes **recursive**: any plan may contain any other
    plan via `parent_ref` (renamed from `portfolio_ref`); a `parent_ref`
    forming a containment cycle is rejected `422`. Sub-resource tables
    are re-keyed by `parent_pid` (not `(parent_kind, parent_pid)`).
    Merge is no longer kind-scoped.
  - **Acceptance:** the matcher matches two plans with differing `kind`
    labels; the single `/api/plans` CRUD + match + merge suite is green;
    the containment-cycle `422` and the child roll-up (`?parent=`) pass;
    `cargo test` / `clippy` / `fmt` clean and the front-end `pnpm run
    check` + build clean.
