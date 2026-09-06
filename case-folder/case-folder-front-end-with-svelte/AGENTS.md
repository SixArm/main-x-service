# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **SvelteKit browser client** for the
[Loco JSON API sibling](../case-folder-service-with-rust). The Svelte
app owns no data; every page fetches from `/api/*`. The Loco app must
be running for this app to work.

Full spec is in [`spec/`](spec/index.md). Prefer reading that to
inferring from the code.

## Repository orientation

| Where                                   | What                                                            |
| --------------------------------------- | --------------------------------------------------------------- |
| `src/routes/+layout.{svelte,ts}`        | Header / nav / footer; `ssr = false` for client-only            |
| `src/routes/+error.svelte`              | Renders when a load function calls `error()`                    |
| `src/routes/**/{+page.ts,+page.svelte}` | Route loader + page component, one folder per URL               |
| `src/lib/api/client.ts`                 | The **only** module that calls `fetch` to the API               |
| `src/lib/store/cache.svelte.ts`         | Rune-reactive cache + mutation helpers                          |
| `src/lib/store/types.ts`                | API-shaped TypeScript types (camelCase)                         |
| `src/lib/store/nhs.ts`                  | Modulus 11 + formatter (pre-flight only; API revalidates)       |
| `src/lib/components/`                   | Lily headless primitives + `FolderGrid` SVAR wrapper            |
| `src/lib/css/`                          | `nhs.css` (theme-invariant NHS tokens + components) + `app.css` |
| `static/assets/themes/`                 | Symlink to the shared Lily theme catalogue, swapped at runtime by `ThemePicker` |
| `svelte.config.js`                      | SvelteKit config (Lily helpers are `file:` deps, not aliased)   |
| `vite.config.ts`                        | `server.fs.allow` for the same sibling repo path                |

## Working rules

### 1. The API is the source of truth

If a list, form, or aggregate **could** be derived from the
[Loco API](../case-folder-service-with-rust/spec/routes.md), it **must** be.
Do not invent local data, do not seed, do not cache to `localStorage`.

### 2. All `fetch` goes through `src/lib/api/client.ts`

Pages and components must not call `fetch` directly. Add or extend a
method on `api.*` instead. This keeps snake/camel conversion in one
place, error handling consistent, and makes mocking in tests trivial.

### 3. Routes follow the load + cache pattern

- `+page.ts` `load()` calls `api.*` and pushes results into the
  reactive cache (or returns per-page data via `data`).
- `+page.svelte` reads from `cache.x` getters and/or the `data` prop.
- Mutations call `cache.x()` methods, which round-trip through the
  client and update the cache on success.

See [`spec/architecture.md`](spec/architecture.md) ("Wiring pattern")
for the full pattern, [`spec/examples.md`](spec/examples.md) for code.

### 4. CSR-only for now

`export const ssr = false;` in `src/routes/+layout.ts`. If you find
yourself wanting SSR, raise it first — same-origin deployment is a
prerequisite (§12 of spec).

### 5. Hard-fail on API errors

Load functions call `error(status, message)` from `@sveltejs/kit` on
API failure. Don't silently fall back to local data. The
`+error.svelte` page surfaces the failure with the API URL and a
pointer to the Loco quick-start.

### 6. Field-level error parsing on `422`

Mutations throw `ApiError`. On `status === 422`, the body is
`{ errors: { field: "message" } }` — map the snake_case field names
back to the form's error state. See `src/routes/folders/new/+page.svelte`
for the canonical pattern.

### 7. NHS Number rules

- Always format on display (`formatNhsNumber`).
- Always pre-flight validate before submitting (`isValidNhsNumber`).
- Never compare formatted strings directly — normalise first
  (`normaliseNhsNumber`).
- The Loco API runs the identical Modulus 11 validator; we
  pre-validate for UX, the API is authoritative.

### 8. Lily components are headless

