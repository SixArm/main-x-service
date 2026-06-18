## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------------+
|                 worker-front-end-with-svelte (SPA)                  |
|   SvelteKit 2 · Svelte 5 runes · SVAR DataGrid · Lily Headless      |
|   ApiClient / WorkerRepository  (src/lib/api/)                      |
+----------------------------------+---------------------------------+
                                   |  HTTPS JSON  (envelope: success/data/error)
                                   |  /api/workers/*  /api/audit/*  /api/health
+----------------------------------v---------------------------------+
|                 worker-service-with-loco (loco.rs 0.16 / Axum 0.8) |
|  +-----------+ +-----------+ +------------+ +--------------------+ |
|  | REST API  | | FHIR R5   | | gRPC stub  | | Swagger UI (utoipa)| |
|  +-----------+ +-----------+ +------------+ +--------------------+ |
|  validation · privacy/masking · audit log · event publish          |
|  +--------------------+      +-----------------------------------+ |
|  | in-service matcher |      | adapter::to_matcher_worker()      | |
|  | (src/matching/)    |      | (src/matching/adapter.rs)         | |
|  +--------------------+      +-----------------+-----------------+ |
+---------+----------------------+---------------|-------------------+
          |                      |               |  embeds (Cargo dep)
+---------v---------+  +---------v--------+  +---v-------------------+
|  PostgreSQL 18    |  |  Tantivy index   |  |  worker-matcher 0.6.1 |
|  (SeaORM, 12+     |  |  (11 fields,     |  |  pure library: no IO, |
|  tables, audit)   |  |  on disk)        |  |  no unsafe, determin. |
+-------------------+  +------------------+  +-----------------------+
```

### 8.2 Dependency direction

Strictly one-way: **front-end → service → matcher**.

- The front-end depends only on the service's REST contract; it never
  imports Rust types or talks to the database / matcher.
- The service depends on the matcher as a normal Cargo dependency
  (crates.io, SemVer-pinned) — not a path dependency — and reaches it
  only through the adapter (§5.3).
- The matcher depends on nothing in this repository. It MUST stay a
  pure library (no IO, no async runtime, no `unsafe`).

### 8.3 SSO integration

The [authentication entity](../../authentication/) is the central
single sign-on provider: passwordless email magic-link, RS256 JWT
issuance, JWKS endpoint for offline verification. Target wiring
(service §13 T-1, front-end out-of-scope note in its `AGENTS.md`):

1. Operator signs in at the authentication front-end; receives a JWT.
2. Worker front-end sends `Authorization: Bearer <jwt>` on every call.
3. Worker service verifies the RS256 signature against the cached
   JWKS — no per-request call to the auth service — and enforces
   roles.

Today no JWT is enforced anywhere in the trio; this is the top
blocker in §13 / §15.

### 8.4 Deployment topology

**Current:** single node per subproject — service + PostgreSQL via
Docker/Podman Compose, front-end as a static SPA (Vite dev server or
any static host), Tantivy index on local disk, in-memory event
publisher.

**Target — multi-region governmental scale (roadmap §15, not
delivered):**

- Stateless service replicas behind regional load balancers
  (Kubernetes + HPA per service roadmap).
- PostgreSQL with cross-region replication; read replicas for search
  hydration and audit queries.
- Durable event bus (Fluvio / Kafka / NATS) replacing the in-memory
  publisher; consumers feed downstream agencies.
- Externalised / replicated search index (today's local-disk Tantivy
  index is the single-node constraint).
- Front-end behind a CDN; per-locale builds.

### 8.5 Data flow (cross-subproject)

**Operator creates a worker:** front-end form → `POST /api/workers` →
service validation → blocking via Tantivy → candidate scoring (adapter
→ canonical matcher) → if duplicates: `409` with candidates, surfaced
inline by the front-end → else INSERT + index + event + audit → `201`.

In-crate flow detail:
[service §8.5](../worker-service-with-loco/spec/08-architecture.md),
[`agents/share/dataflow.md`](../../agents/share/dataflow.md).
