# Domain model

> Part of the [Case Tracking specification](index.md). This is the
> shared, edition-independent domain. The
> [loco domain-model](../case-folder-service-with-rust/spec/domain-model.md)
> adds the wire/`Thing`/`Event` packing detail; the
> [svelte domain-model](../case-folder-front-end-with-svelte/spec/domain-model.md)
> adds the camelCase client types.

## The tracker owns nothing

Every domain entity lives in one of **five external HTTP services**.
The tracker is a pure aggregator: it proxies these services and
snapshots labels onto the records it writes so the audit trail survives
upstream renames, deletes, or outages.

| Service                  | Owns                                                       |
| ------------------------ | ---------------------------------------------------------- |
| **Main Patient Service** | Patient records keyed by NHS Number                        |
| **Main Place Service**   | Buildings → rooms → cabinets (parent chain)                |
| **Main Worker Service**  | Workforce — clinicians, nurses, porters, administrators    |
| **Main Thing Service**   | Folders (`thing_type = "CaseFile"`)                        |
| **Main Event Service**   | Move audit log (`event_type = "FolderMove"`)               |

## Entities

| Entity        | Owned by         | Key fields (conceptual)                                                        |
| ------------- | ---------------- | ------------------------------------------------------------------------------ |
| **Patient**   | Patient Service  | `id`, `nhsNumber`, `name`, `dateOfBirth`                                        |
| **Building**  | Place Service    | `id`, `name`, `description`                                                     |
| **Room**      | Place Service    | `id`, `name`, `buildingId`, `description`                                       |
| **Cabinet**   | Place Service    | `id`, `label`, `roomId`, `capacity`, `description`                              |
| **Worker**    | Worker Service   | `id`, `name`, `role`                                                            |
| **Volume**    | Thing Service    | `id`, `title`, `patientId`, `nhsNumber`, `cabinetId`, `status`, `folderCount`   |
| **Folder**    | Thing Service    | `id`, `title`, `patientId`, `nhsNumber`, `cabinetId`, `status`, `lastMovedAt`, `volumeId` |
| **MoveEvent** | Event Service    | `id`, `folderId`, `from/toCabinet`, `worker`, `movedAt`, `reason` (append-only) |

### Relationships

```
Patient ──< Volume ──> Cabinet ──> Room ──> Building
   │          │
   │          └──< Folder ──> Cabinet
   └──────────────< Folder ──> Cabinet
                     │
                     └──< MoveEvent (append-only) ──> Worker
```

A patient has many folders; a folder lives in at most one cabinet; a
cabinet sits in a room; a room in a building. Each folder accrues an
ordered, append-only sequence of move events.

A **volume** is a *movable bundle of a single patient's folders* — the
classic multi-volume paper case file. A folder belongs to **at most one**
volume (`volumeId`, optional). A volume has its own location (a cabinet)
and is moved as a unit: moving a volume relocates every member folder and
records one move event per folder, so the per-folder audit trail stays
complete. Membership is independent of location — a folder keeps its own
`cabinetId`; the volume move is what re-colocates members.

## Invariants

1. **NHS Number uniqueness is owned by the Main Patient Service**, not
   the tracker. The tracker records whatever NHS Number the service
   returned.
2. **A patient may have many folders, each with a distinct title**
   (e.g. "Volume 1", "Cardiology 2023"). `(patientId, title)` is unique.
3. **Move events are append-only.** Patient name, NHS Number, folder
   title, cabinet labels, worker name + role are **snapshotted** into
   the event at record time so the audit trail survives later renames,
   outages, or cabinet moves.
4. **All cross-service references are opaque UUIDs with no referential
   integrity** between services. Deletes upstream do not cascade; the
   only reconciliation is the snapshots.
5. **Folder status is derived** from the latest move event:
   a destination cabinet → `in-cabinet`; none → `in-transit`. With no
   move history, fall back to the folder's current cabinet pointer.
6. **Snapshots are written at action time** from whatever the upstream
   authoritatively returned, and are **never refreshed automatically**.
7. **A volume and all its member folders belong to the same patient.** A
   folder can be assigned to a volume only if they share `patientId`.
8. **Moving a volume is the only operation that moves a group of folders
   at once.** It updates the volume's cabinet and each member folder's
   cabinet, and appends one move event per member folder.

## Status vocabulary

A folder is exactly one of:

- `in-cabinet` — parked in a known cabinet.
- `in-transit` — moved out with no destination cabinet recorded yet.

There is no `checked-out` or `archived` status; those are not modelled.
