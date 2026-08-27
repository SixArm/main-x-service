# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

## [0.1.0] - 2026-08-04
### Fixed — Playwright stub matched the pre-BFF path (DOC-4, 2026-08-04)

- **All 8 e2e smoke tests were silently broken.** `stubApi()` in
  `tests/e2e/smoke.spec.ts` dispatched on `url.pathname` compared against
  bare service paths (`/api/cases`, `/api/cases/{pid}`, …). Since the
  BFF-proxy auth pivot, `ApiClient`'s base URL is the same-origin proxy
  (`location.origin + "/api/proxy"`, `src/lib/config.ts`), so the
  browser's actual request lands on `/api/proxy/api/cases` — the exact
  match never fired, every stubbed call fell through to the handler's
  final `404`, and 5 of 8 tests failed (`list`, `detail`, the links
  panel, `merge`, and check-duplicates self-exclusion; only the three
  tests asserting a static heading with no data dependency passed).
  Fixed by stripping the `/api/proxy` prefix from `url.pathname` before
  dispatch. Found and fixed while live-verifying this crate's quick-start
  commands for the DOC-4 documentation audit — `pnpm run check` and
  `pnpm test` stayed green throughout, so nothing short of actually
  running `pnpm test:e2e` surfaced it.

### Fixed — documentation audit (DOC-4, 2026-08-04)

- **`.env.example` documented the decommissioned client-held-token
  model's vars** (`PUBLIC_API_BASE_URL`, `VITE_AUTH_FRONTEND_URL`) —
  zero references in `src/`. The real BFF vars `CASE_API_URL` /
  `AUTH_API_URL` (read by `src/lib/server/config.ts`, already correctly
  documented in `README.md`) had no `.env.example` entry at all.
  Rewritten to match (the same bug TUT-1 found live and worked around
  without fixing this file).
- **`AGENTS.md` described a redirect-to-the-auth-front-end sign-in flow**
  that predates this app's own per-app `/signin`/`/verify` magic-link
  route (`href="/signin"`, local); fixed to match. Its `src/lib/config.ts`
  description (`PUBLIC_API_BASE_URL` + `AUTH_FRONTEND_URL` +
  `signInUrl()`) also predated the BFF pivot — the file now only exports
  `API_BASE_URL` (the same-origin proxy) — fixed, along with the missing
  `/cases` and `/board` routes in the layout tree, the missing
  merge/links rows in the API-consumption table, the stale
  `PUBLIC_API_BASE_URL`/`VITE_AUTH_FRONTEND_URL` configuration line, and
  the stale unit-test file list (`auth`/`config` no longer exist;
  `i18n`/`layout`/`link-validation`/`merge-validation` were missing).
- **`index.md`'s "Auth pivot in progress" warning was stale** — the BFF
  pivot landed (per `AGENTS.md` and `spec/index.md §13`); replaced with
  a statement of the landed state, and added the merge/links endpoints
  to the Flow diagram (previously only the four original CRUD routes).
- **`README.md`'s route table omitted `/merge`** (FE-1, landed
  2026-08-03) despite listing every other route including `/board` and
  `/cases`; added.
