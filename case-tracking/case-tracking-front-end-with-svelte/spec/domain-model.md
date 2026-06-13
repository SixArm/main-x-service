# Domain model

> Part of the [Svelte edition specification](index.md). The shared
> domain (entities, invariants, status vocabulary):
> [root domain-model](../../spec/domain-model.md). The wire types are
> owned by the API: [Loco domain-model](../../case-tracker-service-with-rust/spec/domain-model.md)
> and [Loco api-contract](../../case-tracker-service-with-rust/spec/api-contract.md).

This subproject mirrors the wire types in
[`src/lib/store/types.ts`](../src/lib/store/types.ts) with **camelCase**
field names; conversion happens in
[`src/lib/api/client.ts`](../src/lib/api/client.ts).

| Type        | Key camelCase fields                                                                                                                                              |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Patient`   | `id`, `nhsNumber`, `name`, `dateOfBirth \| null`, `folderCount`, `source`                                                                                         |
| `Building`  | `id`, `name`, `description`                                                                                                                                       |
| `Room`      | `id`, `name`, `buildingId`, `description` (mapped from `contained_in_place`)                                                                                      |
| `Cabinet`   | `id`, `label` (mapped from `name`), `roomId`, `capacity \| null`, `description`, `folderCount`, `containerPath`                                                   |
| `Folder`    | `id`, `title`, `patientId`, `nhsNumber`, `patientName`, `cabinetId`, `cabinetLabel`, `status: 'in-cabinet' \| 'in-transit'`, `lastMovedAt \| null`, `notes`, `volumeId \| null`, `volumeTitle \| null` |
| `Volume`    | `id`, `title`, `patientId`, `nhsNumber`, `patientName`, `cabinetId`, `cabinetLabel`, `status`, `folderCount`                                                       |
| `MoveEvent` | `id`, `folderId`, `folderTitle`, `patientId`, `nhsNumber`, `patientName`, `fromCabinetId/Label`, `toCabinetId/Label`, `workerId`, `movedBy`, `workerRole`, `movedAt`, `reason` |
| `Worker`    | `id`, `name`, `role`                                                                                                                                              |
| `Stats`     | `patients`, `folders.{total, inCabinet, inTransit}`, `places.{buildings, rooms, cabinets}`, `moves24h`                                                            |

## Invariants (echoed from the API contract)

1. **NHS Number uniqueness is owned by the Main Patient Service.** The
   client does not enforce uniqueness; the API does.
2. **Folder status is `'in-cabinet'` or `'in-transit'`.** No
   `'checked-out'`, no `'archived'` — those are not in the API.
3. **`MoveEvent` is append-only** in the audit log.
4. **All cross-service IDs are opaque UUIDs.** The client stores them as
   strings and does not parse or compare structurally.
5. **`lastMovedAt`, `capacity`, `dateOfBirth`, `notes`, `description`
   are nullable** — every page must handle `null`.
