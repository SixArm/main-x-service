# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

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

### Added (scaffold)

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
