## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|              project-portfolio-management-front-end-with-svelte                  |
|  SvelteKit 2 SPA · Svelte 5 runes · TypeScript strict         |
|  routes: /plans                                               |
|          /plans/new  /plans/[pid]  /plans/[pid]/edit          |
|          /plans/[pid]/{goals,tasks,issues}                    |
|          /plans/[pid]/{timeline,burndown}                     |
|  lib/api: client.ts → plans.ts + sub-resource repos           |
+------------------------------+-------------------------------+
                               | REST (raw loco JSON, no envelope)
                               | PUBLIC_API_BASE_URL (default :5150)
+------------------------------v-------------------------------+
|              project-portfolio-management-service-with-loco                     |
|  loco.rs 0.16 (Axum 0.8) · port 5150                          |
|  controllers/plans.rs                                         |
|     CRUD + /match + /check-duplicates                         |
|                       + merge + search  (kind-agnostic)       |
|  controllers/{goals,tasks,issues}.rs   (sub-resources)        |
|  controllers/{timeline,burndown}.rs    (derived, read-only)   |
|  controllers/links.rs        entity_links write-side          |
|  controllers/bulk.rs         import/export jobs (bg_pg)        |
|  models/plans.rs + sub-resources                              |
+--------------+-------------------------------+----------------+
               |  path dependency (Cargo)      |
+--------------v---------------+  +------------v---------------+
|  project-portfolio-management-matcher           |  |  PostgreSQL (SeaORM 1.1)   |
|  pure library, no IO         |  |  plans ({…, data JSONB}) +  |
|  MatchingEngine ·            |  |  goals(via JSONB) · tasks · |
|  MatchConfig · Plan ·        |  |  issues · audit_logs ·      |
|  kind-agnostic               |  |  merge_records ·            |
|                              |  |  entity_links · bulk_jobs   |
+------------------------------+  +-----------------------------+
```

Dependency direction is strictly downward: front-end → service →
matcher. The matcher depends on nothing in the workspace (serde,
strsim, unicode-normalization, chrono/time for date proximity,
thiserror only). The service declares
`project-portfolio-management-matcher = { path = "../project-portfolio-management-matcher-rust-crate" }` and
uses the matcher's `Plan` directly as its API DTO for the thin
record — there is no adapter layer (the care-pathway posture).

### 8.2 The single collection and the matchable/operational split

The service is **two co-located concerns** behind one app:

- the **identity registry** — CRUD + matching over the thin `Plan`
  on the one `plans` collection (a JSONB `data` column + a nullable
  denormalised `parent_pid`). Matching is **kind-agnostic**: the
  controller feeds candidate plans without kind filtering and the
  matcher never gates on `kind` (§5.5).
- the **project-management tool** — the operational sub-resources
  (tasks, issues, in their own tables keyed by `parent_pid`) and the
  two derived views, hanging off any plan.

Only the registry half touches the matcher. The PM half is ordinary
relational CRUD with events + audit. The one bridge is `goals[]`
(§5.3): goal sub-resource writes mutate `data.goals[]` so the matchable
payload stays current.

### 8.3 Service layout (loco.rs)

> **Corrected 2026-08-25.** This section was headed "— planned" and drew
> a layout that never landed: a controller per sub-resource
> (`goals.rs`, `tasks.rs`, `issues.rs`, `timeline.rs`, `burndown.rs`),
> plus `links.rs` and `bulk.rs` for features §14 lists as **open gaps**
> (T-7, T-8). The real crate groups controllers by *capability* rather
> than by resource. Drawn from the tree, not from the plan:

```
project-portfolio-management-service-with-loco/
├── src/
│   ├── app.rs                       loco Hooks (routes, workers, boot init)
│   ├── bin/                         loco CLI entrypoint
│   ├── controllers/
│   │   ├── plans.rs                 CRUD + match + check-duplicates + merge + search
│   │   ├── engineering.rs           tasks, board, sprints, milestones, burndown
│   │   ├── governance.rs            proposals, gates, risks, budget, benefits
│   │   ├── strategy.rs              ideas, scenarios, objectives
│   │   ├── collaboration.rs         reviews, assignment, notifications
│   │   ├── automation.rs            rules, runs, scheduled actions
│   │   ├── prioritisation.rs        Smart Score + ranked queue
│   │   ├── visibility.rs            lifecycle funnel + readiness
│   │   ├── insights.rs              executive / financial / technology views
│   │   ├── oversight.rs             auditor, compliance, regulator extracts
│   │   ├── tba.rs                   time-based analysis (§12)
│   │   ├── compliance.rs            integrity + evidence
│   │   ├── metrics.rs docs.rs       Prometheus · OpenAPI + Swagger UI
│   ├── models/                      plans + sub-resource CRUD helpers, _entities/
│   ├── pure rule modules            governance.rs · engineering.rs · strategy.rs ·
│   │                                collaboration.rs · automation.rs ·
│   │                                prioritisation.rs · lifecycle.rs · insights.rs ·
│   │                                visibility.rs · tba.rs · merge.rs · validation.rs
│   ├── auth.rs streaming.rs relay.rs search/ privacy.rs compliance/
│   ├── flow_metrics.rs scheduler.rs snapshots.rs version.rs
│   └── workers/                     loco background jobs
├── migration/src/                   plans, audit_logs, merge_records, event_outbox,
│                                    governance, visibility, strategy, engineering,
│                                    capabilities, insight_snapshots,
│                                    integrity_digests, time_based_analysis
├── config/{development,production,test}.yaml
└── tests/                           matcher-embedding + request-level + enforcement + masking
```

**The pattern worth noting:** each capability is a **pure, DB-free rule
module** (`governance.rs`, `tba.rs`, `lifecycle.rs`, …) with a thin
controller over it. That is what makes the rules exhaustively
unit-testable without a database, and it is the shape the §6.4b / §6.4c
work should follow — a `workflow.rs`, an `okr.rs`, a `value.rs`, each
pure.

**Not present, and tracked as gaps rather than drawn as if built:**
`links.rs` / `entity_links` (T-7) and `bulk.rs` / `bulk_jobs` (T-8).

Run with `cargo loco start` (needs PostgreSQL; `auto_migrate` on in
development). The front-end runs with `pnpm dev` against
`PUBLIC_API_BASE_URL`.

### 8.4 Matching data flow

- **`/plans/match`** — request carries `{query, candidates}`
  (thin records); the controller calls `MatchingEngine::rank` and
  returns the scored pairs. No database access. `kind` labels do not
  affect scoring (§5.5).
- **`/plans/check-duplicates`** — request carries a `Plan`;
  the controller loads up to a capped set of active rows, deserialises
  each payload, calls `match_plans` per candidate, and returns hits
  with `is_match == true`, sorted by score.
  *(roadmap: replace the full scan with search-based candidate blocking
  — OQ-2.)*
- **create-time** — the same path runs on `POST /api/plans` to
  back the `409` real-time duplicate detection (FR-11a).

### 8.5 Cross-service & federation topology

A plan references other entities by `EntityRef` (sponsor org,
lead, assignees) and a parent plan by `parent_ref`, and may
carry `entity_links` to **any** entity, per
[`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md).
The portfolio service is a **write-side** participant: it stores
outbound edges locally and emits `linked` / `unlinked` events on the
bus; it does **not** call the target service on the write path
(optimistic integrity, §5 there). The read-model aggregator
(`link-graph-service`) consuming those events to answer graph queries
is a separate service, out of scope for this trio (§2.3). `EntityRef`s
are never a match signal (§7 there); `parent_ref` is an in-entity
reference and **is** a (supporting) match signal (§5.5).

