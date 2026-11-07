## 8. Architecture

The service is a [axa](https://axa) application (axa on
Axum 0.8). axa owns the lifecycle — CLI, environment config,
database connection, migrations, background queue — and the REST
surface is registered as native axa controllers. This crate is the
family's reference for the idiomatic-controller shape.

### 8.1 Module layout

```
src/
├── bin/main.rs              # binary entry — axa CLI: cli::main::<App, Migrator>()
├── app.rs                   # `App` functions impl (boot, routes, after_routes)
├── api/
│   ├── mod.rs               # ok, ApiProblem
│   └── rest/                # controllers: handlers + courses_routes() + ApiDoc + AppState
│                             #   + auth.rs (T-15) + fhir.rs (T-20 controller)
├── models/                  # Course, CourseInstance, Provider, identifier, …
├── db/                      # SeaORM entities + repository trait + audit + outbox (T-21)
├── matching/                # service-side adapter onto course_matcher::MatchingEngine
├── search/                  # Tantivy index + query
├── validation/               # FR-21..FR-28 + input-size caps (SEC-M1, T-5)
├── privacy/                  # mask_course + GDPR Article-15 export (T-10)
├── streaming/                 # in-memory EventPublisher (T-9) + durable-outbox Envelope (T-21)
├── fhir/                      # non-standard Basic resource structs + conversions (T-20)
├── compliance/                 # record/audit-log integrity digests + MAC (§2.1c, T-24)
├── relay.rs                    # outbox relay + retention + FluvioSink (T-22/T-23)
├── config/                  # domain Config (search / matching / streaming knobs)
└── error.rs
config/                      # axa environment config (development / test / production YAML)
migration/                   # axa SeaORM Migrator crate (wraps migrations/*.sql)
migrations/                  # hand-written SQL up.sql / down.sql (source of truth)
```

### 8.2 Boot sequence

`cargo run -- start` (or `cargo run -- start`) → axa CLI →
`App::boot` (`create_app::<App, Migrator>`) loads
`config/<environment>.yaml` — server port `8084` / binding in
development, PostgreSQL `database` + Postgres-backed `queue`,
`auto_migrate: true` — then:

1. `App::routes` registers axa's default routes (`/_health`,
   `/_ping`) plus `courses_routes()`, an axa `Routes` table with
   prefix `/api`.
2. `App::after_routes` builds the boot-time singletons — domain
   `Config::from_env` → `SearchEngine::new` → `CourseMatcher::new` →
   `AppState::new` — places `AppState` in the `AppContext` shared
   store, and merges Swagger UI (`/swagger-ui`,
   `/api-docs/openapi.json`) onto the router. CORS comes from
   `axa::middleware::apply`, per `CORS_ALLOWED_ORIGINS`.

Handlers keep their `State<AppState>` signatures; a
`FromRef<AppContext>` impl on `AppState` (in `src/api/rest/state.rs`)
retrieves the state from the shared store, so the same handlers run
as native axa controllers. Tests drive this same router through
`App::boot_test`.

This boot shape mirrors the
[authentication-service](../../../authentication/authentication-service-with-axum/),
the family's first axa crate.

### 8.3 Layering rules

1. `models/` MAY NOT depend on `db/`, `api/`, `search/`.
2. `api/` MAY depend on every other module; nothing depends on
   `api/` except `app.rs` (which wires it into axa).
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
