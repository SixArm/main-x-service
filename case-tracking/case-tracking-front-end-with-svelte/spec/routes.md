# Use cases → routes & API calls

> Part of the [Svelte edition specification](index.md). The use-case
> catalogue is shared; the API side of each UC is in the
> [Loco routes](../../case-tracker-service-with-rust/spec/routes.md).

## Use cases → routes

| UC   | Trigger                          | Route                       | API calls                                                                                       |
| ---- | -------------------------------- | --------------------------- | ----------------------------------------------------------------------------------------------- |
| UC-1 | Move a folder                    | `/move`                     | `GET /api/places?kind=cabinet`, `GET /api/workers`, `GET /api/folders?nhs_number=`, `POST /api/moves` |
| UC-2 | Register a new folder            | `/folders/new`              | `GET /api/places?kind=cabinet`, `POST /api/folders`                                              |
| UC-3 | Find a folder                    | `/folders[?q=]`             | `GET /api/folders?q=`                                                                            |
| UC-4 | Audit folder history             | `/history[?q=]`             | `GET /api/moves?q=`                                                                              |
| UC-5 | Inspect cabinet utilisation      | `/` and `/cabinets`         | `GET /api/stats`, `GET /api/places`                                                              |
| UC-6 | Register a new cabinet           | `/cabinets/new`             | `GET /api/places`, `POST /api/places` (`kind=cabinet`)                                           |
| UC-7 | Register a new building / room   | `/buildings/new`, `/buildings/{id}` | `POST /api/places` (`kind=building` / `kind=room`)                                       |
| UC-8 | View a patient's folders         | `/patients[/{nhs}]`         | `GET /api/patients`, `GET /api/patients/{nhs}`                                                   |
| UC-W1 | View a worker's folders         | `/workers[/{id}]`           | `GET /api/workers`, `GET /api/workers/{id}`                                                      |
| UC-P1 | View a place's presence history  | `/cabinets/{id}`, `/rooms/{id}`, `/buildings/{id}` | `GET /api/places/{id}`, `GET /api/places/{id}/history`                          |
| UC-E1 | View a move event's detail       | `/history/{id}`             | `GET /api/moves/{id}`, `GET /api/patients/{nhs}` (sibling folders)                               |
| UC-V1 | Create / rename a volume         | `/volumes/new`, `/volumes/{id}` | `POST /api/volumes`, `PATCH /api/volumes/{id}`                                               |
| UC-V2 | Add / remove a folder            | `/volumes/{id}`             | `POST /api/volumes/{id}/folders`, `DELETE /api/volumes/{id}/folders/{fid}`                       |
| UC-V3 | View a volume                    | `/volumes[/{id}]`           | `GET /api/volumes`, `GET /api/volumes/{id}`                                                      |
| UC-V4 | Move a whole volume              | `/volumes/{id}`             | `POST /api/volumes/{id}/move`                                                                    |
| UC-I1 | Review geofence alerts           | `/alerts`                   | `GET /api/alerts`                                                                                |
| UC-I2 | Run reports                      | `/reports`                  | `GET /api/stats`, `GET /api/places`, `GET /api/moves`, `GET /api/volumes`, `GET /api/workers`    |
| UC-I3 | Scan to move                     | `/scan`                     | `GET /api/folders?nhs_number=`                                                                   |

Folder→history (`/folders/{id}`) and patient→folders (`/patients/{nhs}`)
already existed and are unchanged.

## Negative cases

| ID    | Scenario                                                       | Expected behaviour                                                                 |
| ----- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| UC-N1 | Move when the NHS Number has no folders                        | Form disables the "Folder" picker; submit blocked                                  |
| UC-N2 | Move / create with an invalid NHS Number                       | Client blocks (Modulus 11); API would also `422`                                   |
| UC-N3 | `GET /api/folders/{unknown-id}` returns 404                    | `+page.ts` throws → `+error.svelte` shows "Folder not found"                       |
| UC-N4 | Create a duplicate folder per upstream rules                   | API returns `422`; page surfaces the field-level error                             |
| UC-N5 | API unreachable                                                | `+page.ts` throws → `+error.svelte` shows the connection error and how to start the API |

## Routes & API calls

| Route                          | Load calls                                                                                    | Mutation                                          |
| ------------------------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| `/`                            | `api.stats`, `api.places.list`, `api.folders.list`, `api.moves.list`, `api.patients.list`     | —                                                 |
| `/patients`                    | `api.patients.list`                                                                            | —                                                 |
| `/patients/{nhs}`              | `api.patients.show`                                                                            | —                                                 |
| `/folders[?q=]`                | `api.folders.list({q})`                                                                        | —                                                 |
| `/folders/new`                 | `api.places.list({kind: 'cabinet'})`                                                           | `cache.addFolder` → `POST /api/folders`           |
| `/folders/{id}`                | `api.folders.show`, `api.folders.history`                                                     | —                                                 |
| `/buildings`                   | `api.places.list`                                                                              | —                                                 |
| `/buildings/{id}`              | `api.places.show`, `api.places.list`                                                          | `cache.addRoom` → `POST /api/places` (`room`)     |
| `/buildings/new`               | —                                                                                              | `cache.addBuilding` → `POST /api/places` (`bldg`) |
| `/cabinets`                    | `api.places.list`                                                                              | —                                                 |
| `/cabinets/new`                | `api.places.list`                                                                              | `cache.addCabinet` → `POST /api/places` (`cab`)   |
| `/move`                        | `api.places.list({kind: 'cabinet'})`, `api.workers.list`. On-the-fly: `api.folders.list({nhs_number})` | `cache.recordMove` → `POST /api/moves`            |
| `/history[?q=]`                | `api.moves.list({q})`                                                                          | —                                                 |
| `/history/{id}`                | `api.moves.show(id)`, `api.patients.show(nhs)` (sibling folders)                               | —                                                 |
| `/workers`                     | `api.workers.list`                                                                             | —                                                 |
| `/workers/{id}`                | `api.workers.show(id)`                                                                         | —                                                 |
| `/cabinets/{id}`               | `api.places.show(id)`, `api.places.history(id)`                                                | —                                                 |
| `/rooms/{id}`                  | `api.places.show(id)`, `api.places.history(id)`                                                | —                                                 |
| `/volumes`                     | `api.volumes.list`                                                                             | —                                                 |
| `/volumes/new`                 | `api.places.list({kind:'cabinet'})`                                                            | `api.volumes.create`                              |
| `/volumes/{id}`                | `api.volumes.show(id)`, `api.places.list({kind:'cabinet'})`, `api.folders.list({nhsNumber})`   | `rename` / `addFolder` / `removeFolder` / `move`  |
