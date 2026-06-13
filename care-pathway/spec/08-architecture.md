## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|            care-pathway-front-end-with-svelte                 |
|  SvelteKit 2 SPA · Svelte 5 runes · TypeScript strict         |
|  routes: /  /new  /[pid]  /[pid]/edit                         |
|  lib/api: client.ts → care-pathways.ts (repository)           |
+------------------------------+-------------------------------+
                               | REST (raw loco JSON, no envelope)
                               | PUBLIC_API_BASE_URL (default :5150)
+------------------------------v-------------------------------+
|            care-pathway-service-rust-crate                    |
|  loco.rs 0.16 (Axum 0.8) · port 5150                          |
|  controllers/care_pathways.rs                                  |
|    CRUD + /match + /check-duplicates                          |
|  models/care_pathways.rs  (CRUD over the JSONB payload)       |
+--------------+-------------------------------+----------------+
               |  path dependency (Cargo)      |
+--------------v---------------+  +------------v---------------+
|  care-pathway-matcher        |  |  PostgreSQL (SeaORM 1.1)   |
|  pure library, no IO         |  |  care_pathways table        |
|  MatchingEngine ·            |  |  pid · name · data JSONB ·  |
|  MatchConfig · CarePathway   |  |  active · deleted_at        |
+------------------------------+  +-----------------------------+
```

Dependency direction is strictly downward: front-end → service →
matcher. The matcher depends on nothing in the workspace (serde,
strsim, unicode-normalization, thiserror only). The service declares
`care-pathway-matcher = { path = "../care-pathway-matcher-rust-crate" }`
and uses the matcher's `CarePathway` directly as its API DTO — there
is no adapter layer (contrast with the person entity).

### 8.2 Service layout (loco.rs)

```
care-pathway-service-rust-crate/
├── src/
│   ├── app.rs                       loco Hooks (routes, truncate)
│   ├── bin/main.rs                  loco CLI entrypoint
│   ├── controllers/care_pathways.rs CRUD + match + check-duplicates
│   └── models/
│       ├── care_pathways.rs         CRUD helpers over the payload
│       └── _entities/care_pathways.rs  SeaORM entity
├── migration/src/m20220101_000001_care_pathways.rs
├── config/{development,production,test}.yaml
└── tests/matching.rs                DB-free matcher-embedding tests
```

Run with `cargo loco start` (needs PostgreSQL; `auto_migrate` on in
development). The front-end runs with `pnpm dev` against
`PUBLIC_API_BASE_URL`.

### 8.3 Matching data flow

- **`/match`** — request carries `{query, candidates}`; the
  controller calls `MatchingEngine::rank` and returns the scored
  pairs. No database access.
- **`/check-duplicates`** — request carries a `CarePathway`; the
  controller loads up to 1 000 active rows, deserialises each
  payload, calls `match_care_pathways` per candidate, and returns
  hits with `is_match == true`, sorted by score. *(roadmap: replace
  the full scan with search-based candidate blocking.)*

### 8.4 Deployment topology (national health-system scale)

Today: one stateless service instance + PostgreSQL, the SPA served as
static assets. Target shape *(roadmap, §15)*, consistent with
[`agents/share/architecture.md`](../../agents/share/architecture.md)
and [`agents/share/availability.md`](../../agents/share/availability.md):

- N stateless service replicas behind a load balancer; PostgreSQL
  primary + replicas; connection pooling.
- JWT verification at the service edge against the central
  auth-service JWKS (offline, no per-request auth-service call).
- Durable event bus for CRUD events; OTLP observability pipeline.
- Per-nation deployment with cross-registry linkage through
  deterministic identifiers rather than shared databases.

### 8.5 Positioning vs FHIR PlanDefinition ecosystems

This entity is a **registry of pathway identities**, not a pathway
content store or execution engine. FHIR `PlanDefinition` / CQL / CDS
Hooks / BPM+ Health (BPMN / CMMN / DMN) systems author and execute
pathway logic; this registry tells them *which* pathway is which.
Interop is via identifiers: a `PlanDefinition.url` maps to a `Uri`
identifier or `same_as` entry; a guideline-registry id maps to
`GuidelineId`; a published pathway's DOI maps to `Doi`. Import/export
of `PlanDefinition` identifier metadata is roadmap (§15). See §17 for
the standards landscape.
