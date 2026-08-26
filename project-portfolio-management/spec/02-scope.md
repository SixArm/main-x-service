## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The DTO contract: the API request/response body for the **thin
  matchable record** **is**
  `project_portfolio_management_matcher::Plan`, stored verbatim as JSONB
  (§5). There is no separate service model and no adapter (exactly the
  care-pathway posture).
- **The optional `kind` label.** Portfolio, Project, Product, Program,
  Practice, Process, Purpose, Pathway, Proposal are the values of an
  **optional** `PlanKind` label — one recursive `plans` table, one REST
  collection (`/api/plans`) — used for description / display / grouping,
  **not** as a discriminator and **not** a match gate: matching is
  **kind-agnostic** (§5.5 / §6.3). Any plan may **contain** any other
  plan via `parent_ref` (a recursive tree); a `parent_ref` that points a
  plan at itself or at one of its descendants is rejected `422`.
- **The matchable/operational partition** (§5.6): the thin `Plan`
  matcher type vs the high-volume sub-resources (tasks, issues) that
  live in their own service tables and never enter the matcher payload.
  `Goal` is the one sub-resource that is **also** part of the payload
  (goal titles feed matching).
- **The full-PM-suite commitment** (§1.4): custom workflows, automation
  rules, time tracking, and sprint ceremonies are capabilities this
  entity **owns**, not features delegated to a source tool. Each carries
  an invariant that is a cross-subproject contract, not an
  implementation detail — every custom workflow state declares one of
  `todo` / `active` / `waiting` / `done` (refused at write time if
  absent, so the derived views cannot silently break); recorded effort
  never becomes a per-person productivity metric and never replaces
  calendar time in a flow ratio.
- **The three ordered vocabularies** (§1.5.1): the lifecycle funnel
  (`idea` … `closed`), the gate stage (`g0` … `g5`), and the sequential
  project phase (`initiating` … `closing`) are three uncoupled axes.
  Their independence is a contract: no cross-vocabulary constraint is
  enforced, and divergence surfaces as a readiness finding rather than a
  refused write.
- **The Flow Framework metric vocabulary** (§1.6): which of the five
  metrics map onto existing time-based-analysis figures and which is
  new, so the same number is never built twice under two names.
- The service ↔ front-end wire contract: raw loco JSON (no envelope),
  TypeScript type mirroring (§5.7).
