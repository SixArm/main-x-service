# person-front-end-with-svelte

SvelteKit front-end for the **[Person Service](../person-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/persons` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/persons/new` | Create person; surfaces 409 duplicate candidates |
| `/persons/[id]` | Detail view — identity, identifiers, addresses, telecom, emergency contacts, cross-service links panel |
| `/persons/[id]/edit` | Edit |
| `/persons/[id]/audit` | Per-person audit log |
| `/persons/match` | Match check — score a hypothetical record against the index |
| `/persons/merge` | Merge two persons (main + duplicate); accepts `?main=`/`?duplicate=` to pre-fill both ids |
| `/persons/bulk` | Bulk import/export — upload JSONL/CSV with a dry-run toggle, submit a filtered export, poll jobs to completion |
| `/review` | Stored duplicate-review board — SVAR Kanban (drag-to-decide) + a keyboard-reachable queue table + inline comparison panel |
| `/expiry` | Identity-document expiry calendar — SVAR Calendar |
| `/signin` | Per-app magic-link sign-in (BFF auth page) |
| `/verify` | Magic-link verification (BFF auth page) |

The persistent top navigation bar (every route; collapses behind a hamburger toggle on narrow viewports — FR-13, no left sidebar) also carries a Lily **theme switcher** and **locale switcher** (FR-11 / FR-12); selections persist to `localStorage`.

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System** (all consumed via `file:` dependencies):
  - `lily-design-system-svelte-headless` — accessibility primitives
  - `lily-design-system-svelte-theme-picker` — `ThemePicker` (live in the layout shell)
  - `lily-design-system-svelte-locale-picker` — `LocalePicker` (live in the layout shell)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Person Service — see [`../person-service-with-loco/README.md`](../person-service-with-loco/README.md). Dev default (`loco start`, and this app's own fallback): `http://localhost:5150`. The podman-compose container maps `:8080` instead — see "Configuration" below and the integration-test section.

## Quick start

```bash
cp .env.example .env
pnpm install
pnpm dev
```

Open <http://localhost:5173>.

## Configuration

The browser calls the same-origin BFF proxy at `/api/proxy` — there is no public API env var (see `src/lib/config.ts`). The server-side BFF reads:

| Variable | Default | Purpose |
| --- | --- | --- |
| `PERSON_API_URL` | `http://localhost:5150` | Person Service base URL — the proxy injects a server-exchanged PASETO and forwards |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link login + session→PASETO exchange |

Set in `.env`. Both are read server-side in `src/lib/server/config.ts` and are never exposed to the client bundle.

## Testing

```bash
pnpm test              # vitest unit tests (no live service required)
pnpm test:e2e          # playwright smoke tests (no live service required)
pnpm test:integration  # playwright integration tests — requires live service
pnpm check             # svelte-check type-check
```

The unit tests mock `fetch`. The smoke suite (`tests/e2e/`) asserts the page shells render even when the API is down (the page shows a banner; layout still mounts).

### Integration tests (live Person Service)

`tests/integration/golden-paths.spec.ts` drives the live SvelteKit preview against a running `person-service-with-loco` over real HTTP. Coverage:

| Test | Spec FR | What it asserts |
| --- | --- | --- |
| `list renders the seeded person` | FR-1 | Search box → SVAR Grid shows the seeded row. |
| `create lands on detail page` | FR-3 | Form POST → 200 → redirect to detail; verified via direct REST GET. |
| `second create surfaces match candidates` | FR-3 | Duplicate POST → 409 → MatchResultsList renders inline. |
| `detail page shows nested fields` | FR-5 | ID, birth date, gender visible. |
| `edit persists` | FR-6 | PUT → detail re-fetches updated birth date. |
| `soft-delete hides record` | FR-7 | DELETE → record either gone or `active: false`. |
| `match check renders breakdown` | FR-8 | POST `/match` → MatchResultsList header + at least one row. |
| `merge soft-deletes duplicate` | FR-9 | POST `/merge` → main survives, duplicate `active: false` or 404. |
| `audit log lists entries` | — | `/audit` route renders entries or the empty-state. |

Each test creates its own records with a timestamped family name and cleans up via REST `DELETE` in `afterAll`, so the suite is safely re-runnable.

#### Running locally

```bash
# 1. Start the Rust service (Postgres + Axum) in the background
(cd ../person-service-with-loco && podman compose up -d)

# 2. Wait for the service to report healthy (first Rust build can take ~5 min)
curl -sf http://localhost:8080/api/health && echo ok

# 3. Run the integration suite — health-checks then runs Playwright
bin/e2e

# Forward Playwright flags as usual:
bin/e2e --headed
bin/e2e --ui
bin/e2e tests/integration/golden-paths.spec.ts -g "FR-9"

# 4. Tear down when done
(cd ../person-service-with-loco && podman compose down)
```

To target a different service URL:

```bash
PUBLIC_API_BASE_URL=http://staging.example:8080 bin/e2e
```

## Project layout

```
src/
  app.html
  app.css                  - shared CSS variables + utility classes
  app.d.ts
  hooks.server.ts          - BFF: reads the session cookie into locals
  lib/
    config.ts              - same-origin BFF proxy base (/api/proxy)
    i18n.svelte.ts          - 13-locale string catalog + reactive locale store
    links.ts                - cross-service link kind<->target-type rules
    bulk.ts                 - bulk import/export pure rules (terminal states, dry-run token, …)
    review.ts               - review-queue status vocabulary + score-breakdown helpers
    api/
      types.ts             - Person, HumanName, MatchResult, EntityLink, BulkJobView, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch; FormData pass-through)
      persons.ts           - PersonRepository (CRUD + search + match + merge + audit + links + bulk + review-queue)
    server/                 - BFF-only, never bundled to the browser
      config.ts             - PERSON_API_URL / AUTH_API_URL
      session.ts             - __Host-mxi_session cookie helpers
      auth.ts                 - magic-link request/verify + session->PASETO exchange
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      PersonGrid.svelte    - SVAR DataGrid binding
      HumanNameInput.svelte
      PersonForm.svelte
      MatchResultsList.svelte
      LinksPanel.svelte    - cross-service links panel (detail page)
  routes/
    +layout.svelte         - top nav bar + hamburger + Lily theme/locale pickers
    +page.svelte           - dashboard
    +page.server.ts        - sign-out action
    signin/, verify/        - BFF magic-link login
    api/proxy/[...path]/    - BFF reverse proxy (injects the PASETO)
    persons/
      +page.svelte         - list
      new/+page.svelte
      match/+page.svelte
      merge/+page.svelte
      bulk/+page.svelte     - bulk import/export
      [id]/
        +page.svelte       - detail (incl. links panel)
        edit/+page.svelte
        audit/+page.svelte
    review/+page.svelte     - duplicate review-queue board
    expiry/+page.svelte     - identity-document expiry calendar
tests/
  unit/                     - client, persons, bulk, links-validation, review, i18n, layout (7 files, 69 tests)
  e2e/                      - route-stubbed Playwright smoke tests
  integration/              - golden-paths.spec.ts against a live service
```

## Lily Design System

Three Lily packages are consumed via `file:` dependencies (see `package.json`):

| Package | Used for | Status |
| --- | --- | --- |
| `lily-design-system-svelte-theme-picker` | `ThemePicker` in the layout sidebar (theme switcher, FR-11) | **Live** |
| `lily-design-system-svelte-locale-picker` | `LocalePicker` in the layout sidebar (locale switcher, FR-12) | **Live** |
| `lily-design-system-svelte-headless` | accessibility primitives (focus trap, listbox, combobox, dialog) | Headless package wired; richer primitives (Dialog/Combobox/Banner) tracked in spec §13 T-14 |

`src/routes/+layout.svelte` imports and renders the `ThemePicker` and
`LocalePicker`; the theme selection persists to `localStorage` under
`lily-theme` (the picker's own `storageKey`), while the locale selection
persists under `mxi.person.locale`, owned by the app's own
`src/lib/i18n.svelte.ts` store rather than the picker. Forms still use styled native HTML controls;
deeper headless primitives are swapped in as the design system stabilises.

```svelte
import ThemePicker from "lily-design-system-svelte-theme-picker/ThemePicker.svelte";
import LocalePicker from "lily-design-system-svelte-locale-picker/LocalePicker.svelte";
```

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP scaffold. See [`spec.md`](spec/index.md) for the canonical work queue (§13 Tasks).
