# Domain model

> Part of the [Loco edition specification](index.md). Shared,
> edition-independent domain (entities, invariants, status vocabulary):
> [root domain-model](../../spec/domain-model.md). This file adds the
> Loco-specific service split and the upstream **Client trait**
> interfaces.

The Loco edition splits the world in five — and the tracker owns
_none_ of it:

- **Patient records** live in the **Main Patient Service**. Folders
  reference a `patient_id` (opaque UUID) plus NHS Number / name
  snapshots in their `keywords` + `identifiers`.
- **Places (buildings, rooms, cabinets)** live in the **Main Place
  Service**. Folders reference a cabinet UUID via the Thing's
  `contained_in_thing` field plus a `cabinet_path` keyword snapshot.
- **Workers** live in the **Main Worker Service**. Move events reference
  a `worker_id` (opaque UUID) plus name + role snapshots.
- **Folders** live in the **Main Thing Service** as `Thing` records with
  `thing_type = Other("CaseFile")`.
- **Move audit events** live in the **Main Event Service** as `Event`
  records with `event_type = Other("FolderMove")`.

```
   ┌────── Main Patient Service ──────┐ ┌────── Main Place Service ──────┐
   │ Patient(id, nhs_number, name, …) │ │ Place(id, name, place_type,    │
   └──────────────────────────────────┘ │       contained_in_place, …)   │
                ▲ patient_id            └────────────────────────────────┘
                │                                ▲ cabinet_id
                │                                │
                ┌─────── Main Thing Service ─────────┐
                │ Folder = Thing {                   │
                │   thing_type = Other("CaseFile"),  │
                │   contained_in_thing = cabinet_id, │
                │   identifiers = [NHS Number],      │
                │   keywords = [patient_id=…,        │
                │               patient_name=…,      │
                │               cabinet_path=…],     │
                │ }                                  │
                └────────────────────────────────────┘
                              ▲ folder_id
                              │
                ┌─────── Main Event Service ─────────┐
                │ MoveEvent = Event {                │
                │   event_type = Other("FolderMove"),│
                │   start_date = moved_at,           │
                │   name = folder_title,             │
                │   description = reason,            │
                │   keywords = [folder_id=…,         │
                │               patient_id=…,        │
                │               nhs_number=…,        │
                │               from/to_cabinet_*=…, │
                │               worker_id=…,         │
                │               worker_role=…],      │
                │ }                                  │
                └────────────────────────────────────┘
```

## Tables

There are **no local tables**. The `migration` crate's `Migrator`
returns an empty `Vec`. PostgreSQL is still a boot-time dependency
because Loco's `create_app::<Self, Migrator>(...)` insists on a `db`
field in `AppContext`; nothing else in the codebase touches it. See
[database.md](database.md).

## Invariants

The shared invariants are in [root domain-model](../../spec/domain-model.md#invariants).
Loco-specific notes:

1. **NHS Number uniqueness** is owned by the Main Patient Service.
2. **`folders.(patient_id, title)` is `UNIQUE`** — a patient can have
   many folders but each must have a distinct title.
3. **Move events are append-only.** Patient name, NHS Number, folder
   title, cabinet labels, **and worker name + role** are all
   **snapshotted** into the Event's `keywords` at `record()` time.
4. **All cross-service references are by opaque UUID, with no referential
   integrity** between services; the only reconciliation is the snapshots.
5. **Folder status is derived** from the latest move event:
   `to_cabinet_id` is `Some(_)` → `in-cabinet`; `None` → `in-transit`.
   With no move history, fall back to `contained_in_thing`.
6. **Snapshot fields are written at action time** and never refreshed.

## Main Patient Service client interface

```rust
pub trait Client: Send + Sync {
    async fn find_by_nhs_number(&self, nhs: &str) -> Result<Option<Patient>, Error>;
    async fn find_by_id(&self, id: Uuid)         -> Result<Option<Patient>, Error>;
    async fn create(&self, input: CreatePatient) -> Result<Patient, Error>;
}
```

Implementations live in `main_patient_service::`:

- `main_patient_service::http::HttpClient` — `reqwest`-backed; talks to
  `{MAIN_PATIENT_SERVICE_BASE_URL}/api/persons[/...]`. Wraps NHS
  Number as a FHIR-style Identifier (`identifier_type: OTHER`,
  `system: https://fhir.nhs.uk/Id/nhs-number`).
- `main_patient_service::stub::StubClient` — in-process
  `Mutex<Vec<Patient>>` used by request tests.

The Axum router is layered with a private `RoutingClient` that checks a
`Mutex<Option<Arc<dyn Client>>>` at request time — tests overwrite that
slot via `initializers::main_patient_service_client::set_test_client`,
the live app falls through to the HTTP client.

## Main Thing Service client interface

```rust
pub trait Client: Send + Sync {
    async fn search(&self, query: &str)                      -> Result<Vec<Folder>, Error>;
    async fn find_by_id(&self, id: Uuid)                     -> Result<Option<Folder>, Error>;
    async fn list_for_patient(&self, patient_id: Uuid)       -> Result<Vec<Folder>, Error>;
    async fn list_for_nhs_number(&self, nhs_number: &str)    -> Result<Vec<Folder>, Error>;
    async fn create(&self, input: NewFolder)                 -> Result<Folder, Error>;
    async fn update_cabinet(&self, id, cabinet_id, snapshot) -> Result<Folder, Error>;
}
```

The `Folder` projection mirrors the legacy tracker schema:
`{ id, title, patient_id, nhs_number_snapshot, patient_name_snapshot,
cabinet_id, cabinet_path_snapshot, notes, volume_id, volume_title_snapshot }`.
The HTTP client packs these into the upstream `Thing` shape with
`thing_type = Other("CaseFile")`, storing NHS Number as a
`ThingIdentifier { property_id: Custom("nhs-number"), value }` and
patient/cabinet/volume snapshots as `keywords` entries
(`volume_id=…`, `volume_title=…`). See `main_thing_service::http` for the
full mapping table.

### Volumes (Thing `thing_type = Other("Volume")`)

A **volume** is a movable bundle of one patient's folders (see
[root domain-model](../../spec/domain-model.md)). It is stored in the same
Main Thing Service as a `Thing` with `thing_type = Other("Volume")`, with a
`Volume` projection `{ id, title, patient_id, nhs_number_snapshot,
patient_name_snapshot, cabinet_id, cabinet_path_snapshot }`. A folder points
at its volume via the `volume_id` keyword; a volume's members are the folders
whose `volume_id` matches. The Thing `Client` trait gains:

