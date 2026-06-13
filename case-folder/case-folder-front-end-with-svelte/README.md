# Case Tracking — Svelte front-end

A SvelteKit application for tracking **physical paper case-note folders**
as they move between **physical file cabinets** in a UK NHS hospital
setting. The Svelte app is a **client** of the
[Loco JSON API sibling](../case-folder-service-with-rust) — it owns no
data; every list, form, and audit page round-trips through `/api/*`
on the back-end.

> ⚠️ Demo application. Not a regulated medical record. Do not use with
> real patient data.

## The problem

Alice is a medical records porter. She handles a paper file folder of
case notes for a patient and stores it in one of several file
cabinets. When she moves the folder from one cabinet to another, she
needs to record the move so anyone looking for the folder can find it.

Each patient is identified by their
[UK NHS Number](https://en.wikipedia.org/wiki/NHS_number) — a 10-digit
identifier with a Modulus 11 check digit, formatted `XXX XXX XXXX`.
**One patient can have many folders** (e.g. "Volume 1",
"Cardiology 2023", "Maternity"). The physical hierarchy is
**building → room → cabinet** — each cabinet sits inside exactly one
room.

## What this app does

- **Dashboard** — KPIs from `GET /api/stats`, a folder grid, recent
  moves, and cabinet utilisation.
- **Patient register** (`/patients`) — list patients with folder counts.
- **Patient detail** (`/patients/{nhs}`) — patient + folders + move
  history; falls back to local snapshots when the Main Patient Service
  has no record (the API tells us via `patient_service_match`).
- **Folder register** (`/folders`) — searchable list. Search round-trips
  to `GET /api/folders?q=...`.
- **Add folder** (`/folders/new`) — `POST /api/folders`. The API creates
  the patient if their NHS Number is unknown.
- **Folder detail** (`/folders/{id}`) — folder + per-folder history.
- **Building register** (`/buildings`), **building detail**
  (`/buildings/{id}`), **add building** (`/buildings/new`) —
  `GET/POST /api/places` with `kind=building`. Adding a room from the
  detail page posts `kind=room` + `contained_in_place=<building-id>`.
- **Cabinet register** (`/cabinets`), **add cabinet** (`/cabinets/new`)
  — `GET/POST /api/places` with `kind=cabinet`.
- **Move folder** (`/move`) — live NHS-Number lookup against
  `GET /api/folders?nhs_number=...`, worker picker from
  `GET /api/workers`, cabinet picker from `GET /api/places?kind=cabinet`,
  `POST /api/moves` on submit.
- **Audit history** (`/history`) — searchable global log from
  `GET /api/moves?q=...`.
- **Volumes** (`/volumes`, `/volumes/{id}`, `/volumes/new`) — movable
  bundles of a patient's folders: create, assign/remove folders, and
  move a whole volume at once via `GET/POST/PATCH /api/volumes...`.
- **Workers** (`/workers`, `/workers/{id}`) — workforce register from
  `GET /api/workers`, with per-worker move attributions.
- **Scan** (`/scan`) — find a folder by NHS Number and jump to a move.
- **Reports** (`/reports`) and **Alerts** (`/alerts`) — KPIs / cabinet
  utilisation and cross-building geofence alerts (`GET /api/alerts`).
- **Sign in** (`/login`, `/auth/callback`) — magic-link auth UI against
  `POST /api/auth/request` + `/api/auth/verify`; the session is a
  first-party HttpOnly cookie (the dev server proxies `/api` so it is
  same-origin).

## Stack

- [Svelte 5](https://svelte.dev) + [SvelteKit 2](https://svelte.dev/docs/kit)
  with runes (`$state`, `$derived`, `$effect`, `$props`).
- Client-side only (`ssr = false` in `src/routes/+layout.ts`) — the API
  lives on a different origin during dev.
- [SVAR Svelte](https://svar.dev/svelte/) — `wx-svelte-grid` (in the
  Willow theme) for the dashboard data grid.
- [Lily Design System (Svelte headless)](https://lilydesignsystem.io)
  for accessible UI primitives, styled with NHS UK design tokens.
- Lily Svelte helpers (`~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers`,
  cloned alongside this repo) — `lily-design-system-svelte-locale-picker` and
  `lily-design-system-svelte-theme-picker`, consumed in-source via
  SvelteKit `kit.alias` (no copying, no npm publish).

## Prerequisites

The Loco JSON API must be running. The easiest path for local dev and
the Playwright e2e suite is the API's **stub mode**, which boots a
fully populated API with no need to stand up the five upstream
Main-X-Services:

```bash
cd ../case-folder-service-with-rust
USE_UPSTREAM_STUBS=1 cargo run -- start    # listens on http://localhost:5150
```

For "real" mode talking to the upstream services, run `cargo run --
task seed && cargo run -- start` instead. See
[`../case-folder-service-with-rust/README.md`](../case-folder-service-with-rust/README.md)
for the API surface and `curl` examples.

## Run it

```bash
npm install
npm run dev                  # http://localhost:5173
npm run build
npm run preview
npm run check                # svelte-check
npm run test:e2e             # Playwright e2e suite (requires API running — see Prerequisites)
npm run test:e2e:ui          # interactive Playwright UI
npm run test:e2e:headed      # headed Chromium for debugging
```

Override the API URL with `VITE_API_BASE_URL`:

```bash
VITE_API_BASE_URL=http://localhost:5150 npm run dev
```

If the API is unreachable, the page renders the error route at
`src/routes/+error.svelte` with the failure message. There is **no
seed-data fallback** — the seed lives in the Loco subproject's
`cargo run -- task seed` task.

## Theming & locale

A utility row above the header carries two Lily helpers:

- **Theme** — `nhs` (default) and `nhs-high-contrast`. Selecting one
  swaps a managed `<link>` in `<head>` to load
  `/themes/<slug>.css` and sets `<html data-theme="<slug>">`.
  Colour tokens live in `static/themes/*.css`, scoped to
  `:root[data-theme="…"]`. Theme-invariant tokens (typography,
  spacing, layout) stay in `src/lib/css/nhs.css`.
- **Language** — `en`, `cy`, `gd`. Selecting one sets `<html lang>`
  (and `<html dir>` if an RTL locale is added). No UI-string
  translation today; the value influences assistive-tech voice
  selection.

Both pickers persist via `localStorage` (`case-folder:theme`,
`case-folder:locale`).

The helpers are consumed from the sibling git path
`~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/`
via two SvelteKit `kit.alias` entries (`@lily/theme-picker`,
`@lily/locale-picker`) and a Vite `server.fs.allow` entry that lets
Vite serve files from outside the project root. **The sibling repo
must be cloned next to this one** (under `~/git/lilydesignsystem/…`)
for `npm run dev` and `npm run build` to resolve the imports.

## Layout

```
playwright.config.ts             # Playwright e2e config (auto-boots dev server)
tests/e2e/
├── global-setup.ts              # pings /healthz + verifies seed state
├── helpers/{nhs,seed,forms,unique}.ts
└── *.spec.ts                    # smoke, dashboard, folders, patients,
                                 #   places, move, history, errors, volumes,
                                 #   workers (clickthrough), auth, a11y,
                                 #   ifit, wiring
static/
└── themes/
    ├── nhs.css                  # :root[data-theme="nhs"] colour tokens
    └── nhs-high-contrast.css    # :root[data-theme="nhs-high-contrast"]
src/
├── app.html
├── app.d.ts
├── lib/
│   ├── api/
│   │   └── client.ts                # Typed fetch client; snake → camel mapping
│   ├── components/                  # Lily headless components + FolderGrid (SVAR wrapper)
│   ├── css/
│   │   ├── nhs.css                  # NHS components + theme-invariant tokens
│   │   └── app.css                  # App overrides + utility-row layout
│   └── store/
│       ├── cache.svelte.ts          # rune-reactive cache + mutation helpers
│       ├── nhs.ts                   # NHS Number formatting + Modulus 11
│       └── types.ts                 # API-shaped TypeScript types
└── routes/
    ├── +layout.svelte
    ├── +layout.ts                   # ssr = false (client-only)
    ├── +error.svelte                # API error page
    ├── +page.{ts,svelte}            # Dashboard
    ├── patients/
    │   ├── +page.{ts,svelte}
    │   └── [nhs]/+page.{ts,svelte}
    ├── folders/
    │   ├── +page.{ts,svelte}
    │   ├── new/+page.{ts,svelte}
    │   └── [id]/+page.{ts,svelte}
    ├── buildings/
    │   ├── +page.{ts,svelte}
    │   ├── new/+page.svelte
    │   └── [id]/+page.{ts,svelte}
    ├── cabinets/
    │   ├── +page.{ts,svelte}
    │   └── new/+page.{ts,svelte}
    ├── rooms/[id]/+page.{ts,svelte}
    ├── volumes/
    │   ├── +page.{ts,svelte}
    │   ├── new/+page.{ts,svelte}
    │   └── [id]/+page.{ts,svelte}
    ├── workers/
    │   ├── +page.{ts,svelte}
    │   └── [id]/+page.{ts,svelte}
    ├── move/+page.{ts,svelte}
    ├── scan/+page.svelte
    ├── reports/+page.{ts,svelte}
    ├── alerts/+page.{ts,svelte}
    ├── login/+page.svelte
    ├── auth/callback/+page.{ts,svelte}
    └── history/+page.{ts,svelte}
```

## Wiring pattern (hybrid load + cache)

Every route follows the same pattern:

1. `+page.ts` `load({ fetch, url, params })` calls one or more methods
   on `api.*` (see `src/lib/api/client.ts`), pushes the results into
   the rune-reactive cache, and returns any per-page data.
2. `+page.svelte` consumes the cache via `cache.x` getters (reactive)
   and/or the load `data` prop.
3. Mutations call `cache.addFolder`, `cache.recordMove`,
   `cache.addBuilding`, etc. Each one round-trips through the API
   client and, on success, updates the cache so subsequent renders
   see the change without a refetch.

On API failure the load function calls SvelteKit's `error()` which
renders `+error.svelte` with the API's error message.

## What was removed

- The previous in-memory store (`tracker.svelte.ts`) with seed data.
- The `/patients/new` route — the API has no `POST /api/patients`; new
  patients are a side effect of `POST /api/folders`.
- `FolderStatus` values `'checked-out'` and `'archived'` — the API only
  emits `'in-cabinet'` and `'in-transit'`.

## See also

- [spec/](spec/index.md) — full specification + wiring contract.
- [AGENTS.md](AGENTS.md) — working agreements for collaborators.
- [index.md](index.md) — documentation landing page.
- [`../case-folder-service-with-rust`](../case-folder-service-with-rust)
  — the JSON API back-end.
