# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (CPFE-T5)

`src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch,
token)` with no `try`/`catch`. A network-level failure (the
authentication service unreachable, timed out, connection reset) makes
`fetch` throw rather than resolve — uncaught, that propagated out of
`load` and SvelteKit rendered its generic 500 error page instead of
this route's own friendly UI. The same bug class was found and fixed
first in `place-front-end-with-svelte` (T-26) and
`thing-front-end-with-svelte` (T-23); ported here: a `try`/`catch`
around the call, a new `"serviceUnavailable"` error variant, and its
message in `+page.svelte`. New `tests/unit/verify.test.ts` unit-tests
the `load` function directly (missing token / service unavailable /
invalid token), verified to fail with the `try`/`catch` reverted and
pass with it restored. See spec §13 CPFE-T5.

### Added — the time-based-analysis view (TBA-8)

`/time`, in the main nav: a pathway's cohort measured against elapsed
calendar time, and one patient journey drawn to scale.

- **The cohort half** — the value-adding ratio, coverage, lead time by
  percentile, and a score against a **named NHS access standard**
  chosen from the service's catalogue (RTT 18 weeks, the cancer
  standards, DM01, A&E). The tile carries the target verdict *and the
  date the threshold was last checked*, because targets move and a
  stale one silently mis-scores a cohort.
- **The journey half** — `JourneyTimeline.svelte`, the timeline wall:
  one row, every band sized in proportion to its duration and coloured
  by whether it added value. Inline SVG-free flex layout, no charting
  dependency. Below it, the longest queues, each named by what it sits
  between.

Three things the view is careful about:

1. **Unrecorded time is a band, never a gap in the picture.** On a real
   journey it is usually the widest one. Dropping it is precisely how a
   journey nobody mapped comes to look efficient. It is drawn in a
   neutral rather than a hue, because it means "nobody recorded this",
   not "a fourth kind of activity".
2. **An unmapped journey is never banded.** A journey with no segments
   reports a ratio near zero that looks identical to a catastrophically
   wasteful one. `valueAddingBand` returns `unknown` whenever coverage
   says `unmapped`, whatever the number is, and the coverage tile spells
   out what the figure can and cannot support. Pinned by a unit test.
3. **A single-digit ratio is captioned as the norm.** Tracked NHS
   journeys measure 8–14%, so "14.0%" with no context invites the wrong
   reaction — and an implausibly *high* figure is called out as a
   probably-unmapped journey rather than as excellence.

The three hues are the validated categorical palette's first three
slots, run through the validator in both modes on the **all-pairs**
list, not merely adjacent pairs. Aqua sits below 3:1 on the light
surface, so the wall ships a table view — the palette's relief rule,
pinned by an e2e test. Every band is a `<button>`, so a keyboard reaches
exactly what a pointer does; that too is pinned rather than assumed.

`nav.time` was added to all 13 locale catalogues (the `StringKey` union
makes a missing translation a type error, so there was no half-way
option).

Note for `/sequence`, whose comment says per-step durations are "the
seam that would make this a real timeline": this does **not** close
that. The pathway *template* still carries no durations. What TBA adds
is a real timeline for an *instance*, which is a different object.

### Fixed — Playwright smoke suite broken by the 2026-08-03 BFF-proxy fix (2026-08-04)

- `tests/e2e/smoke.spec.ts`'s API stub matched requests against the bare
  service path (`/api/care-pathways/...`), which is what the client sent
  **before** commit `ad95088e` (2026-08-03, "Fix ApiClient dropping the
  BFF proxy prefix from every request", tasks.md FE-2) corrected
  `ApiClient` to route through the same-origin BFF proxy
  (`/api/proxy/api/care-pathways/...`). That fix landed with a
  regression test in five other front-ends but not here, and this
  suite's stub was never updated to match — **all 6 Playwright tests
  were failing** (`unhandled in stub`) before this fix. The stub now
  strips the `/api/proxy` prefix before matching; verified green
  (6/6) against `vite preview`.

### Doc sync (2026-08-04, DOC-4 audit)

- **`.env.example` documented the decommissioned client-held-token
  model's vars** (`PUBLIC_API_BASE_URL`, `VITE_AUTH_FRONTEND_URL`, zero
  references in `src/`) instead of the real BFF vars
  `CARE_PATHWAY_API_URL`/`AUTH_API_URL` that `src/lib/server/config.ts`
  actually reads (already correctly documented in `README.md`/
  `AGENTS.md`) — rewritten to match.
