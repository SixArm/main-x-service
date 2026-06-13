## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|              organization-front-end-with-svelte               |
|  SvelteKit 2 SPA (Svelte 5 runes, TS strict, no data grid)    |
|  routes: /  /new  /[pid]  /[pid]/edit                         |
|  lib/api: client.ts -> organizations.ts (repository)          |
+------------------------------+--------------------------------+
                               | HTTP, raw JSON (no envelope)
                               | PUBLIC_API_BASE_URL (default :5150)
+------------------------------v--------------------------------+
|               organization-service-rust-crate                 |
|  loco.rs 0.16 app (Axum 0.8)                                  |
|  +----------------------+  +---------------------------+      |
|  | controllers/         |  | controllers/docs.rs       |      |
|  | organizations.rs     |  | /api-docs/openapi.json    |      |
|  | CRUD+search+match+   |  | /swagger-ui               |      |
|  | dup+audit+events     |  +---------------------------+      |
|  +----+--------+--------+                                     |
|       |        |     +-----------------+  +----------------+  |
|       |        +---->| models/audit_   |  | streaming.rs   |  |
|       |              | logs.rs         |  | in-mem ring    |  |
|       v              +-----------------+  | buffer (1000)  |  |
|  +----------------------+                 +----------------+  |
|  | models/              |                                     |
|  | organizations.rs     |--- embeds, calls directly --------+ |
|  +----------+-----------+                                   | |
+-------------|-----------------------------------------------|-+
              |                                               |
+-------------v---------------+   +---------------------------v--+
|  PostgreSQL (SeaORM 1.1)    |   |  organization-matcher        |
|  organizations  (JSONB data)|   |  pure library: MatchingEngine|
|  audit_logs     (snapshots) |   |  no IO, no unsafe, no panics |
+-----------------------------+   +------------------------------+
```

### 8.2 Dependency direction

Strictly one-way; the matcher is the leaf:

- `organization-front-end-with-svelte` → service REST API (HTTP only;
  no shared code — drift policy).
- `organization-service` → `organization-matcher` (Cargo path
  dependency; the matcher type is re-used as the DTO, called directly
  on deserialised payloads — **no adapter**).
- `organization-matcher` → nothing but `serde` + small string/Unicode
  helpers. It MUST NOT acquire IO, async, logging, or service
  dependencies.

### 8.3 Service (loco.rs) structure

```
organization-service-rust-crate/
├── src/
│   ├── app.rs                      loco Hooks: registers organizations + docs routes
│   ├── bin/main.rs                 loco CLI entrypoint (`cargo loco start`)
│   ├── controllers/
│   │   ├── organizations.rs        CRUD + search + match + check-duplicates + audit + events
│   │   └── docs.rs                 OpenAPI JSON + Swagger UI
│   ├── models/
│   │   ├── organizations.rs        CRUD helpers over the JSONB payload (create/find_by_pid/search/list/…)
│   │   ├── audit_logs.rs           record / recent / for_entity
│   │   └── _entities/              SeaORM entities
│   ├── openapi.rs                  hand-written OpenAPI 3 document
│   └── streaming.rs                in-memory OrgEvent ring buffer (OnceLock global)
├── migration/src/                  m…_organizations, m…_audit_logs
└── config/                         development / production / test yaml (auto_migrate in dev)
```

Note: the loco scaffolding leftovers (`src/workers/downloader.rs`,
empty `src/data/` + `src/tasks/`) were removed 2026-06-13 (§13 T-12);
the app registers no background workers.

### 8.4 Data flows

**Create:** HTTP POST → name-required guard (`422` if blank) →
`OrgModel::create`
(serialize payload → INSERT) → audit `created` (best-effort, with
snapshot) → publish `Created` event → `{pid, name}` response.

**Check-duplicates:** HTTP POST → load active rows (cap 1 000) →
deserialise each `data` payload → `MatchingEngine::match_organizations`
per candidate → keep `is_match`, sort by score desc → response.

**Match:** HTTP POST `{query, candidates}` → `MatchingEngine::rank`
→ ranked results. Stateless; no DB access.

### 8.5 Deployment topology

**Today:** one service instance + one PostgreSQL database; front-end
served as static SPA assets; Swagger UI for exploration. Suitable for
development and pilot registries.

**Roadmap (§15, aspirational):** stateless service replicas behind a
load balancer per region; PostgreSQL with replication and
read-replicas; durable event bus replacing the in-memory buffer;
externalised search; JWKS-verified JWTs from the central
authentication service. The service is already stateless **except**
for the in-memory event buffer — replacing it is the gating item for
horizontal scale-out.