Use [Lily Design System Svelte Headless](https://github.com/lilydesignsystem/lily-design-system-svelte-headless/).
Their styles live in `src/lib/css/{nhs.css,app.css}`. Don't edit a
component to add styles; extend the CSS instead.

### 8a. Lily helpers come from the sibling repo

`lily-design-system-svelte-locale-picker` and
`lily-design-system-svelte-theme-picker` are declared as **`file:`
dependencies** in `package.json` pointing at the sibling repo
(`~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/`),
and imported by their package names. `npm install` symlinks them into
`node_modules`, so resolution is standard (no `kit.alias`). The sibling
repo must exist for `npm install` (and therefore dev/build/check) to
work — install **fails loudly** if it is absent.

- Don't vendor the helpers into `src/lib/`. If a helper needs to
  change, change it upstream and let this app pick it up.
- Don't add a fallback path. If the sibling is missing, the `file:`
  install fails loudly — that's intentional; silent fallbacks hide drift.
- Themes come from the shared Lily catalogue at `static/assets/themes/`
  (a symlink); each defines DaisyUI `--color-*` tokens. `src/lib/css/nhs.css`
  bridges the base `--nhs-*` colour tokens onto the active `--color-*` so
  themes restyle the app. Theme-invariant tokens (spacing, typography,
  layout) stay in `src/lib/css/nhs.css` under `:root`. (The old app-local
  `static/themes/nhs*.css` files were dropped.)

### 9. CI gate

```bash
npm run check
npm run format:check
npm run lint
npm run verify:api
USE_UPSTREAM_STUBS=1 cargo run -- start   # in ../case-folder-service-with-rust
npm run test:e2e
```

All required green:

- `npm run check` — zero errors. Warnings in third-party / pre-existing
  components are tolerated unless they came in with this PR.
- `npm run format:check` (Prettier + `prettier-plugin-svelte`, scoped
  to `src`) — zero style issues. `npm run format` fixes them. Config
  is `.prettierrc` (`tabWidth: 4`, `singleQuote: true`, matching this
  project's existing style — no reformatting churn on adoption).
- `npm run lint` (ESLint) — unchanged; runs alongside Prettier rather
  than in place of it, same as every sibling front-end.
- `npm run verify:api` (ST-19) — regenerates `src/lib/api/schema.d.ts`
  from the sibling crate's `openapi.yaml` (`npm run gen:api`, which
  also runs Prettier on its own output so the comparison isn't
  polluted by formatting noise) and fails on any resulting `git diff`.
  Catches an `openapi.yaml` edit whose generated types were never
  regenerated — a real one is exactly what this task found on
  landing: `gen:api`'s raw output had drifted from the committed file
  in both an actual doc-comment update and, separately, quote style
  (double vs. this project's single-quote convention, since `gen:api`
  never ran Prettier on its own output before this change). `npm run
  gen:api` fixes it locally.
- `npm run test:e2e` — the Playwright suite (14 spec files, 73
  `test()` cases; see [spec/testing.md](spec/testing.md) for the table).
  The Loco API must be running in **stub mode** (see Prerequisites in
  [README.md](README.md)).
  Tests are serialised (`fullyParallel: false`) because they share
  the API's upstream state.

### 10. Adding e2e tests

- Suites live in `tests/e2e/*.spec.ts`. One file per feature area.
- Helpers in `tests/e2e/helpers/`:
  - `nhs.generate()` → fresh Modulus-11-valid number, avoids collisions.
  - `unique(prefix)` → unique titles for `addFolder`/`addPlace`.
  - `fieldControl(page, labelText)` → the actual input inside a Lily Field
    (regular `getByLabel` doesn't work — the Field's auto-generated id
    isn't applied to the child input).
- Don't use `getByLabel` on plain Field children. Use `fieldControl`.
- Use `getByLabel('NHS Number', { exact: true })` for the NHS Number
  input (it carries its own `aria-label`).
- For pages with HTML5 `required` attributes, use whitespace strings to
  reach the JS validation handler (the browser blocks empty strings).
- Tests share state — keep them hermetic by generating fresh NHS
  Numbers + unique titles; never assume exact seed counts after
  mutating tests run.

## Common tasks

### Add a new endpoint to the API client

1. Add the route in
   [`../case-folder-service-with-rust/spec/routes.md`](../case-folder-service-with-rust/spec/routes.md)
   first (the Loco subproject is the contract).
2. Add a typed method to the matching namespace in
   `src/lib/api/client.ts`. Reuse the snake → camel mappers; add a
   new one if a new shape is introduced.
3. Use it from a `+page.ts` loader or from a cache method.

### Add a new page

1. Add a use case + route row in [`spec/routes.md`](spec/routes.md).
2. Create `src/routes/<path>/+page.ts` and `+page.svelte` following
   the load + cache pattern.
3. If new mutation behaviour is needed, add a method to
   `src/lib/store/cache.svelte.ts` and document it in [`spec/cache-api.md`](spec/cache-api.md).
4. Add a nav link in `src/routes/+layout.svelte` if it should appear
   in the menu.
5. Run `npm run check`.

### Drop a page

1. Delete its `+page.ts`, `+page.svelte`, and (if empty) the folder.
2. Remove its nav link from `+layout.svelte`.
3. Update [`spec/routes.md`](spec/routes.md).
4. Update the [README](README.md) "What this app does" list.

### Map a new API field

1. Snake-case it in the `ApiX` interface in `client.ts`.
2. CamelCase it in `types.ts`.
3. Map it in the `toX` function in `client.ts`.
4. Use it from the page.

## Style

- TypeScript `strict`. No `any` in new code. Use `unknown` + a
  narrowing guard.
- Inline `///` doc comments on every public type + function in
  `src/lib/api` and `src/lib/store`. Keep them short.
- Module-level `//` doc comments on every route file are optional but
  appreciated when the route does something non-obvious.
- Don't add prose comments for what well-named code already says.
- One short line of `// reason: ...` is fine when the reasoning is
  non-obvious (e.g. a debounce timing, a quirky upstream).

## Sibling projects

- [`../case-folder-service-with-rust`](../case-folder-service-with-rust)
  — the JSON API back-end this client talks to. **Start the API
  before running the dev server.**
- `~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/`
  — source of the `lily-design-system-svelte-locale-picker` and
  `lily-design-system-svelte-theme-picker` `file:` dependencies.
  **Must be cloned next to this repo** under `~/git/lilydesignsystem/`
  for dev/build to succeed.
- The five upstream Main-X-Services live under
  `~/git/sixarm/main-x-service/` (only relevant if you're testing
  against real services rather than the Loco app's in-process stubs).