- **`spec/index.md` had not been updated for the 2026-07-19 –
  2026-08-01 SVAR rebuild**: §2/§5/§6/§9 still described the v0.1–v0.3
  dependency-light app (`/care-pathways` as a route that no longer
  exists; no mention of `/insights`, `/board`, `/gantt`, or the
  instances layer at all), and §7 flatly claimed "dependency-light (no
  data grid / design system)" while the app has used the full SVAR
  suite (DataGrid, FilterBar, Kanban, Gantt) plus Lily since 2026-07-19.
  Rewrote §2/§5/§6/§7/§9/§11/§13/§14/§15 against the actual routes,
  API surface, and test suite. Also surfaced (not silently fixed): the
  list page's v0.2 search-on-submit box and v0.3 recent-activity toggle
  were dropped in the SVAR rebuild without replacement —
  `CarePathwayRepository.search()`/`.recentEvents()` are unit-tested but
  wired to no route — and merge/audit-trail lost their Playwright
  coverage in the same rebuild (vitest-only today). Both flagged as
  open follow-ups in spec §13 rather than resolved unilaterally.
- **Stale test counts**: spec §11/§13 said "46 tests across 5 files"
  and described `tests/unit/auth.test.ts`/`config.test.ts` (removed
  when the BFF migration replaced the client-held-token auth store);
  the live suite is 48 vitest tests across `client.test.ts`,
  `care-pathways.test.ts`, `i18n.test.ts`, `care-pathway-form.test.ts`,
  and `layout.test.ts`. Playwright was "8 tests" in spec, actually 6
  (rewritten for the SVAR routes, see the Fixed section above).
  `pnpm run check` (0/0), `pnpm test` (48/48), `pnpm test:e2e` (6/6),
  and `pnpm build` all verified green.
- `README.md`'s route table still listed `/care-pathways` and omitted
  `/insights`/`/board`/`/gantt`; fixed, and its "How it works" section's
  search/recent-activity mention (unwired, see above) replaced with the
  insights/instances description. `AGENTS.md`'s BFF description and API
  table were already accurate; added a note that search/recent-activity
  are unwired repository methods, for consistency with the spec.

### Added — paged collection reads (2026-08-01)

- **`ApiClient.getPage()`** returns `{ items, total, limit, offset }`,
  reading the service's `X-Total-Count` / `X-Limit` / `X-Offset` headers.
  The plain `get()` throws response headers away, which is fine for one
  record and useless for a collection. A service that predates the
  headers still works: the page length is the fallback.
- **`CarePathwayRepository.listPage()`** wraps it; `list()` is unchanged for callers
  that just want the default page.


### Added
- 2026-07-20 — Reshaped to the four-SVAR-component brief plus the
  instance layer: `/` registry SVAR DataGrid + FilterBar, `/board`
  instance SVAR Kanban (drag → `POST /api/instances/{pid}/status`),
  `/gantt` instance-timeline SVAR Gantt, `/insights` the five registry
  lenses (directory / coverage / variants / providers / languages),
  and the detail page's instances section. The intervention-sequence
  Gantt (`/sequence`) is retained. API client + types extended for the
  instance and insight endpoints; nav + `nav.{insights,board,gantt}`
  keys in 13 locales. svelte-check 0, vitest 46, Playwright 6.

- 2026-07-19 — SVAR moderate fit: new **/sequence** route (nav-linked): the selected pathway's
  interventions as ordered bars in the SVAR Gantt — explicitly a
  **sequence view, not a schedule** (the model carries only
  intervention order; the ordinal axis is unlabelled and each step
  is one nominal unit; per-step durations on the service model are
  the seam for a real timeline). The suggested status Kanban is
  data-gated: CarePathway has no lifecycle status field.
  +nav.sequence x 13 locales.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: new **/care-pathways** index route: the pathway list in the SVAR
  DataGrid with a FilterBar (client-side name filter); row selection
  opens the detail route.

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

- **Auth pivot — BFF + cookie session + PASETO (spec-level; code
  follow-up pending).** The family is moving off the browser-held RS256
  JWT (cross-origin `#access_token` fragment handoff,
  `localStorage["mxi_access_token"]`) to a **Backend-For-Frontend**: the
  browser holds only an httpOnly `__Host-mxi_session` cookie, the
  front-end's own SvelteKit server exchanges the session for a
  short-lived **PASETO v4.public** token and calls the care pathway
  service server-side, and mutating requests are CSRF-protected. RS256
  JWT + JWKS are decommissioned. Human-facing docs (README/agents/index)
  updated to describe the target model; the current runtime still uses
  the older client-held-token flow and the code follow-up is tracked in
  spec §13. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