- Shared invariants that more than one subproject must uphold (§5.8).
- The family integrations this entity adopts wholesale: cross-service
  entity linking
  ([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md))
  and bulk import / export
  ([`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md)).
- Entity-wide goals: government-portfolio scale, multi-locale,
  auditability, technology-compliance posture (§7, §12).

### 2.2 In scope — per subproject

**project-portfolio-management-service-with-loco** owns:

- Plan CRUD with soft delete (`deleted_at`) over the thin matchable
  record on the single `/api/plans` collection.
- The operational sub-resource CRUD: goals, tasks, issues, sprints and
  their ceremonies (§1.4.4), time entries (§1.4.3), and automation rules
  — each a child resource of a plan, keyed by `parent_pid`, in its own
  Postgres table (§10).
- The plan's **project phase** and its transition log (§1.5): the
  `phase` field, one-step-at-a-time advancement, explicit recorded
  backward moves, and the per-phase durations the log makes measurable.
- **Workflow configuration** (§1.4.1): the declared task / issue states,
  their permitted transitions, and the mandatory category mapping that
  keeps the board, burndown, and every time-based-analysis figure
  computable over a custom vocabulary.
- Derived views: timeline / Gantt (goals-with-`target_date` milestones +
  task date ranges), burndown (remaining-vs-estimate over time from task
  snapshots), and the **Flow Framework metrics** (§1.6) — Flow Time,
  Velocity, Efficiency and Load from the existing time-based analysis,
  plus **Flow Distribution**, the feature / defect / risk / debt mix —
  §6.4.
- `POST …/match` (rank an explicit candidate set, no persistence) and
  `POST …/check-duplicates` (match a query against stored plans),
  real-time duplicate detection on create (`409`), record merge (any two
  plans), ILIKE name search — all kind-agnostic across the one
  collection.
- Audit log + event streaming, PASETO v4 public token verification,
  cross-service links, bulk import / export, OpenAPI / Swagger.
- One `plans` table: `pid` + denormalised `name` + the full `Plan` JSONB
  `data` + a nullable denormalised `parent_pid` column; plus the
  sub-resource and family-baseline tables (§10).
- loco.rs app structure, migrations, configuration.

**project-portfolio-management-matcher-rust-crate** owns:

- Pure-library pairwise comparison: kind-agnostic deterministic
  short-circuits (shared deterministic-scheme identifier, same owner-org
  + code, `same_as` URL overlap) + weighted probabilistic scoring (name,
  goals, code, owner org, parent plan, timeframe, keywords,
  relationships, tags) with per-component breakdown.
- The `Plan` domain type, its optional `PlanKind` label, and the
  supporting enums — the entity's canonical DTO — **including the `Goal`
  shape** (goals are part of the matchable payload; their titles are a
  match signal).
- Normalisation (`fold`, code, set folding), Soundex bonus,
  date-proximity scoring, config presets (`strict` / `default` /
  `lenient`).

**project-portfolio-management-front-end-with-svelte** owns:

- Operator routes for the one plans collection — list (`/plans`), create
  (`/plans/new`), detail + delete + check-duplicates (`/plans/[pid]`),
  edit (`/plans/[pid]/edit`), and the sub-resource workspaces (goals /
  tasks / issues, plus timeline + burndown views) — route detail in
  §9.3.
- A plan detail that rolls up its child plans (via `parent_ref`).
- Its own copy of API types, client, and form primitives (drift between
  front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today) — deferrals

> **Corrected 2026-08-25.** This section opened with *"The entity is
> **spec-only; no code exists yet**"* and deferred Tantivy search, the
> durable event bus, and privacy masking — all three of which §14.1
> lists as delivered and backed by named modules. It also deferred the
> deep PM features that §1.4 now commits to. The list below is
> regenerated against §14, which is the authoritative status; the rule
> in §14.3 applies here too — a deferral that outlives its delivery is
> the same failure mode as a gap that outlives its fix.

All three subprojects are **implemented and green** (§14). What remains
out of scope, tracked in §13 / §15 and the crate specs' §13:

**Committed but not yet built** — in scope by §1.4–§1.6, no code yet:

- **Custom workflows** (§1.4.1). Task and issue statuses are still
  compile-time constants (`src/engineering.rs`); nothing declares a
  state vocabulary or its category mapping.
- **Time tracking** (§1.4.3). Nothing exists — `estimate` / `remaining`
  on a task are forecasts, not recorded effort.
- **Sprint planning / daily / review ceremonies** (§1.4.4). The
  `sprints` + `sprint_notes` tables and the categorised retrospective
  notes are delivered; the other three ceremonies are not.
- **Automation breadth** (§1.4.2). The engine is delivered (FR-16c);
  additional triggers — a field change, a phase transition, a date
  arriving, an SLE breach — and multi-action rules are not.
- **Project phases** (§1.5). No `phase` field, no transition log, no
  advancement rules.
- **Flow Distribution** (§1.6). The other four Flow Framework metrics
  are delivered under time-based-analysis vocabulary; the feature /
  defect / risk / debt mix is not computed.

**Deferred, unchanged:**

- **Posts, comments, and membership** sub-resources — collaboration
  threads and membership management remain a roadmap item (§15).
  Sprint notes and review verdicts are structured records against a
  ceremony or a decision, not a general discussion thread.
- Cross-service link **write-side** (`entity_links` + `linked` /
  `unlinked` events) — T-7 — and the link **aggregator**
  (`link-graph-service`), which is out of this trio's scope entirely
  (§15).
- Bulk import / export (`bulk_jobs`, the five endpoints, the codecs) —
  T-8.
- A duplicate **review queue** table — `check-duplicates` returns
  candidates, but no pending / confirmed / rejected decision is
  persisted (T-4 follow-up).
- A `blocked` reason vocabulary, so a constraint finding can name what
  blocked an item rather than stopping at its duration
  ([time-based-analysis.md §17](time-based-analysis.md)).
- gRPC, Parquet import (export-only first), terminology / IANA registry
  existence checks.
- **No FHIR surface** — deliberate, not a gap: no FHIR resource models a
  plan ([`fhir.md`](../../agents/share/fhir.md) §3 puts portfolio out of
  scope).
