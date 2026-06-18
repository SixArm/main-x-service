# Testing strategy

> Part of the [Svelte edition specification](index.md). Cross-cutting
> principles + stub mode: [root testing](../../spec/testing.md).

| Layer         | Tool                                  | Status                                          |
| ------------- | ------------------------------------- | ----------------------------------------------- |
| Type check    | `svelte-check`                        | ✓ in repo (`npm run check`)                     |
| Lint          | ESLint (flat) + typescript-eslint + eslint-plugin-svelte | ✓ in repo (`npm run lint`)           |
| Unit          | vitest                                | ✓ in repo (`npm run test:unit` — nhs + client mappers + cache store) |
| Component     | `@testing-library/svelte` (+ jsdom)   | ✓ in repo (`npm run test:unit` — Icon, InputCount, AddressographBox, ButtonBar) |
| E2E           | Playwright (Chromium)                 | ✓ in repo (`npm run test:e2e`)                  |
| Accessibility | `@axe-core/playwright`                | ✓ in repo (`tests/e2e/a11y.spec.ts`)            |

## Unit + component tests (in repo)

vitest runs in `jsdom`; `npm run test:unit` currently passes **43**
cases across 7 files. Required coverage:

- `store/nhs.ts` (`nhs.test.ts`):
  - `normaliseNhsNumber("943 476 5919") === "9434765919"`
  - `formatNhsNumber("9434765919") === "943 476 5919"`
  - `isValidNhsNumber("943 476 5919") === true`
  - `isValidNhsNumber("943 476 5918") === false`
  - `check === 10 → invalid` branch (`999 000 0140`), leading-zero
    normalisation, empty input, grouped/bare parity.
- `api/client.ts` (`client.test.ts`):
  - snake → camel mapping for every exported mapper (`toPatient`,
    `toFolder`, `toMove`, `toBuilding`, `toRoom`, `toCabinet`,
    `toWorker`, `toStats`, `toVolume`).
  - `ApiError` is thrown with the correct status + body shape.
- `store/cache.svelte.ts` (`cache.svelte.test.ts`):
  - setters / `clearUser` / `upsertFolder` (insert vs replace), the
    synchronous lookups, `cabinetLocation`'s three-step resolution, and
    (with `$lib/api/client` mocked) the cache side effects of
    `recordMove`, `addFolder`, `addBuilding/Room/Cabinet`.
- Components (`@testing-library/svelte`): `Icon`, `InputCount`,
  `AddressographBox`, `ButtonBar` each ship a `*.test.ts` asserting
  rendered output and prop reactivity.

## End-to-end tests (Playwright)

Layout: `playwright.config.ts` + `tests/e2e/**/*.spec.ts`. Tests run in
a single worker against Chromium; Playwright boots the Svelte dev server
itself (`webServer` config).

**Prerequisite**: the Loco JSON API must be running on `:5150`. The
easiest path is **stub mode** (no upstream Main-X-Services needed):

```bash
cd ../case-folder-service-with-rust
USE_UPSTREAM_STUBS=1 cargo run -- start
```

Stub mode swaps every Main-X-Service for an in-process `StubClient` and
seeds it with the same data `cargo run -- task seed` would populate
against real services. The API responds normally — the client can't tell
the difference. `tests/e2e/global-setup.ts` pings `/healthz` and
`/api/stats` before any test runs and fails fast with actionable
instructions if the API isn't up or hasn't been seeded.

Suites (14 spec files, ~65 `test()` cases):

| File                   | Tests | Coverage                                                                                |
| ---------------------- | ----- | --------------------------------------------------------------------------------------- |
| `smoke.spec.ts`        | 4     | Every primary route returns 200; nav links exist; skip-link is first; aria-current.     |
| `dashboard.spec.ts`    | 6     | KPI cards render; patient count; recent moves; cabinet utilisation; FolderGrid.         |
| `folders.spec.ts`      | 11    | List + search by title/patient/no-match; click → detail; create happy path + validation. |
| `patients.spec.ts`     | 7     | List + search; detail (incl. snapshot fallback warning for unknown NHS Number).         |
| `places.spec.ts`       | 8     | Buildings list/show/create/validation; add-room inline; cabinets list/create/validation. |
| `move.spec.ts`         | 5     | NHS lookup populates pane; worker + cabinet pickers; full move workflow; "In transit".  |
| `history.spec.ts`      | 5     | Seeded synthetic events visible; search by name/NHS; clearing filter.                   |
| `errors.spec.ts`       | 4     | `+error.svelte` when API blocked; 404 path; NHS Modulus 11 validation on both forms.    |
| `volumes.spec.ts`      | 2     | Seeded volume detail lists its folders (UC-V3); create → add folder → move whole volume (UC-V1/V2/V4). |
| `clickthrough.spec.ts` | 4     | Worker → moved folders + patients (UC-W1); cabinet → presence history (UC-P1); move row → event detail (UC-E1); building → room → presence history. |
| `auth.spec.ts`         | 3     | Protected route redirects to `/login` when signed out; magic-link sign in → sign out; unknown email does not reveal existence. |
| `ifit.spec.ts`         | 3     | Geofence alerts list a cross-building move (UC-I1); reports KPIs + cabinet utilisation (UC-I2); scan finds a folder by NHS Number and offers a move (UC-I3). |
| `wiring.spec.ts`       | 2     | Patient page shows the addressograph + action bar; volumes page opens the Labels print dialog. |
| `a11y.spec.ts`         | 9     | `@axe-core/playwright` scan (no serious/critical violations) over 9 primary routes (`/`, `/patients`, `/folders`, `/volumes`, `/workers`, `/cabinets`, `/alerts`, `/reports`, `/scan`). |

Helpers in `tests/e2e/helpers/`:

- `nhs.ts` — Modulus-11-valid NHS Number generator + seed-data constants.
- `seed.ts` — names of records the Loco seed task creates.
- `unique.ts` — unique-name generator so multiple runs don't collide.
- `forms.ts` — `fieldControl(page, labelText)` because Lily's `Field`
  component doesn't wire its `<label for>` to the child input.

```bash
npm run test:e2e                # headless
npm run test:e2e:ui             # interactive Playwright UI
npm run test:e2e:headed         # headed Chromium
```

## Dev, build, deploy

```bash
# In another terminal: start the Loco API (see ../case-folder-service-with-rust/README.md)
cd ../case-folder-service-with-rust && cargo run -- task seed && cargo run -- start

# Front-end
npm install
npm run dev                  # http://localhost:5173
npm run check                # svelte-check (0 errors required)
npm run build
npm run preview
```

### Required before merging a change

1. `npm run check` is clean (0 errors).
2. `npm run build` succeeds.
3. With the API running, the dev server loads every route in
   [routes.md](routes.md) with HTTP 200.
4. Any new use case is documented in [routes.md](routes.md).
5. Any new cache method is documented in [cache-api.md](cache-api.md).
6. Any new shape mismatch with the API is reflected in
   [domain-model.md](domain-model.md) and the `client.ts` mappers.

### Build target

The default adapter is `@sveltejs/adapter-auto`. For self-hosted NHS
trust deployment, swap to `@sveltejs/adapter-node` and put the SvelteKit
server + the Loco API behind one ingress (same origin = fewer security
questions).
