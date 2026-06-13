# JSON contract

> Part of the [Loco edition specification](index.md). The route list is
> in [routes.md](routes.md); recipes are in [examples.md](examples.md).

A machine-readable **OpenAPI 3.1** document for every endpoint lives at
[`../openapi.yaml`](../openapi.yaml) — keep it in step with this file.

## Conventions

- Request bodies: `application/json`. The Axum `Json<T>` extractor
  parses + validates; malformed JSON yields a `400` automatically.
- Response bodies: `application/json`.
- IDs: UUID v4, serialized as canonical hyphenated lower-case strings.
- Timestamps: RFC 3339 UTC (e.g. `2026-06-02T14:30:00+00:00`).
- NHS Numbers: canonical display form `XXX XXX XXXX` in response bodies;
  either form accepted in path/query params.
- Optional fields use `null`, not omitted. List envelopes omit `query`
  when there is no search term.

## Envelopes

- **List responses** use a list envelope:

  ```json
  { "items": [ ... ], "query": "Maternity" }
  ```

  `query` is omitted when no search was requested.

- **Single resource** responses are the resource itself, no envelope.
- **Aggregate views** (`/api/stats`, `/api/patients/{nhs}`,
  `/api/places`, `/api/places/{id}`) define their own shape; see
  `src/responses/mod.rs` + each controller for the exact serde structs.

## Error shapes

| Status                 | Body                                             | When                                                  |
| ---------------------- | ------------------------------------------------ | ----------------------------------------------------- |
| `400 Bad Request`      | Loco default                                     | malformed JSON / unsupported content-type             |
| `404 Not Found`        | `{ "error": "Folder not found" }`                | resource not found                                    |
| `422 Unprocessable`    | `{ "errors": { "field": "message", ... } }`      | validation failure (each failing field is keyed)      |
| `503 Service Unavail.` | `{ "error": "Main X Service unreachable: ..." }` | upstream Main-X-Service returned an error / timed out |

Helpers for these live in `src/responses/mod.rs`: `responses::not_found`,
`responses::unprocessable`, `responses::service_unavailable`.

## Click-through aggregate shapes

Derived read-only views over the move-event log (see
[root design D-10](../../spec/design.md)). All are computed from existing
projections — no new stored data.

- **`WorkerShow`** (`GET /api/workers/{id}`):

  ```json
  {
    "worker": { "id": "…", "name": "Mira (records)", "role": "administrator" },
    "moved_folders":   [ Folder, … ],
    "patient_folders": [ Folder, … ],
    "moves":           [ Move, … ]
  }
  ```

  `moved_folders` = the distinct folders this worker has moved;
  `patient_folders` = every folder of any patient this worker has handled
  (a superset of `moved_folders`); `moves` = the worker's move events.

- **`PlaceHistory`** (`GET /api/places/{id}/history`): the `Place` fields
  are **flattened** into the top level (same convention as `PlaceShow`),
  alongside a `presences` array:

  ```json
  {
    "id": "…", "name": "Cabinet A1", "place_kind": "cabinet", "…(Place fields)…": "…",
    "presences": [
      {
        "folder_id": "…", "folder_title": "Volume 1 — General",
        "patient_id": "…", "nhs_number": "943 476 5919", "patient_name": "Alice Johnson",
        "cabinet_id": "…", "cabinet_label": "Main Hospital — Ward A — Cabinet A1",
        "entered_at": "2026-05-02T09:00:00+00:00",
        "left_at":    "2026-06-01T10:15:00+00:00",
        "entered_reason": "Folder created",
        "left_reason":    "Outpatient appointment"
      }
    ]
  }
  ```

  A `left_at` of `null` means the folder is still present. For a cabinet,
  `presences` covers that cabinet; for a room/building it aggregates all
  contained cabinets (each presence keeps its own `cabinet_id`/`cabinet_label`).
  Newest interval first.

- **`Move`** (`GET /api/moves/{id}`): the single move event, same shape as a
  row of `GET /api/moves`. `404` if the id is unknown.

- **`Volume`** (list rows + create/rename results): a movable bundle of one
  patient's folders.

  ```json
  {
    "id": "…", "title": "Alice Johnson — Vol 1",
    "patient_id": "…", "nhs_number": "943 476 5919", "patient_name": "Alice Johnson",
    "cabinet_id": "…", "cabinet_label": "Main Hospital — Ward A — Cabinet A1",
    "status": "in-cabinet", "folder_count": 2
  }
  ```

- **`VolumeShow`** (`GET /api/volumes/{id}` and the bodies of the
  add/remove/move endpoints): the `Volume` fields **flattened**, plus its
  member `folders` and the merged `history` of their move events:

  ```json
  { "…(Volume fields)…": "…", "folders": [ Folder, … ], "history": [ Move, … ] }
  ```

  `POST /api/volumes/{id}/move` accepts `{ to_cabinet_id, worker_id?, moved_by?,
  reason? }`, relocates the volume and every member folder, appends one move
  event per folder, and returns the updated `VolumeShow`. Folder rows now also
  carry `volume_id` and `volume_title` (null when unfiled).

- **`Alert`** (`GET /api/alerts`): an iFIT-style geofence breach — a move
  whose origin and destination cabinets are in **different buildings** (see
  [root design D-12](../../spec/design.md)). Derived from the move log +
  place hierarchy; newest first.

  ```json
  {
    "move_id": "…", "folder_id": "…", "folder_title": "Volume 1 — General",
    "patient_name": "Alice Johnson", "nhs_number": "943 476 5919",
    "from_building": "Main Hospital", "to_building": "Off-site Archive",
    "from_cabinet_label": "Main Hospital — Ward A — Cabinet A1",
    "to_cabinet_label": "Off-site Archive — Basement — Archive Cabinet 1",
    "moved_by": "Mira (records)", "moved_at": "2026-06-02T14:30:00+00:00",
    "reason": "Discharged to archive"
  }
  ```

## Soft-fail vs hard-fail

The default is to **fail loudly with `503`** when an upstream service is
unreachable, so callers can retry. Two intentional exceptions:

- `GET /api/stats` always returns `200` with zeros for whichever slice
  is unavailable (and a warning in the logs). A dashboard partial render
  beats a hard failure here.
- `GET /api/patients/{nhs}` falls back to Main Thing Service snapshots
  when the Main Patient Service is down (the `patient` field is `null`
  and `patient_service_match` is `false`). This preserves the
  audit-trail-survives-outages invariant.

(See the [root design D-8](../../spec/design.md) for the policy rationale.)
