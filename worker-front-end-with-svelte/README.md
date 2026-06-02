# worker-front-end-with-svelte

SvelteKit front-end for the **[Worker Service](../worker-service-rust-crate/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/workers` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/workers/new` | Create worker; surfaces 409 duplicate candidates |
| `/workers/[id]` | Detail view — identity, identifiers, addresses, telecom, emergency contacts |
| `/workers/[id]/edit` | Edit |
| `/workers/[id]/audit` | Per-worker audit log |
| `/workers/match` | Match check — score a hypothetical record against the index |
| `/workers/merge` | Merge two workers (main + duplicate) |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Worker Service — see [`../worker-service-rust-crate/README.md`](../worker-service-rust-crate/README.md). Default: `http://localhost:8080`.

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
| `PUBLIC_API_BASE_URL` | `http://localhost:8080` | Worker Service REST base URL |

Set in `.env`. Because the variable is prefixed with `PUBLIC_`, SvelteKit exposes it to the client bundle.

## Testing

```bash
pnpm test         # vitest unit tests (no live service required)
pnpm test:e2e     # playwright smoke tests (no live service required)
pnpm check        # svelte-check type-check
```

The unit tests mock `fetch`. The Playwright suite asserts the page shells render even when the API is down (the page shows a banner; layout still mounts).

## Project layout

```
src/
  app.html
  app.css                  - shared CSS variables + utility classes
  app.d.ts
  lib/
    config.ts              - PUBLIC_API_BASE_URL
    api/
      types.ts             - Worker, HumanName, MatchResult, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      workers.ts           - WorkerRepository (CRUD + search + match + merge + audit)
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      WorkerGrid.svelte    - SVAR DataGrid binding
      HumanNameInput.svelte
      WorkerForm.svelte
      MatchResultsList.svelte
  routes/
    +layout.svelte         - sidebar nav
    +page.svelte           - dashboard
    workers/
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
    workers.test.ts        - WorkerRepository wrapping tests
  e2e/
    workers.spec.ts        - smoke tests
```

## Lily Design System

The Lily file: dependency resolves to `~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-headless`. Components import via deep path:

```svelte
import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";
```

See the commented example in `src/routes/+layout.svelte`. The MVP currently uses styled native HTML controls; swap in Lily primitives as the design system stabilises.

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP scaffold. See [`spec.md`](spec.md) for the canonical work queue (§13 Tasks).
