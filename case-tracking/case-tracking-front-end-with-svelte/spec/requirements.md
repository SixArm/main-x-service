# Requirements (Svelte edition)

> Part of the [Svelte edition specification](index.md). These implement
> the product requirements in [root requirements](../../spec/requirements.md)
> (FR-/NFR- IDs referenced below). This file states the **UI-level**
> requirements and acceptance criteria.

## UI requirements

| ID    | Requirement                                                                                       | Root trace   |
| ----- | ------------------------------------------------------------------------------------------------- | ------------ |
| UR-1  | Provide a browser page for every use case UC-1..UC-8 (see [routes.md](routes.md)).                | FR-1..FR-8   |
| UR-2  | Dashboard surfaces KPIs, recent moves, and cabinet utilisation from `/api/stats` + `/api/places`. | FR-5         |
| UR-3  | Move workflow does live NHS lookup, worker pick, cabinet pick, then `POST /api/moves`.            | FR-1         |
| UR-4  | Forms pre-flight NHS Numbers (Modulus 11) and surface API `422` field errors.                     | NFR-1        |
| UR-5  | Patient detail shows the snapshot-fallback warning when `patient_service_match: false`.           | FR-7, NFR-3  |
| UR-6  | All network access goes through `src/lib/api/client.ts`; pages never `fetch` directly.            | NFR-6        |
| UR-7  | Routes follow load+cache; the cache holds no persistence and does no fetching of its own.         | NFR-4        |
| UR-8  | Load failures render `+error.svelte` (404 / 503) with actionable text.                            | NFR-3        |
| UR-9  | Meet the WCAG 2.2 AA baseline in [accessibility.md](accessibility.md).                             | NFR-5        |

## Acceptance criteria

- **AC-UR3:** entering a valid seeded NHS Number on `/move` populates the
  folder pane; choosing a cabinet + worker and submitting records the
  move and updates the folder's status in the visible list.
- **AC-UR4:** submitting a folder/move form with an invalid NHS Number is
  blocked client-side; an API `422` maps `errors.{field}` onto the form.
- **AC-UR5:** visiting `/patients/{unknown-nhs}` shows the snapshot
  warning and the folders derived from snapshots.
- **AC-UR6:** no `+page.svelte`/component calls `fetch` directly (only
  `api.*`).
- **AC-UR8:** with the API stopped, every route renders `+error.svelte`
  with the connection error and how to start the API.
- **AC-UR9:** every page has one `<h1>`, a first-focusable skip link, and
  labelled form fields.

Traceability to design lives in [design.md](design.md); delivery status
in [tasks.md](tasks.md).
