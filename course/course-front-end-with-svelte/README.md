# course-front-end-with-svelte

SvelteKit front-end for the **[Course Service](../course-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/courses` | List & search (full-text, fuzzy) with SVAR DataGrid |
| `/courses/new` | Create course; surfaces 409 duplicate candidates |
| `/courses/[id]` | Detail view — identity (course code, status, educational level, credits, time required), identifiers, teaches, keywords, alternate names, same-as links, instances (read-only) |
| `/courses/[id]/edit` | Edit |
| `/courses/[id]/audit` | Per-course audit log |
| `/courses/match` | Match check — score a hypothetical record against the index |
| `/courses/merge` | Merge two courses (main + duplicate) |
| `/board` | Course lifecycle Kanban board |
| `/calendar` | CourseInstance schedule calendar |
| `/signin` | Per-app magic-link sign-in (BFF auth page) |
| `/verify` | Magic-link verification (BFF auth page) |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Course Service — see [`../course-service-with-loco/README.md`](../course-service-with-loco/README.md). Default: `http://localhost:8084` (the family allocates `8080` to person-service).

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
| `COURSE_API_URL` | `http://localhost:5150` | Course Service base URL — the proxy injects a server-exchanged PASETO and forwards |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link login + session→PASETO exchange |

Set in `.env`. Both are read server-side in `src/lib/server/config.ts` and are never exposed to the client bundle.

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
    config.ts              - same-origin BFF proxy base (/api/proxy)
    api/
      types.ts             - Course, CourseIdentifier, MatchResult, … (mirrors the Rust models)
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
      CourseIdentifierInput.svelte
      CourseForm.svelte
      courseFormValidate.ts   - pure validate + normalizeForWire (FR-4; unit-tested)
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
    client.test.ts            - ApiClient envelope + error tests
    courses.test.ts           - CourseRepository wrapping + search params
    form.test.ts              - createForm rune controller
    courseFormValidate.test.ts - FR-4 validation + normalizeForWire
  e2e/
    courses.spec.ts        - smoke tests
```

## Lily Design System

The Lily file: dependency resolves to `~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-headless`. Components import via deep path:

```svelte
import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";
```

The Lily **theme selector** (45 shared themes at `/assets/themes/`) and **locale selector** (13 locales) are wired live in the layout shell — `src/routes/+layout.svelte` imports and renders `ThemeSelect` and `LocaleSelect`. Lily Headless is available for further primitives as the design system stabilises.

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP wired against the now-real Course Service surface. Routes for
list / new / detail / edit / delete / match / merge / audit are
live; instance and syllabus-section edit UI are the remaining
gaps tracked in [`spec.md §13`](spec/13-tasks.md). 27 vitest + 5
Playwright smoke tests.