- **`spec/index.md`** — §5's information architecture omitted `/cases`
  and `/board` (present in §2's own scope line and in `AGENTS.md`); added.
  §7 flatly claimed "dependency-light (no data grid / design system)",
  contradicted by the SVAR DataGrid/Kanban/Filter + Lily picker
  dependencies added 2026-07-19 (see below) — corrected. §11's testing
  narrative and §13's task counts still described the pre-merge/pre-links
  suite (`auth.test.ts`/`config.test.ts`, "40 tests across 5 files" /
  "5 tests"); corrected to the live counts (61 vitest / 8 Playwright
  across 7 + 1 files) and to the e2e stub-path fix above. §14/§15 still
  read like the v0.1 four-route MVP with auth as a roadmap item, despite
  §13 marking BFF auth, merge, and links **done**; rewritten against the
  real eight-route, BFF-complete surface.
- Verified live: `pnpm run check` (0 errors / 0 warnings), `pnpm test`
  (61/61), `pnpm test:e2e` (8/8, after the stub fix above), `pnpm run
  build` all green.

### Added — cross-service links panel (FE-2, 2026-08-03)

- **"Subject of this case" panel** on the detail route (`/[pid]`):
  lists, asserts, and withdraws the `subject_of` (case → person) edges
  this case originates, via `GET`/`POST`/`DELETE
  /api/cases/{pid}/links`. Case originates exactly one edge kind, so
  there is **no kind picker** — `kind` is fixed to `subject_of`.
- **Sensitivity-aware presentation.** The edge asserts that a named
  person is the subject of a governmental case
  (`agents/share/cross-service-linking.md` §10): a plainly-labelled
  section with an explanatory note, and a `confirm()` that names the
  person reference being retracted before a withdrawal.
- **`CaseRepository.listLinks()` / `createLink()` / `deleteLink()`**,
  with `EntityLink` / `CreateLinkRequest` types and the `SUBJECT_OF`
  constant.
- **Pure `validateLink` guard** (`src/lib/components/link-validation.ts`)
  mirroring the service's `validate_edge`: a person `EntityRef` URN
  (`person:<uuid>`) and a confidence in `[0,1]`. Returns an i18n key, so
  the message renders in the operator's locale.
- **Server errors surfaced from `description`, not `error`.** Loco's
  `ErrorDetail` puts the machine code (`validation`) in `error` and the
  reason in `description`; the shared client's generic extractor prefers
  `error`, so the panel reaches past it — an operator sees why the edge
  was refused rather than the word "validation".
- **24 new i18n keys across all 13 locales**, plus unit tests
  (`link-validation`, repository URL pins, links-block catalog coverage
  including the `{ref}` placeholder) and a Playwright smoke assertion.

### Added — record-merge UI (2026-08-03)

- **`/merge` route** (nav-linked): merge a confirmed duplicate case into a
  survivor. Optional side-by-side preview of both records, a native
  `confirm()` before the destructive call, and a **recent merges** table
  (merged-at / main / duplicate / reason / actor) loaded on mount and
  refreshed after a successful merge.
- **`CaseRepository.merge()`** → `POST /api/cases/merge` and
  **`recentMerges()`** → `GET /api/cases/merges/recent`, with
  `MergeRequest` / `MergeResponse` / `MergeRecordRow` types.
- The wire shape is this service's own, not a sibling's: the request is
  `{main_pid, duplicate_pid, reason?}` and the response is
  `{main_pid, duplicate_pid, main}` — there is **no `merge_record`
  wrapper**, so the merge row's id and timestamp are not in the response.
  The page links to the survivor and reads timestamps from the history
  endpoint instead.
- **`validateMerge`** (`src/lib/components/merge-validation.ts`) is a pure
  helper returning an **i18n key** (not English prose), so the guard is
  unit-testable and the message follows the selected locale. It rejects a
  self-merge locally, which the service would answer `422`.
- 25 new strings across all 13 locales; the parity test additionally pins
  that every locale's `merge.confirm` keeps both `{dup}` / `{main}`
  placeholders, since a translation that drops one would silently render
  a prompt naming no case.

### Added — paged collection reads (2026-08-01)

- **`ApiClient.getPage()`** returns `{ items, total, limit, offset }`,
  reading the service's `X-Total-Count` / `X-Limit` / `X-Offset` headers.
  The plain `get()` throws response headers away, which is fine for one
  record and useless for a collection. A service that predates the
  headers still works: the page length is the fallback.
- **`CaseRepository.listPage()`** wraps it; `list()` is unchanged for callers
  that just want the default page.
- Note the service reports the **collection's** total, taken before
  the per-record concealment it applies, so a caller may legitimately
  receive fewer rows than `total` suggests. That is deliberate: a
  caller-specific total would leak how many records concealment is
  hiding.


### Added

- 2026-07-19 — SVAR strong fit: new **/board** route (nav-linked): cases as SVAR Kanban cards,
  one column per unit lifecycle status; dragging a card writes the
  status change via the normal full-record PUT and reloads. The
  refs-only list endpoint means the board loads the full records
  behind the refs (capped at 100) — a status-bearing list endpoint
  is the optimisation seam. One new i18n key (`nav.board`) x 13.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: new **/cases** index route: the case list in the SVAR DataGrid
  with a FilterBar (client-side title filter); row selection opens
  the detail route.

- 2026-07-19 — Lily Design System: the hand-rolled locale `<select>` is replaced by the Lily
  **LocaleSelect** (wired to the i18n store; `applyDir` off), and
  the **Lily headless** component library is now a dependency
  alongside the existing ThemeSelect.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

### Changed

- **Auth pivot.** The family
  authentication model moved from **client-held RS256 JWT bearer tokens**
  (fragment handoff + `localStorage["mxi_access_token"]`) to a
  **Backend-For-Frontend (BFF) + httpOnly cookie session + CSRF**, with
  the BFF exchanging the session for a short-lived **PASETO v4.public**
  token for server-side service calls — see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  as the source of truth; RS256/JWKS are decommissioned. Human-facing
  docs (README / index / AGENTS) describe the BFF + cookie model;
  the browser holds no token. The runtime implements the BFF
  (`src/hooks.server.ts`, `src/lib/server/*`, the `/api/proxy/[...path]`
  server route); the old client-held bearer flow (`auth.svelte.ts`,
  fragment capture) is removed.
- **Doc harmonization pass.** Refreshed `AGENTS.md` to match the spec's
  auth/SSO layer: added `src/lib/auth.svelte.ts` and the `tests/` tree to
  the layout, noted `config.ts` now also exports `AUTH_FRONTEND_URL` /
  `signInUrl()`, added a bearer-token / SSO ground rule, and documented
  `VITE_AUTH_FRONTEND_URL` alongside `PUBLIC_API_BASE_URL`. Added an
  SSO token-handoff worked example to `index.md` so the navigation aid
  reflects the implemented sign-in flow (spec §6.7-6.8, §8). No code
  change; vitest 40 green, `pnpm run check` 0/0.

### Added

- **Cross-origin SSO token handoff (consumer side)** (family contract
  [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md),
  "Token acquisition handoff").
  - `src/lib/auth.svelte.ts` gains `captureTokenFromHash(hash)` — a pure
    parser pulling `access_token` out of a `#…access_token=<jwt>…` URL
    fragment (with/without leading `#`, URL-decoded, `null` otherwise) —
    and a browser-only `captureFromLocation()` that reads
    `window.location.hash`, stores any token, then strips the fragment
    via `history.replaceState` so the bearer credential never lingers in
    the address bar / history.
  - The layout runs `captureFromLocation()` once in `onMount` before any
    route makes an API call.
  - `src/lib/config.ts` gains `AUTH_FRONTEND_URL` (from
    `VITE_AUTH_FRONTEND_URL`, default `http://localhost:5173`) and
    `signInUrl(origin?, basePath?)`, building
    `${AUTH_FRONTEND_URL}/signin?return_to=<encoded origin + base>`
    (trailing slash trimmed; origin / base injectable for tests).
  - Layout sidebar now shows a primary **Sign in** link (redirects to the
    auth front-end) when signed out; the manual paste field is demoted to
    a dev-only `<details>`. **Sign out** unchanged.
  - Tests: `auth.test.ts` adds the `captureTokenFromHash` cases
    (well-formed, multi-param, no leading `#`, URL-decode, empty/`#`,
    no-token, garbage → `null`); new `config.test.ts` covers `signInUrl`
    (encoded `return_to`, base path, trailing-slash safety). vitest 31
    green; Playwright smoke suite stays green.

- **Session bearer-token attachment** (family contract
  [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md)).
  - New reactive session store `src/lib/auth.svelte.ts` (`token` /
    `setToken` / `clearToken`), hydrated from the family-shared
    `localStorage["mxi_access_token"]` and guarded for SSR / `vite
    preview` / vitest where `localStorage` is absent.
  - `ApiClient` now reads the session token by default and attaches
    `Authorization: Bearer <token>` on every request when present; a
    per-call `token` (string or `null`) still overrides, and a
    `tokenSource` seam keeps it unit-testable.
  - Minimal session affordance in the layout sidebar: paste / clear the
    token (issued by the central authentication-service magic-link flow),
    so operator traffic passes the service's blanket JWT enforcement
    (`CASE_REQUIRE_AUTH`) once activated.
  - Tests: vitest `auth.test.ts` (no-token default, round-trip, guarded
    write-through under the shared key) + new `ApiClient` cases
    (store-default header, per-call `null` override). Playwright smoke
    suite stays green.

### Added (scaffold)

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Case Service (governmental case management), copy-adapted from
  care-pathway-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client (get/post/put/delete); `CaseRepository`.
  - `types.ts` mirrors `case_matcher::Case` (the service DTO),
    including `CaseType`, `CaseStatus`, `Priority`, and
    `IdentifierScheme`, plus the `ALL_*` dropdown arrays.
  - `CaseForm` editing title (required), case number, agency id/name,
    case type / status / priority dropdowns, opened date, comma-list
    inputs (alternate titles / subjects / keywords / same-as /
    languages), and identifier rows (scheme + value).
  - SPA mode; dependency-light (no SVAR/Lily). `pnpm run check` clean
    (0/0); production build succeeds.
  - **Test suites.** vitest unit tests (`tests/unit/`, 16) for the
    `ApiClient` and `CaseRepository` — verb/path/body/bearer-token,
    error classification, and a regression pinning the
    `check-duplicates` path. Playwright smoke tests (`tests/e2e/`, 4)
    load the four routes with the API stubbed via `page.route`; they
    run against the production build (`vite preview`) to dodge the
    `vite dev` cold-start module-load race. `playwright.config.ts`
    included.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
