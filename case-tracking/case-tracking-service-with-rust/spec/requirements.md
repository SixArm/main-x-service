# Requirements (Loco edition)

> Part of the [Loco edition specification](index.md). These implement
> the product requirements in [root requirements](../../spec/requirements.md)
> (FR-/NFR- IDs referenced below). This file states the **API-level**
> requirements and acceptance criteria.

## API requirements

| ID    | Requirement                                                                                       | Root trace      |
| ----- | ------------------------------------------------------------------------------------------------- | --------------- |
| AR-1  | Expose a JSON API for folders, moves, places, patients, workers, stats, and a `/healthz` probe.   | FR-1..FR-9      |
| AR-2  | `POST /api/moves` records a move: snapshot worker, write Event audit log + update Thing cabinet.   | FR-1, NFR-2     |
| AR-3  | `POST /api/folders` finds-or-creates the patient upstream and creates the folder + initial event.  | FR-2            |
| AR-4  | All list endpoints use the `{ items, query? }` envelope; single resources are unwrapped.           | NFR-6           |
| AR-5  | Validate NHS Numbers (Modulus 11) and required fields server-side; reject with `422` field errors. | NFR-1           |
| AR-6  | `GET /api/stats` and `GET /api/patients/{nhs}` soft-fail to partial/snapshot data on upstream loss. | NFR-3           |
| AR-7  | All other endpoints hard-fail with `503` when an upstream is unreachable.                          | NFR-3           |
| AR-8  | Run fully against in-process stub upstreams via `USE_UPSTREAM_STUBS=1`.                            | NFR-7           |
| AR-9  | Own no local domain tables; proxy the five Main-X-Services; snapshot labels on write.              | NFR-4, NFR-2    |
| AR-10 | No HTML, templates, CSS, or client JS — JSON only.                                                | (scope)         |

## Acceptance criteria

- **AC-AR2:** `POST /api/moves` with a `worker_id` returns `201` +
  `Location` and the resulting `MoveEvent` carries snapshotted worker
  name + role; a blank `worker_id` with `moved_by` text snapshots the
  free text; both blank → `"Unknown porter"`.
- **AC-AR3:** `POST /api/folders` for a new NHS Number creates the
  patient upstream and returns `201`; for an existing patient,
  `patient_name`/`date_of_birth` may be omitted.
- **AC-AR5:** an invalid NHS Number returns
  `422 { "errors": { "nhs_number": "…" } }`.
- **AC-AR6:** with the Main Patient Service down, `GET /api/patients/{nhs}`
  returns `200`, `patient: null`, `patient_service_match: false`, and the
  folders derived from Thing snapshots.
- **AC-AR7:** with the Main Thing Service down, `GET /api/folders`
  returns `503 { "error": "Main Thing Service unreachable: …" }`.
- **AC-AR8:** the request-test suite passes with no real upstreams.

Traceability to design lives in [design.md](design.md); delivery status
in [tasks.md](tasks.md).