- **Docs ↔ spec harmonization pass.** Refreshed `AGENTS.md`, `README.md`,
  and `index.md` from "CRUD + matching" to the shipped surface (name
  search, merge, audit trail, recent activity, bearer-token / SSO auth):
  added the search / recent-events / audit rows to the AGENTS API table,
  the full `types.ts` export set and `care-pathways.ts` method list, an
  Auth section, and `pnpm test` / `pnpm test:e2e` to the command blocks;
  added the search / merge / audit / recent-activity endpoints and the
  bearer/SSO note to the `index.md` Flow block. Populated the previously
  empty `.env.example` with `PUBLIC_API_BASE_URL` and
  `VITE_AUTH_FRONTEND_URL`. Corrected the stale "29 tests" count in
  spec §13, and the stale "4 tests" Playwright count there to 8 (the four
  routes plus search / merge / audit / recent-activity smoke tests).

### Added

- **`in_language` (BCP-47 languages) form + detail wiring.** The
  `CarePathway.in_language` field (a real, BCP-47-validated service field)
  was carried in `types.ts` but neither editable nor displayed. Added a
  comma-separated **Languages** input to `CarePathwayForm` (round-tripping
  through `build()` like the other list fields) and a "Languages" row to
  the detail page. Spec §8 now lists it among the editable fields.
- **`CarePathwayForm` component tests.** New
  `tests/unit/care-pathway-form.test.ts` (5 tests, via
  `@testing-library/svelte` mounted client-side by the `svelteTesting()`
  vite plugin): the required-name guard blocks `onsubmit` on a
  blank/whitespace name and shows the banner; `build()` normalization
  (trim scalars, blank→null, comma-list split incl. `in_language`, drop
  empty condition-code / identifier rows, collapse a `Custom`
  care-setting / identifier-scheme seed).
- **Merge error-path tests.** Two repository tests pin that `merge()`
  propagates a `404` (unknown pid) and `422` (equal-pid) as a classified
  `ApiError` for the detail-page error banner (spec §6.7).

## [0.3.0] — 2026-06-15

### Added

- **Cross-origin SSO token handoff (consumer side).** The session
  affordance now leads with **Sign in**, which redirects to the central
  authentication front-end
  (`${VITE_AUTH_FRONTEND_URL}/signin?return_to=<origin + base>`); after
  the magic-link, the auth front-end hands the access token back via the
  URL fragment (`…#access_token=<jwt>`, allowlist-gated). `auth.svelte.ts`
  gains a pure `captureTokenFromHash(hash)` (URL-decoded `access_token`,
  else `null`) and a browser-only `captureFromLocation()` that stores the
  token and strips the fragment with `history.replaceState`; the layout
  `onMount` runs it before any API call. The manual paste field is kept
  behind a disclosure as a dev convenience. New config:
  `VITE_AUTH_FRONTEND_URL` (default `http://localhost:5173`) + a
  `signInUrl(origin?, basePath?)` builder (encoded `return_to`, base-path
  aware, trailing-slash safe). vitest adds 10 tests (`auth.test.ts`: 7 ×
  `captureTokenFromHash`; new `config.test.ts`: 3 × `signInUrl`);
  `pnpm run check` 0/0 and Playwright smoke stay green. Family contract:
  `agents/share/jwt-enforcement.md`.

- **Bearer-token auth (front-end half of blanket JWT enforcement).** A
  new reactive token store `src/lib/auth.svelte.ts` holds the access
  token, hydrated from the family-shared `localStorage` key
  `mxi_access_token` (guarded for SSR / `vite preview`), exposing
  `setToken` / `clearToken` / `token`. `ApiClient` now reads this store on
  every request and attaches `Authorization: Bearer <token>` when present
  (a per-call `token` — string or `null` — still overrides). The layout
  sidebar gains a minimal session affordance to paste/clear the token; the
  token is obtained out-of-band from the central authentication-service
  (passwordless magic-link). This lets operator traffic through once the
  service turns on blanket enforcement (`CARE_PATHWAY_REQUIRE_AUTH`, off
  by default). vitest adds 6 tests (`tests/unit/auth.test.ts`: store
  round-trip + client attachment/omission/override); Playwright smoke
  stays green. Family contract: `agents/share/jwt-enforcement.md`. Full
  magic-link redirect wiring is a follow-up.

