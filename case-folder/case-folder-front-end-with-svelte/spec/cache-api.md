# Cache API

> Part of the [Svelte edition specification](index.md). The cache
> singleton lives in
> [`src/lib/store/cache.svelte.ts`](../src/lib/store/cache.svelte.ts).

```ts
// Reactive getters (read inside $derived):
cache.stats         // Stats | null
cache.folders       // Folder[]
cache.patients      // Patient[]
cache.buildings     // Building[]
cache.rooms         // Room[]
cache.cabinets      // Cabinet[]
cache.workers       // Worker[]
cache.moves         // MoveEvent[]

// Setters (used by +page.ts load functions):
cache.setStats(s)
cache.setFolders(list)
cache.setPatients(list)
cache.setBuildings(list)
cache.setRooms(list)
cache.setCabinets(list)
cache.setWorkers(list)
cache.setMoves(list)
cache.upsertFolder(f)

// Lookups (synchronous; read from cache only):
cache.buildingById(id)
cache.roomById(id)
cache.cabinetById(id)
cache.cabinetLocation(id)   // string, "In transit" when id is null

// Mutations (return promises; throw on API failure):
cache.addFolder({ nhsNumber, patientName?, dateOfBirth?, title, cabinetId?, notes? }): Folder
cache.recordMove({ folderId, toCabinetId, workerId?, movedBy?, reason? }): MoveEvent
cache.addBuilding({ name, description? }): string  // returns new UUID
cache.addRoom({ name, buildingId, description? }): string
cache.addCabinet({ label, roomId, capacity?, description? }): string
```

## Mutation contracts

- `addFolder` — calls `POST /api/folders`. On `422` the caller (form
  submit handler) parses the `errors` body into field-level errors. The
  new folder is upserted into the cache via `upsertFolder`.
- `recordMove` — calls `POST /api/moves`. On success: the move is
  prepended to `cache.moves`; the matching folder's `cabinetId` /
  `cabinetLabel` / `status` / `lastMovedAt` are updated in place so any
  list rendered alongside the move form reflects the change.
- `addBuilding/Room/Cabinet` — calls `POST /api/places` with the
  matching `kind`. The new place is appended to the relevant array.

## What the cache does not do

- **No persistence.** Reload re-fetches from the API.
- **No fetching of its own.** Only the route `+page.ts` loaders trigger
  network calls. The cache is a pure reactive container with three
  classes of writer: setters, upserters, and mutators.
- **No optimistic UI.** Cache updates land only after the API confirms.
