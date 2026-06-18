## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The DTO contract: the API request/response body for the **thin
  matchable record** **is** `plan_matcher::Plan`, stored verbatim as
  JSONB (§5). There is no separate service model and no adapter
  (exactly the care-pathway posture).
- **The matchable/operational partition** (§5.6): the thin `Plan`
  matcher type vs the high-volume sub-resources (goals, tasks,
  issues, posts, comments, members) that live in their own service
  tables and never enter the matcher payload. `Goal` is the one
  sub-resource that is **also** part of the payload (goal titles feed
  matching).
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

**plan-service-with-loco** owns:

- Plan CRUD with soft delete (`deleted_at`) over the thin matchable
  record.
- The operational sub-resource CRUD: goals, tasks, issues, posts,
  comments, members — each a child resource of a `Plan`, keyed by the
  plan `pid`, in its own Postgres table (§10).
- Derived views: timeline / Gantt (goals-with-`target_date`
  milestones + task date ranges) and burndown (remaining-vs-estimate
  over time from task snapshots) — §6.4.
- `POST …/match` (rank an explicit candidate set, no persistence) and
  `POST …/check-duplicates` (match a query against stored plans),
  real-time duplicate detection on create (`409`), record merge,
  ILIKE name search.
- Audit log + event streaming, PASETO v4 public token verification,
  cross-service links, bulk import / export, OpenAPI / Swagger.
- One `plans` table: `pid` + denormalised `name` + the full `Plan`
  JSONB `data`; plus the sub-resource and family-baseline tables (§10).
- loco.rs app structure, migrations, configuration.

**plan-matcher-rust-crate** owns:

- Pure-library pairwise comparison: deterministic short-circuits
  (shared deterministic-scheme identifier, same owner-org + plan code,
  `same_as` URL overlap) + weighted probabilistic scoring (name,
  goals, plan code, owner org, plan type, timeframe, keywords,
  relationships, tags) with per-component breakdown.
- The `Plan` domain type and its supporting enums — the entity's
  canonical DTO — **including the `Goal` shape** (goals are part of
  the matchable payload; their titles are a match signal).
- Normalisation (`fold`, plan-code, set folding), Soundex bonus,
  date-proximity scoring, config presets (`strict` / `default` /
  `lenient`).

**plan-front-end-with-svelte** owns:

- Operator routes: list (`/`), create (`/new`), detail + delete +
  check-duplicates (`/[pid]`), edit (`/[pid]/edit`), and the
  sub-resource workspaces (goals / tasks / issues / posts / members,
  plus timeline + burndown views) — route detail in §9.2.
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today) — MVP deferrals

The entity is **spec-only; no code exists yet** (§14). The first
buildable slice is CRUD + matching over the thin record plus the core
sub-resources; explicitly deferred, tracked in §13 / §15 and the
crate specs' §13:

- Deep PM features mirrored from source tools (custom workflows,
  automation, sprint ceremonies, time tracking).
- Tantivy full-text search over the JSONB payload + the front-end
  search box (ILIKE name search ships first).
- Binary attachments on posts / comments.
- Durable event bus (the MVP ships an in-memory stream, same shape as
  the sibling services; the durable bus is the family roadmap item —
  [`agents/share/event-bus.md`](../../agents/share/event-bus.md)).
- The cross-service link **aggregator** (`link-graph-service`); the
  plan service ships the **write-side** `entity_links` + `linked` /
  `unlinked` events only (§9.5).
- Privacy masking / GDPR export beyond the audit posture in §12
  (member / person references are personal data; full masking is
  re-assessed there).
- gRPC, Parquet import (export-only first), terminology / IANA
  registry existence checks.