```rust
async fn create_volume(&self, input: NewVolume)            -> Result<Volume, Error>;
async fn find_volume_by_id(&self, id: Uuid)                -> Result<Option<Volume>, Error>;
async fn list_volumes(&self)                               -> Result<Vec<Volume>, Error>;
async fn rename_volume(&self, id: Uuid, title: String)     -> Result<Volume, Error>;
async fn update_volume_cabinet(&self, id, cabinet_id, snap)-> Result<Volume, Error>;
async fn set_folder_volume(&self, folder_id, volume_id, volume_title) -> Result<Folder, Error>;
```

Moving a volume is orchestrated in `controllers/volumes.rs`: it calls
`update_volume_cabinet` for the volume and, per member folder, records a
move event (Main Event Service) and `update_cabinet` (Main Thing Service).

## Main Event Service client interface

```rust
pub trait Client: Send + Sync {
    async fn record(&self, input: RecordMove)             -> Result<MoveEvent, Error>;
    async fn list_all(&self)                              -> Result<Vec<MoveEvent>, Error>;
    async fn list_recent(&self, limit: u32)               -> Result<Vec<MoveEvent>, Error>;
    async fn list_for_folder(&self, folder_id: Uuid)      -> Result<Vec<MoveEvent>, Error>;
    async fn list_for_patient(&self, patient_id: Uuid)    -> Result<Vec<MoveEvent>, Error>;
}
```

The `MoveEvent` projection: `{ id, folder_id, patient_id, nhs_number,
patient_name, folder_title, from/to_cabinet_id, from/to_cabinet_label,
worker_id, moved_by, worker_role_snapshot, moved_at, reason }`.

## Main Place Service client interface

```rust
pub trait Client: Send + Sync {
    async fn search(&self, query: &str, place_type: Option<&str>) -> Result<Vec<Place>, Error>;
    async fn find_by_id(&self, id: Uuid)                          -> Result<Option<Place>, Error>;
    async fn create(&self, input: CreatePlace)                    -> Result<Place, Error>;
}
```

`Place { id, name, place_type: Option<String>, description,
contained_in_place: Option<Uuid>, capacity }`. The upstream `PlaceType`
is an enum with `Hospital`, `Landform`, …, and `Other(String)`. The
tracker only cares about three values, exposed as constants:

- `PlaceType::HOSPITAL` (`"Hospital"`) — a building
- `PlaceType::RECORDS_ROOM` (`"RecordsRoom"`) — a room
- `PlaceType::FILE_CABINET` (`"FileCabinet"`) — a cabinet

Hierarchy is expressed via `contained_in_place` (a room points at its
building, a cabinet at its room). The free helper `label_path(client,
cabinet_id) -> String` walks that chain (with a depth-4 guard) and
returns `"Building — Room — Cabinet"`. Controllers call it once when a
folder is created or moved and snapshot the result onto
`folders.cabinet_path_snapshot` / `move_events.to_cabinet_label`.

## Main Worker Service client interface

```rust
pub trait Client: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<Worker>, Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Worker>, Error>;
}
```

`Worker { id: Uuid, name: String, role: Option<String> }`. The role
string comes straight from the service's `worker_type` enum (`doctor`,
`nurse`, `carer`, `staff`, `employee`, `manager`, `supervisor`,
`consultant`, `other`); the tracker treats it opaquely and just emits it
on the wire.

### Worker on a move

`POST /api/moves` accepts both `worker_id` (an MWS UUID) and an optional
free-text `moved_by` field:

- If `worker_id` is set we call `mws.find_by_id(...)` and snapshot the
  authoritative name + role.
- If not set we snapshot the typed `moved_by` with `worker_id = NULL`
  and `worker_role_snapshot = NULL`.
- If `moved_by` is also blank, the snapshot reads `"Unknown porter"`.
