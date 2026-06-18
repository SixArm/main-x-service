# Volume

## What is a volume?

A **volume** is a *movable bundle of a single patient's folders* — the
classic multi-volume paper case file (e.g. "Volume 1", "Volume 2",
"Cardiology 2023"). It lets staff move and track several of one patient's
folders as a single unit while keeping a complete per-folder audit trail.

## Key facts

- **One patient.** A volume and all its member folders share the same
  `patientId` ([domain-model invariant 7](domain-model.md)). A folder can
  be assigned to a volume only if they belong to the same patient.
- **At most one volume per folder.** A folder belongs to **zero or one**
  volume (`volumeId` is optional on the folder).
- **It has its own location.** A volume points at a cabinet (`cabinetId` /
  `cabinetLabel`), independent of each member folder's own `cabinetId`.
- **It moves as a unit.** Moving a volume relocates every member folder and
  appends **one move event per folder**, so the append-only per-folder audit
  trail stays complete. Per [invariant 8](domain-model.md), moving a volume
  is the **only** operation that moves a group of folders at once.
- **Membership is independent of location.** A folder keeps its own
  `cabinetId`; the volume move is what re-colocates members.

## Shape

A `Volume` carries (see [domain-model.md](domain-model.md)):

`id`, `title`, `patientId`, `nhsNumber`, `patientName`, `cabinetId`,
`cabinetLabel`, `status` (`in-cabinet` | `in-transit`), `folderCount`.

## Volume vs folder vs batch

| Concept    | Spans                  | Lifetime               | Moves as a unit?                 |
| ---------- | ---------------------- | ---------------------- | -------------------------------- |
| **Folder** | one paper folder       | persistent             | yes (a single folder)            |
| **Volume** | one **patient's** folders | persistent grouping | yes — the only group move today  |
| **Batch**  | many folders/volumes, **across patients** | transient (one action) | proposed — see [batch.md](batch.md) |

A volume is a *domain* grouping (one patient, persists over time); a
[batch](batch.md) is an *operational* grouping (any patients, exists only
for one bulk action).
