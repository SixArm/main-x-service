# Glossary

> Part of the [Case Tracking specification](index.md). Shared vocabulary
> for both editions.

| Term              | Meaning                                                                                       |
| ----------------- | --------------------------------------------------------------------------------------------- |
| **Audit log**     | The append-only sequence of `MoveEvent`s held by the Main Event Service, exposed via `/api/moves`. |
| **Cabinet**       | A physical file cabinet that holds folders. A `Place` in the Place Service.                    |
| **Building / Room** | The parent levels of the place hierarchy: building → room → cabinet.                        |
| **Folder**        | A paper case-note file for one patient. A `Thing` with `thing_type = "CaseFile"`.              |
| **Main-X-Service** | The five upstream HTTP services (Patient, Place, Worker, Thing, Event) the tracker proxies.   |
| **MoveEvent**     | An append-only record of a folder changing location, with snapshots of all referenced labels.  |
| **Modulus 11**    | The check-digit algorithm used to validate NHS Numbers. See [nhs-number.md](nhs-number.md).    |
| **NHS Number**    | UK patient identifier; 10 digits, validated by Modulus 11.                                     |
| **Place**         | The unified building/room/cabinet entity owned by the Main Place Service.                      |
| **Porter**        | Hospital staff member who physically transports folders.                                       |
| **SaMD**          | Software as a Medical Device — the regulatory class the project deliberately stays below.      |
| **Snapshot**      | A denormalised copy of an upstream label, written at action time so audit data survives outages. |
| **Stub mode**     | `USE_UPSTREAM_STUBS=1` — in-process fake upstreams seeded with demo data; the shared test harness. |
| **Status**        | A folder is `in-cabinet` or `in-transit`. No other states exist.                               |

Edition-specific terms (Loco, SeaORM, runes, SVAR Grid, etc.) live in
each subproject's `spec/glossary.md`.
