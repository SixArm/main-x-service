## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------------+
|                      Operators & Integrators                        |
|   (registry stewards, government agencies, gazetteer authorities)   |
+----------------------------+---------------------------------------+
                             |
+----------------------------v---------------------------------------+
|              place-front-end-with-svelte (SvelteKit SPA)            |
|  routes: / /places /places/new /places/[id](/edit|/audit)           |
|          /places/match /places/merge                                |
|  src/lib/api: types.ts + client.ts + places.ts (wire contract)      |
+----------------------------+---------------------------------------+
                             | REST (JSON envelope, PUBLIC_API_BASE_URL)
+----------------------------v---------------------------------------+
|              place-service-with-loco (loco.rs / Axum)              |
|  +---------------+ +----------------+ +-------------------------+   |
|  |  Validation   | |  Matching      | |  Search (Tantivy)       |   |
|  |  & Privacy    | |  adapter.rs ---+-+--> place-matcher crate  |   |
|  +---------------+ |  scoring.rs    | |  + geo-radius (bbox)    |   |
|                    +----------------+ +-------------------------+   |
|  +---------------+ +----------------+ +-------------------------+   |
|  |  Repository   | |  Audit log     | |  Event publisher        |   |
|  |  (SeaORM)     | |                | |  (in-memory; Fluvio →)  |   |
|  +---------------+ +----------------+ +-------------------------+   |
+--------+--------------------+--------------------+-----------------+
         |                    |                    |
+--------v------+  +----------v-------+  +---------v----------+
|  PostgreSQL   |  |  Tantivy index   |  |  Event stream      |
|  13 tables    |  |  names, address, |  |  PlaceCreated /    |
|  (PostGIS     |  |  identifiers,    |  |  Updated / Merged  |
|   planned)    |  |  place_type      |  |  / Deleted         |
+---------------+  +------------------+  +--------------------+
```

### 8.2 Dependency direction

Strictly one-way; no cycles:

```
place-front-end-with-svelte  --HTTP-->  place-service-with-loco  --Cargo dep-->  place-matcher-rust-crate
```

- The **matcher** depends on nothing in the trio (pure library, no IO).
- The **service** embeds the matcher (`place-matcher` in `Cargo.toml`,
  re-exported as `matcher_lib`) and bridges through
  `src/matching/adapter.rs`.
- The **front-end** knows only the service's REST API; it never links
  Rust code.

### 8.3 SSO integration

Sign-on for the whole index is the
[authentication entity](../../authentication/): passwordless
magic-link, RS256 JWT issuance, JWKS for offline verification. The
place service will verify JWTs locally against the JWKS (no per-request
call to the auth service); the front-end will redirect unauthenticated
operators to the auth front-end. **Neither is wired yet** — service
spec §13 T-8 and entity [§13](13-tasks.md) E-5.

### 8.4 Deployment topology

**Today (single region, single node):** one service instance + one
PostgreSQL + local Tantivy index directory; front-end served as static
SPA assets; Podman Compose for dev.

**Roadmap (governmental scale — aspirational, see [§15](15-roadmap.md)):**

- Stateless service replicas behind a load balancer; Kubernetes (Helm,
  HPA) with PVCs for the search index.
- PostgreSQL streaming replication; multi-region read replicas with a
  single write region (data-residency constraints: [§16](16-open-questions.md)).
- Durable event bus (Fluvio) replacing the in-memory publisher, so
  peer entities and downstream agencies can consume place events.
- PostGIS-backed spatial queries replacing the app-side Haversine
  fallback.
