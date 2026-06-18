# person-front-end-with-svelte

SvelteKit front-end for the **[Person Service](../person-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/persons` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/persons/new` | Create person; surfaces 409 duplicate candidates |
| `/persons/[id]` | Detail view — identity, identifiers, addresses, telecom, emergency contacts |
| `/persons/[id]/edit` | Edit |
| `/persons/[id]/audit` | Per-person audit log |
| `/persons/match` | Match check — score a hypothetical record against the index |
| `/persons/merge` | Merge two persons (main + duplicate) |

The persistent layout sidebar (every route) also carries a Lily **theme switcher** and **locale switcher** (FR-11 / FR-12); selections persist to `localStorage`.

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System** (all consumed via `file:` dependencies):
  - `lily-design-system-svelte-headless` — accessibility primitives
  - `lily-design-system-svelte-theme-select` — `ThemeSelect` (live in the layout shell)
  - `lily-design-system-svelte-locale-select` — `LocaleSelect` (live in the layout shell)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Person Service — see [`../person-service-with-loco/README.md`](../person-service-with-loco/README.md). Default: `http://localhost:8080`.

## Quick start

```bash
cp .env.example .env
pnpm install
pnpm dev
```

Open <http://localhost:5173>.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PUBLIC_API_BASE_URL` | `http://localhost:8080` | Person Service REST base URL |

Set in `.env`. Because the variable is prefixed with `PUBLIC_`, SvelteKit exposes it to the client bundle.

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
  lib/
    config.ts              - PUBLIC_API_BASE_URL
    api/
      types.ts             - Person, HumanName, MatchResult, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      persons.ts           - PersonRepository (CRUD + search + match + merge + audit)
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
  routes/
    +layout.svelte         - sidebar nav + Lily theme/locale pickers
    +page.svelte           - dashboard
    persons/
      +page.svelte         - list
      new/+page.svelte
      match/+page.svelte
      merge/+page.svelte
      [id]/
        +page.svelte       - detail
        edit/+page.svelte
        audit/+page.svelte
tests/
  unit/
    client.test.ts         - ApiClient envelope + error tests
    persons.test.ts        - PersonRepository wrapping tests
  e2e/
    persons.spec.ts        - smoke tests
```

## Lily Design System

Three Lily packages are consumed via `file:` dependencies (see `package.json`):

| Package | Used for | Status |
| --- | --- | --- |
| `lily-design-system-svelte-theme-select` | `ThemeSelect` in the layout sidebar (theme switcher, FR-11) | **Live** |
| `lily-design-system-svelte-locale-select` | `LocaleSelect` in the layout sidebar (locale switcher, FR-12) | **Live** |
| `lily-design-system-svelte-headless` | accessibility primitives (focus trap, listbox, combobox, dialog) | Headless package wired; richer primitives (Dialog/Combobox/Banner) tracked in spec §13 T-14 |

`src/routes/+layout.svelte` imports and renders the `ThemeSelect` and
`LocaleSelect`; their selections persist to `localStorage` under
`lily-theme` / `lily-locale`. Forms still use styled native HTML controls;
deeper headless primitives are swapped in as the design system stabilises.

```svelte
import ThemeSelect from "lily-design-system-svelte-theme-select/ThemeSelect.svelte";
import LocaleSelect from "lily-design-system-svelte-locale-select/LocaleSelect.svelte";
```

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP scaffold. See [`spec.md`](spec/index.md) for the canonical work queue (§13 Tasks).
