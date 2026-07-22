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

### 8.3 Service layout (loco.rs) — planned

```
project-portfolio-management-service-with-loco/
├── src/
│   ├── app.rs                       loco Hooks (routes, truncate)
│   ├── bin/main.rs                  loco CLI entrypoint
│   ├── controllers/
│   │   ├── plans.rs                 CRUD + match + check-duplicates + merge + search
│   │   ├── goals.rs tasks.rs issues.rs   sub-resources
│   │   ├── timeline.rs burndown.rs  derived, read-only
│   │   ├── links.rs                 entity_links write-side
│   │   ├── bulk.rs                  import/export jobs
│   │   └── docs.rs                  OpenAPI + Swagger UI
│   ├── models/                      plans + sub-resource CRUD helpers
│   ├── matching helpers, merge.rs, validation.rs, streaming.rs, auth.rs
│   └── workers/bulk.rs              bg_pg drain
├── migration/src/                   plans, sub-resources, audit_logs, merge_records,
│                                    entity_links, bulk_jobs
├── config/{development,production,test}.yaml
└── tests/                           matcher-embedding + request-level
```

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

This entity is a **registry of plan identities (organised into
recursive containment trees) with a charter-level PM tool attached**,
not a replacement for Jira / Asana / MS Project / Linear / GitHub
Projects. Those tools own deep workflow, automation, and sprint
mechanics; this registry tells the portfolio *which* plan is which,
dedupes them, and tracks the charter-level goals / tasks / issues.
Interop is via identifiers: a Jira project key maps to a
`JiraProjectKey` identifier, an Asana GID to `AsanaGid`, and so on (the
deterministic R-0 schemes), so a project synced from a source tool
deduplicates against its registry twin in the `plans` collection. See
§17.
