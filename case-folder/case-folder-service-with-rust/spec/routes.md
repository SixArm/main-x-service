# Use cases → routes

> Part of the [Loco edition specification](index.md). The use-case
> catalogue (UC-1..UC-9 + negative cases) is shared and described in the
> [root requirements](../../spec/requirements.md) and the
> [Svelte routes file](../../case-folder-front-end-with-svelte/spec/routes.md)
> (trigger / pre-conditions / steps / post-conditions per UC).

## API mapping

| UC   | Method | Path                               | Controller                | Patient                             | Place                                  | Worker           | Thing                                          | Event                          |
| ---- | ------ | ---------------------------------- | ------------------------- | ----------------------------------- | -------------------------------------- | ---------------- | ---------------------------------------------- | ------------------------------ |
| UC-1 | POST   | `/api/moves`                       | `controllers/moves.rs`    | no                                  | **`label_path`**                       | **`find_by_id`** | **`find_by_id` + `update_cabinet`**            | **`record`**                   |
| UC-2 | POST   | `/api/folders`                     | `controllers/folders.rs`  | **`find_by_nhs_number` + `create`** | **`label_path`**                       | no               | **`create`**                                   | **`record`** (initial event)   |
| UC-3 | GET    | `/api/folders?q=`                  | `controllers/folders.rs`  | no                                  | no                                     | no               | **`search(q)`**                                | `list_all` (status/last-moved) |
| UC-4 | GET    | `/api/moves?q=`                    | `controllers/moves.rs`    | no                                  | no                                     | no               | no                                             | **`list_all` + local filter**  |
| UC-5 | GET    | `/api/stats`                       | `controllers/stats.rs`    | no                                  | `search` for cabinet counts            | no               | **`search("")` for folder counts**             | **`list_all`**                 |
| UC-6 | GET    | `/api/places` + `POST /api/places` | `controllers/places.rs`   | no                                  | **`search` + `find_by_id` + `create`** | no               | `search("")` for cabinet folder counts         | no                             |
| UC-8 | GET    | `/api/patients[/{nhs}]`            | `controllers/patients.rs` | **`find_by_nhs_number`**            | no                                     | no               | **`list_for_patient` / `list_for_nhs_number`** | **`list_for_patient`**         |
| UC-9 | GET    | `/api/workers[?q=]`                | `controllers/workers.rs`  | no                                  | no                                     | **`search(q)`**  | no                                             | no                             |
| UC-W1 | GET   | `/api/workers/{id}`                | `controllers/workers.rs`  | no                                  | no                                     | **`find_by_id`** | **`search("")`** (folders)                     | **`list_all`** (filter worker) |
| UC-P1 | GET   | `/api/places/{id}/history`         | `controllers/places.rs`   | no                                  | **`find_by_id` + descendants**         | no               | no                                             | **`list_all`** (cabinet pairing) |
| UC-E1 | GET   | `/api/moves/{id}`                  | `controllers/moves.rs`    | no                                  | no                                     | no               | no                                             | **`list_all`** (find by id)    |
| UC-V1 | POST  | `/api/volumes`                     | `controllers/volumes.rs`  | **`find_by_nhs_number`**            | **`label_path`**                       | no               | **`create_volume`**                            | no                             |
| UC-V2 | POST/DELETE | `/api/volumes/{id}/folders[/{fid}]` | `controllers/volumes.rs` | no                          | no                                     | no               | **`find/set_folder_volume`**                   | no                             |
| UC-V3 | GET   | `/api/volumes/{id}`                | `controllers/volumes.rs`  | no                                  | no                                     | no               | **`find_volume_by_id` + member folders**       | **`list_for_patient`** (history) |
| UC-V4 | POST  | `/api/volumes/{id}/move`           | `controllers/volumes.rs`  | no                                  | **`label_path`**                       | **`find_by_id`** | **`update_volume_cabinet` + `update_cabinet`/folder** | **`record`** (per member)      |

The "live folder lookup as porter types" behaviour from the Svelte UI is
served by `GET /api/folders?nhs_number=...` — clients are free to
debounce that on the front-end.

There is **no `/api/patients` POST**: registering a patient is a side
effect of `POST /api/folders` (which calls
`main_patient_service::find_or_create`). For ad-hoc patient
registration, hit the Main Patient Service directly.

