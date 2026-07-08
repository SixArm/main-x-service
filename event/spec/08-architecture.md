## 8. Architecture

### 8.1 The trio

```
+--------------------------------------------------------------+
|                    Operators (browser)                        |
+------------------------------+-------------------------------+
                               |
+------------------------------v-------------------------------+
|  event-front-end-with-svelte                                 |
|  SvelteKit 2 + Svelte 5 runes + SVAR DataGrid + Lily         |
|  SPA; ApiClient + EventRepository (envelope-aware fetch)     |
+------------------------------+-------------------------------+
                               |  HTTP JSON, /api/*
+------------------------------v-------------------------------+
|  event-service-with-loco (loco.rs / Axum)                   |
|  +----------------+ +----------------+ +------------------+  |
|  | REST /api   | | FHIR (501 stub)| | gRPC (stub)      |  |
|  +----------------+ +----------------+ +------------------+  |
|  +----------------+ +----------------+ +------------------+  |
|  | Validation     | | Privacy/Mask   | | Audit log        |  |
|  +----------------+ +----------------+ +------------------+  |
|  +----------------+ +-------------------------------------+  |
|  | In-service     | | matching/adapter.rs                 |  |
|  | matcher        | |   to_matcher_event()                |  |
|  +----------------+ +------------------+------------------+  |
+------------------------------+----------|--------------------+
         |              |                 |  (library embed)
+--------v-----+ +------v-------+ +-------v-------------------+
| PostgreSQL   | | Tantivy      | | event-matcher-rust-crate  |
| (SeaORM)     | | search index | | pure library: no IO,      |
| events,      | | name+date    | | deterministic, per-field  |
| audit_log, … | | blocking     | | MatchBreakdown            |
+--------------+ +--------------+ +---------------------------+
```

### 8.2 Dependency direction

Strictly one-way:

```
front-end  →(HTTP)→  service  →(Cargo dep)→  matcher
```

- The matcher depends on nothing in the trio; it is a leaf library
  (`#![forbid(unsafe_code)]`, no tokio, no IO).
- The service embeds the matcher and re-exports it as `matcher_lib`;
  the bridge is `src/matching/adapter.rs` (§5.3).
- The front-end calls only the REST API (FR-15). It never imports
  Rust code and never touches PostgreSQL or Tantivy.

### 8.3 API versioning

The REST surface is versioned under **`/api`** (confirmed in the
service [`AGENTS/restful.md`](../event-service-with-loco/AGENTS/restful.md)
and the front-end
[spec §9](../event-front-end-with-svelte/spec/09-api-consumption.md)).
Breaking wire-format changes require a `/api/v2` — an entity-level
decision, recorded here.

### 8.4 SSO integration

The Main X Index uses the
[authentication entity](../../authentication/) as the single
sign-on provider: passwordless magic-link, RS256 JWT, JWKS published
for offline verification by peers. The event service will verify
JWTs locally against the JWKS (no per-request call to the auth
service). **Status: not yet enforced** — service T-8 / entity ET-5.

### 8.5 Deployment topology

Today (delivered): single region — N stateless service instances
behind a load balancer; PostgreSQL primary + replicas; per-instance
Tantivy index rebuilt from the database; front-end as static SPA
assets on a CDN/edge host; containers run non-root with health
checks.

Roadmap (§15, aspirational — not implemented):

- **Multi-region active-active** — regional PostgreSQL clusters with
  cross-region replication; region-local search indexes; an
  entity-wide durable event bus replacing the in-memory publisher so
  regions converge.
- **Externalised search** — Tantivy index moved out of the app
  instance (shared index service or rebuild-from-stream) so
  instances scale without per-node reindexing.
- **Edge-localised front-end** — locale negotiation per
  [`agents/share/locales.md`](../../agents/share/locales.md).

### 8.6 Data-flow summary

Create / match / merge flows are owned by the service spec
([§8.5](../event-service-with-loco/spec/08-architecture.md)); the
entity-level shape mirrors
[`agents/share/dataflow.md`](../../agents/share/dataflow.md). The
front-end adds one entity-specific flow: create-with-409 — `POST
/api/events` returning `409` is rendered inline as duplicate
candidates with links into the merge route.
