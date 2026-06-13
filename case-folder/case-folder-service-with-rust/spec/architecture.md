# Architecture

> Part of the [Loco edition specification](index.md). Project-level view
> of how the two editions fit: [root architecture](../../spec/architecture.md).

## Layered diagram

```
┌──────────────────────────────────────────────────────────────────┐
│  HTTP (JSON)                                                       │
│   /healthz, /api/stats, /api/folders[/…], /api/places[/…],         │
│   /api/moves, /api/patients[/…], /api/workers                      │
├──────────────────────────────────────────────────────────────────┤
│  src/controllers/*                                                 │
│    - axum #[debug_handler]s returning Response                     │
│    - axum::extract::Json for request bodies                        │
│    - 201/Location for creates, 422 for validation, 503 upstream    │
├──────────────────────────────────────────────────────────────────┤
│  src/responses/mod.rs                                              │
│    - serializable wire structs (Patient/Place/Folder/Move/…)       │
│    - error helpers: not_found / unprocessable / service_unavail.   │
├──────────────────────────────────────────────────────────────────┤
│  src/main_{patient,place,worker,thing,event}_service/*             │
│    - Client trait + Http impl (reqwest) + Stub impl                │
│    - private RoutingClient injected via Extension layer            │
├──────────────────────────────────────────────────────────────────┤
│  PostgreSQL (vestigial — connection only, no tables)               │
└──────────────────────────────────────────────────────────────────┘
```

## Loco lifecycle

```
cargo run -- start
   │
   ▼
cli::main::<App, Migrator>()
   │
   ▼
App::boot(...) → create_app::<App, Migrator>(...)
   │
   ├─ Migrator runs pending migrations (none)
   ├─ initializers() registers RoutingClient layers for each Main-X-Service
   └─ routes() composes AppRoutes
         ├─ controllers::healthz::routes()  → /healthz
         ├─ controllers::stats::routes()    → /api/stats
         ├─ controllers::folders::routes()  → /api/folders[/{id}[/history]]
         ├─ controllers::moves::routes()    → /api/moves
         ├─ controllers::patients::routes() → /api/patients[/{nhs}]
         ├─ controllers::places::routes()   → /api/places[/{id}]
         └─ controllers::workers::routes()  → /api/workers
```

## File layout

```
case-folder-service-with-rust/
├── AGENTS.md
├── Cargo.toml
├── README.md
├── index.md
├── rust-toolchain.toml
├── spec/                       ← this specification
├── config/
│   ├── development.yaml
│   ├── test.yaml
│   └── production.yaml
├── migration/
│   ├── Cargo.toml
│   └── src/lib.rs               MigratorTrait → vec![]
└── src/
    ├── bin/main.rs              cli::main::<App, Migrator>()
    ├── lib.rs
    ├── app.rs                   impl Hooks for App
    ├── nhs.rs                   Modulus 11 + formatter + unit tests
    ├── controllers/
    │   ├── mod.rs
    │   ├── healthz.rs           GET /healthz
    │   ├── stats.rs             GET /api/stats + latest_move_per_folder
    │   ├── folders.rs           list / create / show / history
    │   ├── moves.rs             list / create
    │   ├── patients.rs          list / show
    │   ├── places.rs            list / create / show
    │   └── workers.rs           list
    ├── responses/
    │   └── mod.rs               wire structs + error helpers
    ├── initializers/
    │   ├── mod.rs
    │   ├── main_patient_service_client.rs
    │   ├── main_place_service_client.rs
    │   ├── main_worker_service_client.rs
    │   ├── main_thing_service_client.rs
    │   └── main_event_service_client.rs
    ├── main_patient_service/    Client + Http + Stub
    ├── main_place_service/      Client + Http + Stub + label_path
    ├── main_worker_service/     Client + Http + Stub
    ├── main_thing_service/      Client + Http + Stub
    ├── main_event_service/      Client + Http + Stub
    ├── models/                  empty
    └── tasks/
        ├── mod.rs
        └── seed.rs              registers seed data with all five services
```