There is also **no `/api/workers` POST**. Workers are managed in the
Main Worker Service. The tracker just proxies its search endpoint.

## Route table

| Method | Route                       | Controller         | Body | Success                       | Failure                                                |
| ------ | --------------------------- | ------------------ | ---- | ----------------------------- | ------------------------------------------------------ |
| GET    | `/healthz`                  | `healthz::index`   | —    | `200 {"status":"ok"}`         | —                                                      |
| GET    | `/api/stats`                | `stats::index`     | —    | `200 Stats`                   | upstream warnings logged, zeros returned               |
| GET    | `/api/folders`              | `folders::index`   | —    | `200 List<Folder>`            | `503` if Thing service is down                         |
| POST   | `/api/folders`              | `folders::create`  | JSON | `201` + `Location` + `Folder` | `422` validation, `503` upstream                       |
| GET    | `/api/folders/{id}`         | `folders::show`    | —    | `200 Folder`                  | `404 {"error":"Folder not found"}`, `503` upstream     |
| GET    | `/api/folders/{id}/history` | `folders::history` | —    | `200 List<Move>`              | `503` Event service                                    |
| GET    | `/api/patients`             | `patients::index`  | —    | `200 List<Patient>`           | `503` Patient service                                  |
| GET    | `/api/patients/{nhs}`       | `patients::show`   | —    | `200 PatientShow`             | (always 200 — falls back to snapshots)                 |
| GET    | `/api/places`               | `places::index`    | —    | `200 PlacesIndex`             | `503` Place service                                    |
| POST   | `/api/places`               | `places::create`   | JSON | `201` + `Location` + `Place`  | `422` validation                                       |
| GET    | `/api/places/{id}`          | `places::show`     | —    | `200 PlaceShow`               | `404`, `503` Place service                             |
| GET    | `/api/workers`              | `workers::index`   | —    | `200 List<Worker>`            | `503` Worker service                                   |
| GET    | `/api/workers/{id}`         | `workers::show`    | —    | `200 WorkerShow`              | `404`, `503` Worker/Event/Thing service                |
| GET    | `/api/places/{id}/history`  | `places::history`  | —    | `200 PlaceHistory`            | `404`, `503` Place/Event service                       |
| GET    | `/api/moves`                | `moves::index`     | —    | `200 List<Move>`              | `503` Event service                                    |
| GET    | `/api/moves/{id}`           | `moves::show`      | —    | `200 Move`                    | `404`, `503` Event service                             |
| GET    | `/api/volumes`              | `volumes::index`   | —    | `200 List<Volume>`            | `503` Thing service                                    |
| POST   | `/api/volumes`              | `volumes::create`  | JSON | `201` + `Location` + `Volume` | `422` (unknown patient / bad title), `503` upstream    |
| GET    | `/api/volumes/{id}`         | `volumes::show`    | —    | `200 VolumeShow`              | `404`, `503` upstream                                  |
| PATCH  | `/api/volumes/{id}`         | `volumes::rename`  | JSON | `200 Volume`                  | `404`, `422` empty title                               |
| POST   | `/api/volumes/{id}/folders` | `volumes::add_folder` | JSON | `200 VolumeShow`           | `404`, `422` different patient                         |
| DELETE | `/api/volumes/{id}/folders/{fid}` | `volumes::remove_folder` | — | `200 VolumeShow`        | `404`                                                  |
| POST   | `/api/volumes/{id}/move`    | `volumes::move_volume` | JSON | `200 VolumeShow`          | `404`, `422` bad cabinet, `503` upstream               |
| POST   | `/api/moves`                | `moves::create`    | JSON | `201` + `Location` + `Move`   | `404` unknown folder, `422` validation, `503` upstream |
| GET    | `/api/alerts`               | `alerts::index`    | —    | `200 List<Alert>`             | `503` Place/Event service                              |

`{nhs}` accepts either `9434765919` or `943 476 5919` (URL-encoded). The
handler normalises it via `nhs::format_nhs_number`.

Routes go under `/api` with **no version segment** — API versioning
happens via the `Accept` mediatype. `/healthz` at the root is the only
exception.
