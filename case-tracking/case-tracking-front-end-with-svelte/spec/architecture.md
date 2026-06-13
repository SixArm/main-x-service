# Architecture

> Part of the [Svelte edition specification](index.md). Project-level
> view of how the two editions fit: [root architecture](../../spec/architecture.md).

## Layered diagram

```
┌──────────────────────────────────────────────────────────────┐
│  src/routes/**                                                 │
│   +page.ts  → calls api.* and seeds the cache                  │
│   +page.svelte → consumes cache + load `data` prop             │
├──────────────────────────────────────────────────────────────┤
│  src/lib/store/cache.svelte.ts                                 │
│   Rune-reactive arrays + setters used by load functions        │
│   + addFolder / recordMove / addBuilding / addRoom / addCabinet │
├──────────────────────────────────────────────────────────────┤
│  src/lib/api/client.ts                                         │
│   Typed fetch wrappers; snake_case → camelCase mapping         │
├──────────────────────────────────────────────────────────────┤
│  HTTP/JSON (CORS-friendly, same-origin in prod recommended)    │
│                                                                │
│                       ▼                                        │
│  Loco JSON API on http://localhost:5150  — see                 │
│  ../../case-tracker-service-with-rust/spec/index.md                   │
└──────────────────────────────────────────────────────────────┘
```

## Wiring pattern

Every route follows the **load + cache** hybrid:

1. `+page.ts` `load({ fetch, params, url })` calls one or more `api.*`
   methods (passing through SvelteKit's `fetch`), hydrates the reactive
   cache, and returns any per-request data not best held globally (e.g.
   a single folder).
2. `+page.svelte` reads from `cache.x` getters (reactive) and/or from
   `data` (the load return value). It runs no fetches itself.
3. **Mutations** call `cache.x()` methods — these round-trip through the
   API client and splice the new record into the cache on success. After
   certain creates the page calls `invalidateAll()` to refresh siblings.

## Error policy

- `+page.ts` calls `error(status, message)` from `@sveltejs/kit` on API
  failure. SvelteKit renders `src/routes/+error.svelte`.
- 404 from the API → `error(404, 'Folder not found')` (or similar).
- All other failures → `error(503, message)`.
- Inside mutation handlers (form submits), errors are caught locally and
  rendered as field-level errors (`422` body's `errors.{field}`) or a
  page-level Alert.

## File layout

```
case-tracker-front-end-with-svelte/
├── AGENTS.md
├── README.md
├── index.md
├── spec/                          ← this specification
├── package.json
├── pnpm-workspace.yaml
├── svelte.config.js               ← `kit.alias` → Lily helpers (sibling repo)
├── tsconfig.json
├── vite.config.ts                 ← `server.fs.allow` for the sibling repo path
├── static/
│   └── themes/
│       ├── nhs.css                ← `:root[data-theme="nhs"]` colour tokens
│       └── nhs-high-contrast.css  ← `:root[data-theme="nhs-high-contrast"]`
└── src/
    ├── app.d.ts
    ├── app.html
    ├── lib/
    │   ├── api/
    │   │   └── client.ts          ← typed fetch + snake/camel
    │   ├── components/            ← Lily headless + FolderGrid wrapper
    │   ├── css/{nhs.css, app.css}
    │   └── store/
    │       ├── cache.svelte.ts    ← rune-reactive cache + mutations
    │       ├── nhs.ts             ← Modulus 11 + formatter
    │       └── types.ts           ← API-shaped TS types
    └── routes/
        ├── +error.svelte
        ├── +layout.svelte
        ├── +layout.ts             ← ssr = false
        ├── +page.{ts,svelte}      ← dashboard
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
        ├── move/+page.{ts,svelte}
        └── history/+page.{ts,svelte}
```
