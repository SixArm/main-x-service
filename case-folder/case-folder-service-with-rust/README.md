# Case Tracking — Loco JSON API

A back-end **JSON API** that tracks the physical location of paper
case-note folders in a UK NHS hospital setting. Implemented in Rust on
[Loco](https://loco.rs) (Axum + SeaORM). There is no built-in user
interface — front-ends consume this service over HTTP.

**Domain.** Case Tracking is a **pure aggregator API** — it
owns no domain tables. Everything it manages lives in one of five
external HTTP services:

| External service         | Owns                                                                  |
| ------------------------ | --------------------------------------------------------------------- |
| **Main Patient Service** | Patient records (NHS Number, name, date of birth)                     |
| **Main Place Service**   | Physical places — buildings, rooms, cabinets (with parent chain)      |
| **Main Worker Service**  | Workforce (clinicians, doctors, nurses, administrators, secretaries)  |
| **Main Thing Service**   | Folders — each folder is a `Thing` with `thing_type = "CaseFile"`     |
| **Main Event Service**   | Move audit log — each move is an `Event` with `event_type = "FolderMove"` |

Folders cross-reference patients (Main Patient Service UUID) and
cabinets (Main Place Service UUID). Each move event snapshots every
label (patient details, cabinet labels, worker name + role) so the
audit log remains useful even when the upstream services rename
records or go offline.

- **[Loco](https://loco.rs)** 0.16 — Rails-style Rust web framework on Axum
- **PostgreSQL** dependency is currently *vestigial* — Loco still needs a database connection at boot, but the tracker has zero local tables.
- **[main-person-service-rust-crate](../../person/person-service-rust-crate)** — Main Patient Service backend
- **[main-place-service-rust-crate](../../place/place-service-rust-crate)** — Main Place Service backend
- **[main-worker-service-rust-crate](../../worker/worker-service-rust-crate)** — Main Worker Service backend
- **[main-thing-service-rust-crate](../../thing/thing-service-rust-crate)** — Main Thing Service backend *(see caveat below)*
- **[main-event-service-rust-crate](../../event/event-service-rust-crate)** — Main Event Service backend

> ⚠️ **Thing-service caveat.** As of writing, `main-thing-service`
> ships a working web UI but **no REST endpoints yet** — its spec
> describes 15 endpoints at `/api/things/*`, but those routes aren't
> wired in the crate's `web::routes::router()`. The Loco app's HTTP
> client targets the spec'd surface and works against the in-process
> stub used in tests; against a real deployment it will fail until the
> upstream crate exposes the routes.

> ⚠️ Demo software. Same regulatory caveats as the Svelte sibling
> project apply — see [spec/database.md](spec/database.md).

## API versioning

Routes are mounted under `/api/...` with **no version prefix in the
URL**. API versioning is signalled through HTTP — clients can
negotiate via `Accept: application/json; version=1` or an explicit
`Accept: application/vnd.case-folder.v1+json` mediatype once
multiple versions exist. For now the service emits `application/json`
and treats every request as the current version.

## Prerequisites

- Rust stable (the workspace pins it via `rust-toolchain.toml`)
- A running PostgreSQL 14+ (boot-time only; no tables created)
- Optional external services (the API soft-fails or returns `503` when
  any of them is unreachable — see [spec/api-contract.md](spec/api-contract.md)):
  - **Main Patient Service** on `http://localhost:5151` — env override `MAIN_PATIENT_SERVICE_BASE_URL`
  - **Main Worker Service** on `http://localhost:5152` — env override `MAIN_WORKER_SERVICE_BASE_URL`
  - **Main Place Service** on `http://localhost:5153` — env override `MAIN_PLACE_SERVICE_BASE_URL`
  - **Main Thing Service** on `http://localhost:5154` — env override `MAIN_THING_SERVICE_BASE_URL`
  - **Main Event Service** on `http://localhost:5155` — env override `MAIN_EVENT_SERVICE_BASE_URL`
- (Optional) the Loco CLI: `cargo install loco`

## Quick start

```bash
# 1) Create the database (boot-time dependency only)
createdb case_folder_development

# 2) Set DATABASE_URL (or accept the default)
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/case_folder_development

# 3) Point at your external services (defaults shown)
export MAIN_PATIENT_SERVICE_BASE_URL=http://localhost:5151
export MAIN_WORKER_SERVICE_BASE_URL=http://localhost:5152
export MAIN_PLACE_SERVICE_BASE_URL=http://localhost:5153
export MAIN_THING_SERVICE_BASE_URL=http://localhost:5154
export MAIN_EVENT_SERVICE_BASE_URL=http://localhost:5155

# 4) (Optional) Seed demo data — registers 3 buildings/4 rooms/5 cabinets
#    in the Place service, 6 patients in the Patient service, 9 folders
#    in the Thing service, and 9 initial move events in the Event service.
cargo run -- task seed

# 5) Start the server
cargo run -- start
#  → listening on http://localhost:5150

# 6) Hit the API
curl http://localhost:5150/healthz
curl http://localhost:5150/api/stats
curl http://localhost:5150/api/folders
```

## Stub mode (no upstream services required)

Set `USE_UPSTREAM_STUBS=1` to boot the API with **in-process stubs**
for every Main-X-Service, automatically seeded with the demo data
`cargo run -- task seed` would otherwise populate against real
services:

```bash
USE_UPSTREAM_STUBS=1 cargo run -- start
```

This is the easiest way to run the Svelte sibling locally and is
required for that subproject's Playwright e2e suite.

The seed task is idempotent — re-running it skips records that already
exist. If any external service is unreachable the task warns and falls
back to placeholder UUIDs so the rest of the demo still works.

## Routes at a glance

| Method | Path                              | Purpose                                                  |
| ------ | --------------------------------- | -------------------------------------------------------- |
| GET    | `/healthz`                        | Liveness — `{ "status": "ok" }`, no downstream calls     |
| GET    | `/api/stats`                      | Aggregate counters across every Main-X-Service           |
| GET    | `/api/folders?q=&nhs_number=`     | List folders (search or by NHS Number)                   |
| POST   | `/api/folders`                    | Create a folder (finds or creates the patient)           |
| GET    | `/api/folders/{id}`               | Show a single folder                                     |
| GET    | `/api/folders/{id}/history`       | Move events for one folder                               |
| GET    | `/api/patients?q=`                | List patients (snapshot-derived; `q` is an NHS Number)   |
| GET    | `/api/patients/{nhs}`             | Show a patient + their folders + history                 |
| GET    | `/api/places?q=&kind=`            | Buildings, rooms, cabinets — optionally filtered         |
| POST   | `/api/places`                     | Create a building, room, or cabinet                      |
| GET    | `/api/places/{id}`                | Show a single place                                      |
| GET    | `/api/workers?q=`                 | List/search workers                                      |
| GET    | `/api/moves?q=`                   | Global move audit log (free-text filter)                 |
| POST   | `/api/moves`                      | Record a move (writes Event Service + Thing Service)     |

All `POST` endpoints accept `application/json` request bodies and
return the created resource at `201 Created` with a `Location` header.
Validation failures return `422 Unprocessable Entity` with
`{ "errors": { "field": "message" } }`. Unreachable upstream services
return `503 Service Unavailable` with `{ "error": "..." }`.

## Useful subcommands

```bash
cargo run -- start             # web server
cargo run -- routes            # print the route table
cargo run -- task              # list available tasks
cargo run -- task seed         # populate demo records across all services

# Tests (use stub clients — no real services needed)
DATABASE_URL=postgres://postgres@localhost:5432/case_folder_test \
  cargo test                   # 6 unit (nhs) + 29 request tests
```

The request tests need a Postgres database (`case_folder_test`
by default) for Loco's boot, but no migrations run. Create it once
with `createdb case_folder_test`. Tests run serially
(`#[serial]`) and call `clean_db()` at the top of each test which
resets every in-process stub service.

## Layout

```
case-folder-service-with-rust/
├── AGENTS.md                       ← working agreements for collaborators
├── Cargo.toml
├── README.md                       ← this file
├── index.md                        ← documentation landing page
├── rust-toolchain.toml
├── spec/                           ← full specification (topic files + SDD)
├── config/
│   ├── development.yaml
│   ├── test.yaml
│   └── production.yaml
├── migration/                      ← SeaORM migrations — intentionally empty
│   ├── Cargo.toml
│   └── src/lib.rs                  (Migrator::migrations() returns vec![])
└── src/
    ├── bin/main.rs                 ← cli::main entrypoint
    ├── lib.rs
    ├── app.rs                      ← impl Hooks for App
    ├── nhs.rs                      ← Modulus 11 validator + formatter
    ├── controllers/
    │   ├── mod.rs
    │   ├── healthz.rs              ← GET /healthz
    │   ├── stats.rs                ← GET /api/stats + latest_move_per_folder helper
    │   ├── folders.rs              ← /api/folders[/{id}[/history]]
    │   ├── moves.rs                ← /api/moves
    │   ├── patients.rs             ← /api/patients[/{nhs}]
    │   ├── places.rs               ← /api/places[/{id}]
    │   └── workers.rs              ← /api/workers
    ├── responses/
    │   └── mod.rs                  ← serializable wire structs + error helpers
    ├── initializers/
    │   ├── mod.rs
    │   ├── main_patient_service_client.rs
    │   ├── main_place_service_client.rs
    │   ├── main_worker_service_client.rs
    │   ├── main_thing_service_client.rs
    │   └── main_event_service_client.rs
    ├── main_patient_service/       ← Client trait + Patient + http + stub
    ├── main_place_service/         ← Client trait + Place + http + stub + label_path
    ├── main_worker_service/        ← Client trait + Worker + http + stub
    ├── main_thing_service/         ← Client trait + Folder projection + http + stub
    ├── main_event_service/         ← Client trait + MoveEvent projection + http + stub
    ├── models/                     ← intentionally empty
    └── tasks/
        ├── mod.rs
        └── seed.rs                 ← registers seed data with all five services
```

## See also

- [spec/](spec/index.md) — full specification (domain, use cases, route detail, response shapes, regulatory checklist, examples)
- [AGENTS.md](AGENTS.md) — working agreements for human + AI collaborators
- [index.md](index.md) — documentation landing page
- [Sibling Svelte app](../case-folder-front-end-with-svelte) — same domain, different stack
