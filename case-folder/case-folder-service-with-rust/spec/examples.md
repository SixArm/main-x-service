# Examples

> Part of the [Loco edition specification](index.md). Contract details:
> [api-contract.md](api-contract.md); routes: [routes.md](routes.md).

## List folders by NHS Number

```bash
curl 'http://localhost:5150/api/folders?nhs_number=943%20476%205919'
```

```json
{
  "items": [
    {
      "id": "0e2a…",
      "title": "Volume 1",
      "patient_id": "f4c2…",
      "nhs_number": "943 476 5919",
      "patient_name": "Alice Johnson",
      "cabinet_id": "7b6e…",
      "cabinet_label": "Main Hospital — Records Room A — Cabinet A1",
      "status": "in-cabinet",
      "last_moved_at": "2026-06-01T10:15:32+00:00",
      "notes": null
    }
  ]
}
```

## Create a folder

```bash
curl -X POST http://localhost:5150/api/folders \
  -H 'content-type: application/json' \
  -d '{
    "nhs_number": "943 476 5919",
    "patient_name": "Alice Johnson",
    "date_of_birth": "1991-04-12",
    "title": "Volume 1",
    "cabinet_id": "7b6e…"
  }'
```

`201 Created` with `Location: /api/folders/{id}` and the new folder in
the body. If the patient already exists in the Main Patient Service,
`patient_name` and `date_of_birth` can be omitted.

## Record a move

```bash
curl -X POST http://localhost:5150/api/moves \
  -H 'content-type: application/json' \
  -d '{
    "folder_id": "0e2a…",
    "to_cabinet_id": "9f12…",
    "worker_id": "a8c4…",
    "reason": "Outpatient appointment"
  }'
```

`201 Created`. Omit `worker_id` and pass `moved_by: "Alice (porter)"` to
snapshot a free-text porter name instead.

## Validation failure

```bash
curl -X POST http://localhost:5150/api/folders \
  -H 'content-type: application/json' \
  -d '{ "nhs_number": "943 476 5918", "title": "General" }'
```

`422 Unprocessable Entity`:

```json
{
  "errors": { "nhs_number": "Enter a valid 10-digit NHS Number (Modulus 11)." }
}
```

## Upstream service down

```bash
# while the Main Thing Service is offline
curl http://localhost:5150/api/folders
```

`503 Service Unavailable`:

```json
{ "error": "Main Thing Service unreachable: connection refused" }
```

`GET /api/stats` and `GET /api/patients/{nhs}` are the documented
soft-fail exceptions — see [api-contract.md](api-contract.md).

## Move a whole volume (all member folders together)

```bash
# Create a volume, then move every folder it contains in one call.
curl -X POST http://localhost:5150/api/volumes \
  -H 'content-type: application/json' \
  -d '{
    "nhs_number": "943 476 5919",
    "title": "Alice Johnson — Vol 1",
    "cabinet_id": "7b6e…"
  }'

curl -X POST http://localhost:5150/api/volumes/{id}/move \
  -H 'content-type: application/json' \
  -d '{
    "to_cabinet_id": "9f12…",
    "worker_id": "a8c4…",
    "reason": "Archive room reorganisation"
  }'
```

`200 OK` with the updated `VolumeShow` (new cabinet label + the
relocated member folders). One move event is recorded per member folder,
each snapshotting the worker name + role. A bad/unknown `to_cabinet_id`
returns `422`; an upstream outage returns `503`.

The companion volume routes follow the same conventions:

```bash
# Assign an existing folder to a volume (same patient only)
curl -X POST http://localhost:5150/api/volumes/{id}/folders \
  -H 'content-type: application/json' -d '{ "folder_id": "0e2a…" }'

# Remove a folder from a volume
curl -X DELETE http://localhost:5150/api/volumes/{id}/folders/{fid}

# Rename a volume
curl -X PATCH http://localhost:5150/api/volumes/{id} \
  -H 'content-type: application/json' -d '{ "title": "Alice Johnson — Vol 2" }'
```

## List geofence-breach alerts

```bash
curl http://localhost:5150/api/alerts
```

```json
{
  "items": [
    {
      "move_id": "c1d2…",
      "folder_id": "0e2a…",
      "folder_title": "Volume 1",
      "nhs_number": "943 476 5919",
      "from_building": "Main Hospital",
      "to_building": "Annexe Building",
      "moved_by": "Joe Porter",
      "moved_at": "2026-06-01T10:15:32+00:00"
    }
  ]
}
```

Only **cross-building** moves are reported: a folder that moves between
two cabinets in the same building is not a breach, and a move with a
missing endpoint cabinet (in-transit / created-in-place) is suppressed.
`503` if the Place or Event service is unreachable.

## Add a new controller route

```rust
// src/controllers/reports.rs
use axum::{debug_handler, Json};
use loco_rs::prelude::*;
use serde_json::json;

#[debug_handler]
pub async fn index() -> Json<serde_json::Value> {
    Json(json!({ "items": [] }))
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api/reports").add("/", get(index))
}
```

Wire it up in `app.rs`:

```rust
fn routes(_ctx: &AppContext) -> AppRoutes {
    AppRoutes::with_default_routes()
        .add_route(controllers::healthz::routes())
        // ...
        .add_route(controllers::reports::routes())
}
```

Then add `pub mod reports;` to `src/controllers/mod.rs`.

## Record a move programmatically (from a task or test)

```rust
use case_folder_service_with_rust::main_event_service::RecordMove;

events
    .record(RecordMove {
        folder_id,
        patient_id,
        nhs_number: "943 476 5919".into(),
        patient_name: "Alice Johnson".into(),
        folder_title: "Volume 1".into(),
        from_cabinet_id: None,
        to_cabinet_id: Some(cabinet_id),
        from_cabinet_label: "(new folder)".into(),
        to_cabinet_label: "Main Hospital — Room A — Cabinet C1".into(),
        worker_id: None,
        moved_by: "Alice (porter)".into(),
        worker_role_snapshot: None,
        reason: Some("Outpatient appointment".into()),
    })
    .await?;
```