### 8.6 Deployment topology (government-portfolio scale)

Target shape *(roadmap, §15)*, consistent with
[`agents/share/architecture.md`](../../agents/share/architecture.md)
and [`agents/share/availability.md`](../../agents/share/availability.md):

- N stateless service replicas behind a load balancer; PostgreSQL
  primary + replicas; connection pooling.
- PASETO v4 public token verification at the service edge against the
  central auth-service's published Ed25519 key (offline, no per-request
  auth-service call;
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  supersedes the RS256-JWT model).
- Durable event bus for CRUD + sub-resource + link events; OTLP
  observability pipeline.
- Per-department deployment with cross-registry linkage through
  deterministic identifiers and the `EntityRef` graph rather than
  shared databases.

### 8.7 Positioning vs full PM suites

> **Reversed 2026-08-25**, in step with §1.3. This section read: *"not a
> replacement for Jira / Asana / MS Project / Linear / GitHub Projects.
> Those tools own deep workflow, automation, and sprint mechanics."*
> They no longer do — §1.4 makes custom workflows, automation rules,
> time tracking, and sprint ceremonies capabilities this entity owns.

This entity is a **registry of plan identities (organised into recursive
containment trees) that is also a full project-management suite**. Both
faces are first-class (§1.1), and the second is no longer scoped to
charter level: the operational record covers goals, tasks, issues,
sprints and their ceremonies, recorded effort, configurable workflows,
automation rules, and the sequential project phase a plan is managed
through (§1.5), with derived views over all of it — timeline / Gantt,
burndown, and the Flow Framework metrics (§1.6).

**Architecturally the two faces stay separate**, and that is what makes
one record able to carry both: the thin matchable `Plan` is the API DTO,
the JSONB payload, and the matcher input, while the operational
sub-resources live in their own tables and never enter the matcher
payload (§5.6, §8.2). Adding suite depth therefore adds tables and
endpoints; it does not widen what matching sees.

**Interop is unchanged in mechanism and changed in purpose.** A Jira
project key still maps to a `JiraProjectKey` identifier, an Asana GID to
`AsanaGid`, and so on (the deterministic R-0 schemes), so a plan synced
from a source tool still deduplicates against its registry twin in the
`plans` collection. What those identifiers are *for* is now **migration
and coexistence** — running alongside a tool a department has not left
yet, and identifying the same initiative on both sides — rather than
delegating the parts this entity declined to build. See §17.
