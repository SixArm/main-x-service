## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|              portfolio-front-end-with-svelte                  |
|  SvelteKit 2 SPA · Svelte 5 runes · TypeScript strict         |
|  routes: /{portfolios,projects,products,programs}             |
|          …/new  …/[pid]  …/[pid]/edit                         |
|          …/[pid]/{goals,tasks,issues}                         |
|          …/[pid]/{timeline,burndown}                          |
|  lib/api: client.ts → workItems.ts + sub-resource repos       |
+------------------------------+-------------------------------+
                               | REST (raw loco JSON, no envelope)
                               | PUBLIC_API_BASE_URL (default :5150)
+------------------------------v-------------------------------+
|              portfolio-service-with-loco                     |
|  loco.rs 0.16 (Axum 0.8) · port 5150                          |
|  controllers/portfolios.rs projects.rs products.rs programs.rs|
|     (identical shape) CRUD + /match + /check-duplicates       |
|                       + merge + search  (within-kind)         |
|  controllers/{goals,tasks,issues}.rs   (sub-resources)        |
|  controllers/{timeline,burndown}.rs    (derived, read-only)   |
|  controllers/links.rs        entity_links write-side          |
|  controllers/bulk.rs         import/export jobs (bg_pg)        |
|  models/{portfolios,projects,products,programs}.rs + subs     |
+--------------+-------------------------------+----------------+
               |  path dependency (Cargo)      |
+--------------v---------------+  +------------v---------------+
|  portfolio-matcher           |  |  PostgreSQL (SeaORM 1.1)   |
|  pure library, no IO         |  |  portfolios · projects ·    |
|  MatchingEngine ·            |  |  products · programs        |
|  MatchConfig · WorkItem ·    |  |  (each {…, data JSONB}) +   |
|  kind gate (R-GATE)          |  |  goals(via JSONB) · tasks · |
|                              |  |  issues · audit_logs ·      |
|                              |  |  merge_records ·            |
|                              |  |  entity_links · bulk_jobs   |
+------------------------------+  +-----------------------------+
```

Dependency direction is strictly downward: front-end → service →
matcher. The matcher depends on nothing in the workspace (serde,
strsim, unicode-normalization, chrono/time for date proximity,
thiserror only). The service declares
`portfolio-matcher = { path = "../portfolio-matcher-rust-crate" }` and
uses the matcher's `WorkItem` directly as its API DTO for the thin
record — there is no adapter layer (the care-pathway posture).

### 8.2 The four-kind model and the matchable/operational split

The service is **two co-located concerns** behind one app:

- the **identity registry** — CRUD + matching over the thin `WorkItem`
  across **four collections** (`portfolios`, `projects`, `products`,
  `programs`), each its own table with a JSONB `data` column. Every
  collection uses the **identical** controller shape; the only
  difference is the `kind` it pins and (for the three child kinds) the
  denormalised `portfolio_pid` column. Matching is **within a
  collection only**: the controller feeds same-kind candidates and the
  matcher's kind gate (§5.5) guarantees a project is never scored
  against a product.
- the **project-management tool** — the operational sub-resources
  (tasks, issues, in their own tables keyed by `(parent_kind,
  parent_pid)`) and the two derived views, hanging off any work item.

Only the registry half touches the matcher. The PM half is ordinary
relational CRUD with events + audit. The one bridge is `goals[]`
(§5.3): goal sub-resource writes mutate `data.goals[]` so the matchable
payload stays current.

### 8.3 Service layout (loco.rs) — planned

```
portfolio-service-with-loco/
├── src/
│   ├── app.rs                       loco Hooks (routes, truncate)
│   ├── bin/main.rs                  loco CLI entrypoint
│   ├── controllers/
│   │   ├── portfolios.rs projects.rs products.rs programs.rs
│   │   │                            CRUD + match + check-duplicates + merge + search
│   │   ├── goals.rs tasks.rs issues.rs   sub-resources
│   │   ├── timeline.rs burndown.rs  derived, read-only
│   │   ├── links.rs                 entity_links write-side
│   │   ├── bulk.rs                  import/export jobs
│   │   └── docs.rs                  OpenAPI + Swagger UI
│   ├── models/                      per-kind + sub-resource CRUD helpers
│   ├── matching helpers, merge.rs, validation.rs, streaming.rs, auth.rs
│   └── workers/bulk.rs              bg_pg drain
├── migration/src/                   portfolios, projects, products, programs,
│                                    sub-resources, audit_logs, merge_records,
│                                    entity_links, bulk_jobs
├── config/{development,production,test}.yaml
└── tests/                           matcher-embedding + request-level
```

Run with `cargo loco start` (needs PostgreSQL; `auto_migrate` on in
development). The front-end runs with `pnpm dev` against
`PUBLIC_API_BASE_URL`.

### 8.4 Matching data flow

- **`/{collection}/match`** — request carries `{query, candidates}`
  (thin records, all of the collection's kind); the controller calls
  `MatchingEngine::rank` and returns the scored pairs. No database
  access. A mismatched `kind` in the payload is gated to 0.0 (§5.5).
- **`/{collection}/check-duplicates`** — request carries a `WorkItem`;
  the controller loads up to a capped set of active rows **from that
  collection**, deserialises each payload, calls `match_work_items` per
  candidate, and returns hits with `is_match == true`, sorted by score.
  *(roadmap: replace the full scan with search-based candidate blocking
  — OQ-2.)*
- **create-time** — the same path runs on `POST /api/{collection}` to
  back the `409` real-time duplicate detection (FR-11a).

### 8.5 Cross-service & federation topology

A work item references other entities by `EntityRef` (sponsor org,
lead, assignees) and a parent portfolio by `portfolio_ref`, and may
carry `entity_links` to **any** entity, per
[`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md).
The portfolio service is a **write-side** participant: it stores
outbound edges locally and emits `linked` / `unlinked` events on the
bus; it does **not** call the target service on the write path
(optimistic integrity, §5 there). The read-model aggregator
(`link-graph-service`) consuming those events to answer graph queries
is a separate service, out of scope for this trio (§2.3). `EntityRef`s
are never a match signal (§7 there); `portfolio_ref` is an in-entity
reference and **is** a (supporting) match signal for child kinds
(§5.5).

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

This entity is a **registry of work-item identities (organised under
portfolios) with a charter-level PM tool attached**, not a replacement
for Jira / Asana / MS Project / Linear / GitHub Projects. Those tools
own deep workflow, automation, and sprint mechanics; this registry
tells the portfolio *which* portfolio / project / product / program is
which, dedupes them within their kind, and tracks the charter-level
goals / tasks / issues. Interop is via identifiers: a Jira project key
maps to a `JiraProjectKey` identifier, an Asana GID to `AsanaGid`, and
so on (the deterministic R-0 schemes), so a project synced from a
source tool deduplicates against its registry twin in the `projects`
collection. See §17.