- **Recent-activity view.** The list page (`/`) gains a "Show recent
  activity" toggle that lazy-loads `GET /api/care-pathways/events/recent`
  on first open (it does not auto-load on mount) via a new
  `CarePathwayRepository.recentEvents()` → returning a `PathwayEvent[]`
  (`{kind, pid, name, seq}`, mirroring the service's
  `streaming::PathwayEvent`; `kind` is created/updated/deleted/merged).
  Events render newest-first (highest `seq` first) with the kind, the
  name (linked to the pathway by pid), and the `seq`; loading, empty, and
  error states are handled. vitest adds 1 unit test (path); Playwright
  adds 1 smoke test (toggle → events render newest-first with kind +
  seq).
- **Audit-trail view.** The detail page (`/[pid]`) gains a "Show audit
  trail" toggle that lazy-loads `GET /api/care-pathways/{pid}/audit` on
  first open (it does not auto-load on mount) via a new
  `CarePathwayRepository.audit(pid)` → returning an `AuditEntry[]`
  (`{action, actor, snapshot?, created_at?}`, mirroring the service's
  `audit_logs` model). Rows render newest-first with the action, the
  actor (or "—" when `null`), and the timestamp; loading, empty, and
  error states are handled. vitest adds 2 unit tests (path + pid
  URL-encoding); Playwright adds 1 smoke test (toggle → rows render with
  action + "—" actor).
- **Merge-duplicate action.** The detail page (`/[pid]`) now offers a
  "Merge into this record" action on each potential-duplicate row (the
  detail record is the survivor/main; the row's pid is the duplicate).
  A two-step inline confirm calls a new
  `CarePathwayRepository.merge(mainPid, duplicatePid, reason?)` →
  `POST /api/care-pathways/merge` with body `{main_pid, duplicate_pid,
  reason?}` (pids in the body, not the URL), returning the new
  `MergeResult` (`{main_pid, duplicate_pid, main}`). On success the page
  adopts the returned survivor record, re-runs check-duplicates, and
  shows a success message; equal pids are guarded client-side and
  `404`/other errors surface via the existing error banner. vitest adds
  2 unit tests (body shape + reason-omitted); Playwright adds 1 smoke
  test (check-duplicates → confirm merge → success state, asserting the
  merge endpoint fired).
## [0.2.0] — 2026-06-13

### Added

- **List search box.** The list page (`/`) gains a name-search box
  (search-on-submit + **Clear**). A non-blank query calls
  `GET /api/care-pathways/search?q=` (URL-encoded) via a new
  `CarePathwayRepository.search(q)`; an empty query or **Clear**
  restores the full `list()`. Loading and empty-result states handled.
  vitest adds 2 unit tests (path + URL-encoding); Playwright adds 1
  smoke test (matching keeps the row, non-matching shows the empty
  message). Closes the spec §13 "search box" task.
- **Test suites (T-5).** vitest unit tests (`tests/unit/`, 16) for the
  `ApiClient` and `CarePathwayRepository` — verb/path/body/bearer-token,
  error classification, and a regression pinning the `check-duplicates`
  path. Playwright smoke tests (`tests/e2e/`, 4) load the four routes
  with the API stubbed via `page.route`; they run against the
  production build (`vite preview`) to dodge the `vite dev` cold-start
  module-load race. `playwright.config.ts` added.

### Fixed

- Copy-paste artifacts from the scaffold source: `client.ts` header
  said "Authentication Service"; `app.html` description said "Course
  Service" — both now read "Care Pathway Service".

## [0.1.0] — 2026-06-12

### Added

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Care Pathway Service, copy-adapted from
  organization-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client (get/post/put/delete); `CarePathwayRepository`.
  - `types.ts` mirrors `care_pathway_matcher::CarePathway` (the service
    DTO), including `CodeSystem`, `CareSetting`, and `IdentifierScheme`.
  - `CarePathwayForm` editing scalars, care setting, target condition
    codes (system + code rows), interventions/keywords, and identifiers.
  - SPA mode; dependency-light (no SVAR/Lily). `pnpm run check` clean
    (0/0); production build succeeds.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
