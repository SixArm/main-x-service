## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|                    Operator / Integrator                      |
+------------------------------+-------------------------------+
                               |
+------------------------------v-------------------------------+
|  course-front-end-with-svelte          (SvelteKit SPA, :5173) |
|  SvelteKit 2 + Svelte 5 runes + SVAR DataGrid + Lily Headless |
|  src/lib/api/{types,client,courses}.ts  (envelope-aware)      |
+------------------------------+-------------------------------+
                               | HTTP JSON  /api/*  (:8084)
+------------------------------v-------------------------------+
|  course-service-with-loco              (loco.rs 0.16 / Axum) |
|  +----------------+ +-----------------+ +------------------+  |
|  | REST controllers| | Validation     | | Privacy / masking|  |
|  | (idiomatic loco)| | FR-21..FR-28   | | + GDPR export    |  |
|  +----------------+ +-----------------+ +------------------+  |
|  +----------------+ +-----------------+ +------------------+  |
|  | Repositories   | | SearchEngine    | | Audit + events   |  |
|  | (SeaORM)       | | (Tantivy)       | | (in-memory MVP)  |  |
|  +----------------+ +-----------------+ +------------------+  |
|  +-----------------------------------------------------------+|
|  | matching/adapter.rs  →  course_matcher::MatchingEngine     ||
|  +-----------------------------------------------------------+|
+-------+----------------------+-------------------------------+
        |                      |
+-------v--------+   +---------v---------+   +------------------+
|  PostgreSQL    |   |  Tantivy index    |   | course-matcher-  |
|  9 tables      |   |  (local dir)      |   | rust-crate (pure |
|  (SeaORM)      |   |                   |   | lib, no IO)      |
+----------------+   +-------------------+   +------------------+
```

Dependency direction is strictly one-way: **front-end → service →
matcher**. The matcher depends on nothing in the entity; the
front-end knows only the REST wire format.

### 8.2 Idiomatic loco controllers (reference implementation)

The service boots through loco.rs (CLI, `AppContext`, config from
`config/*.yaml`, migrations, background queue). `src/app.rs`
implements `Hooks`: REST handlers register as **native loco
controllers** in `App::routes` (`AppRoutes::with_default_routes()
.add_route(courses_routes())`, prefixed `/api`), and the boot-time
singletons (`SearchEngine`, `CourseMatcher`, domain `Config`,
`AppState`) are built once in `App::after_routes` and placed in the
`AppContext` shared store, retrieved via `FromRef<AppContext>`.
Swagger UI + CORS are layered on top there.

This crate is the **family reference** for the idiomatic-loco
controller conversion — when converting a sibling service, copy this
shape. See [`agents/share/loco.md`](../../agents/share/loco.md).

### 8.3 SSO integration (roadmap)

Sign-on is centralised in the
[authentication entity](../../authentication/): the front-end will
obtain an RS256 JWT via passwordless magic-link; the service will
verify offline against the auth service's JWKS. No per-entity user
store. Until service T-15 lands, all endpoints are unauthenticated —
acceptable for development only, blocking for any governmental
deployment (§13, §15).

### 8.4 Deployment topology

**Today (MVP):** one service instance + one PostgreSQL + a local
Tantivy index directory + the SPA served statically; events on an
in-process bus. Podman / docker-compose files ship with the service.

**Target (roadmap §15, aspirational):** stateless service replicas
behind a load balancer per region; PostgreSQL with multi-region
replication; search externalized or rebuilt per replica from the
database; durable event bus (Fluvio adapter under feature flag)
feeding downstream consumers; front-end on a CDN. The service is
already stateless-by-design except for the local search index, which
is the known scaling pinch point.

### 8.5 Data flow (entity view)

**Create:** front-end form → `POST /api/courses` → validate →
blocker (Tantivy) → adapter → matcher scoring → on duplicate `409` +
candidates rendered inline in the create form → on success INSERT →
index → audit log → event → `201` rendered as detail view.

**Match / merge:** operator match-check or merge route → service
`match` / `merge` endpoints → matcher breakdown / merge snapshot →
rendered with per-component scores.

Per-crate internals: service
[spec §8](../course-service-with-loco/spec/08-architecture.md),
matcher [spec §5](../course-matcher-rust-crate/spec/05-algorithm-overview.md),
front-end [spec §8](../course-front-end-with-svelte/spec/08-architecture.md).
