## 8. Architecture

The service is a [loco.rs](https://loco.rs) application (loco 0.16 on
Axum 0.8). Loco owns the lifecycle — CLI, environment config,
database connection, migrations, background queue — and the REST
surface is registered as native loco controllers. This crate is the
family's reference for the idiomatic-controller shape.

### 8.1 Module layout

```
src/
├── bin/main.rs              # binary entry — loco CLI: cli::main::<App, Migrator>()
├── app.rs                   # loco Hooks impl (boot, routes, after_routes)
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   └── rest/                # controllers: handlers + courses_routes() + ApiDoc + AppState
├── models/                  # Course, CourseInstance, Provider, identifier, …
├── db/                      # SeaORM entities + repository trait + audit
├── matching/                # service-side adapter onto course_matcher::MatchingEngine
├── search/                  # Tantivy index + query
├── config/                  # domain Config (search / matching / streaming knobs)
└── error.rs
config/                      # loco environment config (development / test / production YAML)
migration/                   # loco SeaORM Migrator crate (wraps migrations/*.sql)
migrations/                  # hand-written SQL up.sql / down.sql (source of truth)
```

### 8.2 Boot sequence

`cargo loco start` (or `cargo run -- start`) → loco CLI →
`App::boot` (`create_app::<App, Migrator>`) loads
`config/<environment>.yaml` — server port `8084` / binding in
development, PostgreSQL `database` + Postgres-backed `queue`,
`auto_migrate: true` — then:

1. `App::routes` registers loco's default routes (`/_health`,
   `/_ping`) plus `courses_routes()`, a loco `Routes` table with
   prefix `/api`.
2. `App::after_routes` builds the boot-time singletons — domain
   `Config::from_env` → `SearchEngine::new` → `CourseMatcher::new` →
   `AppState::new` — places `AppState` in the `AppContext` shared
   store, and merges Swagger UI (`/swagger-ui`,
   `/api-docs/openapi.json`) plus permissive CORS onto the router.

Handlers keep their `State<AppState>` signatures; a
`FromRef<AppContext>` impl on `AppState` (in `src/api/rest/state.rs`)
retrieves the state from the shared store, so the same handlers run
as native loco controllers. The hand-built Axum router
(`create_router`) is retained solely for the `tower::oneshot`
integration tests.

This boot shape mirrors the
[authentication-service](../../../authentication/authentication-service-with-loco/),
the family's first loco crate.

### 8.3 Layering rules

1. `models/` MAY NOT depend on `db/`, `api/`, `search/`.
2. `api/` MAY depend on every other module; nothing depends on
   `api/` except `app.rs` (which wires it into loco).
3. `matching/` is a thin adapter over the canonical
   [`course-matcher`](../../course-matcher-rust-crate/) crate.
4. `search/` MAY depend on `models/` only.

### 8.4 Data flow

**Create:** HTTP POST → validate → duplicate-detection (search +
matcher) → on duplicate return `409 MatchResult[]`; on success
repository INSERT → search index → audit log → event publish →
response.

**Match:** HTTP POST → blocker (`search_by_name_and_provider`) →
load candidates → `CourseMatcher::find_matches` → renormalised
weighted score → response.

**Merge:** HTTP POST → fetch both → fold identifiers / instances /
syllabus / links into main → update main → soft-delete duplicate →
update index → audit log + `CourseMerged` event → response.
