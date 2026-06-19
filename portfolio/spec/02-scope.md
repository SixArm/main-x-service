## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The DTO contract: the API request/response body for the **thin
  matchable record** **is** `portfolio_matcher::WorkItem`, stored
  verbatim as JSONB (§5). There is no separate service model and no
  adapter (exactly the care-pathway posture).
- **The four matchable kinds.** Portfolio, Project, Product, Program
  are distinct record types — one `WorkItemKind` discriminator, four
  service tables, four REST collections — and matching is **within a
  kind only** (the matcher's kind gate, R-GATE, §5.5 / §6.3). Child
  kinds (Project / Product / Program) carry a `portfolio_ref` to their
  parent portfolio; Portfolio is the umbrella and carries none.
- **The matchable/operational partition** (§5.6): the thin `WorkItem`
  matcher type vs the high-volume sub-resources (tasks, issues) that
  live in their own service tables and never enter the matcher payload.
  `Goal` is the one sub-resource that is **also** part of the payload
  (goal titles feed matching).
- The service ↔ front-end wire contract: raw loco JSON (no
  envelope), TypeScript type mirroring (§5.7).
- Shared invariants that more than one subproject must uphold (§5.8).
- The family integrations this entity adopts wholesale:
  cross-service entity linking
  ([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md))
  and bulk import / export
  ([`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md)).
- Entity-wide goals: government-portfolio scale, multi-locale,
  auditability, technology-compliance posture (§7, §12).

### 2.2 In scope — per subproject

**portfolio-service-with-loco** owns:

- WorkItem CRUD with soft delete (`deleted_at`) over the thin matchable
  record, across the **four collections** (`portfolios`, `projects`,
  `products`, `programs`), each with the identical controller shape.
- The operational sub-resource CRUD: goals, tasks, issues — each a
  child resource of a work item, keyed by `(parent_kind, parent_pid)`,
  in its own Postgres table (§10).
- Derived views: timeline / Gantt (goals-with-`target_date`
  milestones + task date ranges) and burndown (remaining-vs-estimate
  over time from task snapshots) — §6.4.
- `POST …/match` (rank an explicit candidate set, no persistence) and
  `POST …/check-duplicates` (match a query against stored work items of
  the same kind), real-time duplicate detection on create (`409`),
  record merge, ILIKE name search — all **within a single collection**
  (the kind gate enforces it; you never match a project against a
  product).
- Audit log + event streaming, PASETO v4 public token verification,
  cross-service links, bulk import / export, OpenAPI / Swagger.
- One table per kind (`portfolios`, `projects`, `products`,
  `programs`): `pid` + denormalised `name` + the full `WorkItem` JSONB
  `data` (+ a denormalised `portfolio_pid` column on the child kinds);
  plus the sub-resource and family-baseline tables (§10).
- loco.rs app structure, migrations, configuration.

**portfolio-matcher-rust-crate** owns:

- Pure-library pairwise comparison: a hard **kind gate** (different
  `kind` → no match), then deterministic short-circuits (shared
  deterministic-scheme identifier, same owner-org + code, `same_as`
  URL overlap) + weighted probabilistic scoring (name, goals, code,
  owner org, parent portfolio, timeframe, keywords, relationships,
  tags) with per-component breakdown.
- The `WorkItem` domain type, its `WorkItemKind` discriminator, and the
  supporting enums — the entity's canonical DTO — **including the
  `Goal` shape** (goals are part of the matchable payload; their titles
  are a match signal).
- Normalisation (`fold`, code, set folding), Soundex bonus,
  date-proximity scoring, config presets (`strict` / `default` /
  `lenient`).

**portfolio-front-end-with-svelte** owns:

- Operator routes for the four collections — list (`/portfolios`,
  `/projects`, `/products`, `/programs`), create (`…/new`), detail +
  delete + check-duplicates (`…/[pid]`), edit (`…/[pid]/edit`), and the
  sub-resource workspaces (goals / tasks / issues, plus timeline +
  burndown views) — route detail in §9.3.
- A portfolio detail that rolls up its child projects / products /
  programs.
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today) — MVP deferrals

The entity is **spec-only; no code exists yet** (§14). The first
buildable slice is CRUD + matching over the thin record across the four
collections plus the core sub-resources; explicitly deferred, tracked
in §13 / §15 and the crate specs' §13:

- Deep PM features mirrored from source tools (custom workflows,
  automation, sprint ceremonies, time tracking).
- **Posts, comments, and membership** sub-resources (the plan-family
  lineage had them; this entity ships goals / tasks / issues only —
  collaboration threads and membership management are a roadmap item,
  §15).
- Tantivy full-text search over the JSONB payload + the front-end
  search box (ILIKE name search ships first).
- Durable event bus (the MVP ships an in-memory stream, same shape as
  the sibling services; the durable bus is the family roadmap item —
  [`agents/share/event-bus.md`](../../agents/share/event-bus.md)).
- The cross-service link **aggregator** (`link-graph-service`); the
  portfolio service ships the **write-side** `entity_links` + `linked`
  / `unlinked` events only (§9.5).
- Privacy masking / GDPR export beyond the audit posture in §12
  (lead / assignee / person references are personal data; full masking
  is re-assessed there).
- gRPC, Parquet import (export-only first), terminology / IANA
  registry existence checks.
