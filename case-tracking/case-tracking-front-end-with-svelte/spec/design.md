# Design (Svelte edition)

> Part of the [Svelte edition specification](index.md). System-level
> decisions: [root design](../../spec/design.md). This file records the
> **Svelte-specific** decisions that satisfy [requirements.md](requirements.md).

## SD-1 — Load + cache hybrid (UR-7)

`+page.ts` loaders call `api.*` and hydrate a single rune-reactive cache
singleton; `+page.svelte` reads cache getters and/or the `data` prop and
never fetches. Mutations call `cache.x()` which round-trips the API and
splices results in on success. _Satisfies:_ one place to reason about
data flow; no component-level fetching.

## SD-2 — Single API client, snake↔camel at the edge (UR-6)

[`src/lib/api/client.ts`](../src/lib/api/client.ts) is the only module
that calls `fetch`. It converts snake_case wire fields to camelCase
client types ([domain-model.md](domain-model.md)) and throws `ApiError`
carrying `{ status, body }`. _Satisfies:_ consistent error handling +
trivial mocking.

## SD-3 — CSR-only for now (UR-1)

`ssr = false` in `+layout.ts`. The API and app run on different dev
ports; same-origin SSR is a deployment concern deferred to a production
gate. _Satisfies:_ simplest correct default until same-origin lands.

## SD-4 — Hard-fail loads, field-level mutation errors (UR-4, UR-8)

Loaders call `error(status, message)` → `+error.svelte`. Mutation
handlers catch `ApiError`; on `422` they map `errors.{field}` onto form
state, otherwise show a page-level Alert. _Satisfies:_ no silent
fallback to invented local data.

## SD-5 — No local persistence (UR-7)

No `localStorage`/IndexedDB of domain data; the cache evaporates on
reload (the only persisted values are the locale/theme picker prefs).
_Satisfies:_ the no-PII-beyond-session privacy posture.

## SD-6 — Headless Lily + external token CSS (UR-9)

Lily components are headless; their styles live in `src/lib/css/*` and
per-theme `static/themes/*.css`. Locale/theme pickers are sibling-repo
helpers resolved via `kit.alias`, not vendored. _Satisfies:_ themeable,
accessible UI without forking components.

## Requirement → design trace

| Requirement | Satisfied by |
| ----------- | ------------ |
| UR-1, UR-2  | SD-1, SD-3   |
| UR-3        | SD-1, SD-2   |
| UR-4, UR-8  | SD-4         |
| UR-5        | SD-2, SD-4   |
| UR-6        | SD-2         |
| UR-7        | SD-1, SD-5   |
| UR-9        | SD-6         |
