# course-front-end-with-svelte

SvelteKit front-end for the **[Course Service](../course-service-rust-crate/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/courses` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/courses/new` | Create course; surfaces 409 duplicate candidates |
| `/courses/[id]` | Detail view — identity, identifiers, addresses, telecom, emergency contacts |
| `/courses/[id]/edit` | Edit |
| `/courses/[id]/audit` | Per-course audit log |
| `/courses/match` | Match check — score a hypothetical record against the index |
| `/courses/merge` | Merge two courses (main + duplicate) |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Course Service — see [`../course-service-rust-crate/README.md`](../course-service-rust-crate/README.md). Default: `http://localhost:8084` (the family allocates `8080` to person-service).

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
| `PUBLIC_API_BASE_URL` | `http://localhost:8084` | Course Service REST base URL |

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
      types.ts             - Course, HumanName, MatchResult, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      courses.ts           - CourseRepository (CRUD + search + match + merge + audit)
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      CourseGrid.svelte    - SVAR DataGrid binding
      HumanNameInput.svelte
      CourseForm.svelte
      MatchResultsList.svelte
  routes/
    +layout.svelte         - sidebar nav
    +page.svelte           - dashboard
    courses/
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
    courses.test.ts        - CourseRepository wrapping tests
  e2e/
    courses.spec.ts        - smoke tests
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

MVP wired against the now-real Course Service surface. Routes for
list / new / detail / edit / delete / match / merge / audit are
live; instance and syllabus-section edit UI are the remaining
gaps tracked in [`spec.md §13`](spec/13-tasks.md). 9 vitest + 5
Playwright smoke tests.
