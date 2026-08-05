# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]
### Added — wire `/plans` itself to `listPage()` (2026-08-05)

The sibling gap the entry directly below flagged and left open:
`PlanRepository.listPage()` existed (and was tested at the repository
level) but `src/routes/plans/+page.svelte` still called the unpaginated
`list()`. Now it calls `listPage()` and shows the same `shown / total`
count as `/organizations`, `/automations`, and `/reviews`.

- **`src/routes/plans/+page.svelte`** — `onMount` calls `repo.listPage()`
  instead of `repo.list()`; a `.count` paragraph shows `shown / total`
  (hidden when the list is empty), reusing the exact style/markup
  pattern from the organization front-end's `/organizations` list route.
- No `?parent=` roll-up wiring added — the list route never took a
  parent scope before this change either.
- 3 new `tests/unit/plans.test.ts` cases (65 → 68 vitest total) pinning
  `listPage()`'s URL, header parsing, and the header-absent fallback.
  No new Playwright coverage needed: the existing `list page renders the
  seeded plan` smoke test already exercises the now-paginated path.
- `svelte-check` 0/0; `pnpm run build` green; 23/23 Playwright pass.

### Added — pagination on the automation + review capability views (repo tasks.md PG-1, 2026-08-05)

The service's `automations`, `automations/runs`, `scheduled-actions`,
and `reviews` list endpoints gained real `?limit=&offset=` pagination
(`X-Total-Count`/`X-Limit`/`X-Offset`), closing the last PG-1 sub-bullet
left open on 2026-08-01. This wires the two consuming routes to it.

- **`src/lib/api/capabilities.ts`** — `listAutomationsPage`, `runsPage`,
  `listScheduledPage`, `listReviewsPage`, `inboxPage`: thin
  `ApiClient.getPage()` wrappers alongside their existing unpaginated
  siblings, same pattern as `PlanRepository.listPage()` next to `list()`.
- **`/automations`** — the rules table, the deadline queue, and the run
  log each show a `shown / total` count (hidden when the list is empty),
  matching the organization service's front-end `/organizations`
  convention.
- **`/reviews`** — the delegation list gets the same count.
- **Notifications stay unwired.** `inboxPage()` exists for contract
  parity, but no route calls `inbox()`/`inboxPage()` — a documented gap
  since the 2026-07-22 capability-views entry, and building a
  notifications inbox page is new UI scope this task did not take on.
- **No Prev/Next pager was added**, matching the family's headers-plus-
  count convention (`agents/share/restful.md`) — deep paging past the
  default page still works via `?limit=&offset=`, just not from a UI
  control yet.
- 3 new `tests/unit/capabilities.test.ts` cases (62 → 65 vitest total),
  reusing `tests/unit/client.test.ts`'s `getPage` header-stub pattern.
  No new Playwright coverage: `/plans`'s `listPage()` has none either
  (it is not wired into `/plans/+page.svelte`), so there was no
  established e2e depth for this change to match.
- `svelte-check` 0/0; `pnpm run build` green.

### Fixed — doc/code reconciliation + e2e route staleness (2026-08-04, DOC-4 audit)

- **`.env.example`** documented the decommissioned client-held-token
  model's vars (`PUBLIC_API_BASE_URL`, `VITE_AUTH_FRONTEND_URL`, and a
  leftover "Case Service" header comment copy-pasted from the case
  front-end) — zero references in `src/`. Rewritten to the real
  server-side-only vars `PROJECT_PORTFOLIO_MANAGEMENT_API_URL` /
  `AUTH_API_URL` (`src/lib/server/config.ts`), matching README/AGENTS.md
  which already described them correctly for the BFF proxy but not for
  the file operators actually copy.
- **`tests/e2e/smoke.spec.ts` and `tests/e2e/ppm.spec.ts` were
  navigating to `/projects*` routes that have not existed since the
  2026-07-20 `/plans` collection unification**, plus both stubs matched
  request paths without stripping the BFF proxy prefix
  (`/api/proxy/api/...`, from `ad95088e`). Neither `pnpm run check` nor
  `pnpm test` (vitest) nor `pnpm run build` exercises Playwright, so this
  was invisible until `pnpm test:e2e` was actually run — which had
  apparently not happened since the route unification landed. Fixed
  both: route paths (`/projects` → `/plans`, including the task-board
  route), stub match paths, and the proxy-prefix strip. All 23
  Playwright tests now pass (was 21 failing of 23).
- **`spec/index.md`, `AGENTS.md`, `README.md`, `index.md`** described
  several capabilities as built that have no repository method, type, or
  route: a server-round-trip name search, a recent-activity feed, a
  per-plan audit timeline, a per-component match-score breakdown visual,
  and the plan-detail child-plan roll-up. Also corrected: the auth model
  description (this app's own `/signin` + `/verify` BFF pages, not a
  redirect to a central authentication front-end — `signInUrl()` and
  `src/lib/auth.svelte.ts` do not exist), stale `src/` tree entries
  (`src/lib/i18n/` is one file, not a directory; `PlanRepository` does
  not carry `audit`/`recentEvents`/sub-resource methods), and several
  stale test-suite descriptions and counts. See `spec/index.md` §6, §8,
  §9, §11, §13 and `AGENTS.md`'s Status note for the corrected detail.

### Added — record-merge page (2026-08-03)

- **`/plans/merge`** — an operator page for folding a confirmed-duplicate
  plan into a survivor: survivor pid + duplicate pid + optional reason,
  an optional side-by-side preview, a native `confirm()` before the
  destructive call (the duplicate is soft-deleted), and a recent-merge
  history table. Reachable from a new **Merge** entry in the top-bar nav.
- **`PlanRepository.recentMerges()`** — `GET /api/plans/merges/recent`,
  the service's merge history (newest first, service cap 100).
- **`MergeRequest` / `MergeResponse` / `MergeRecordRow`** wire types.
  This service's merge response is `{main_pid, duplicate_pid, main}` —
  there is deliberately **no** `merge_record` wrapper the way the person
  service has one, so the page reads the history row back separately.
- **`src/lib/components/merge-validation.ts`** — the pre-merge guard as a
  pure, unit-testable function returning an i18n key: both pids required,
  and they must differ (the service answers `422` on a self-merge, so
  catching it here saves the round trip and states the reason in the
  operator's own language).
- 24 new i18n keys across all 13 locales.

### Changed

- **`PlanRepository.merge()`** now takes a single `MergeRequest` object
  rather than three positional arguments, so the call site reads as the
  wire body it becomes.

### Added — paged collection reads (2026-08-01)

- **`ApiClient.getPage()`** returns `{ items, total, limit, offset }`,
  reading the service's `X-Total-Count` / `X-Limit` / `X-Offset` headers.
  The plain `get()` throws response headers away, which is fine for one
  record and useless for a collection. A service that predates the
  headers still works: the page length is the fallback.
- **`PlanRepository.listPage()`** wraps it; `list()` is unchanged for callers
  that just want the default page.
- `listPage()` keeps the optional `parent` scope, which the service
  applies to its count as well as its page.


### Added

- 2026-07-22 — **Capability views for collaborative review, workflow
  automation, prioritisation, and bird's-eye visibility**, consuming
  the service's new endpoints (service spec §9.4a):
  - `src/lib/api/capabilities.ts` — `CapabilityClient` plus the wire
    types (`Review`, `Consensus`, `Automation`, `AutomationRun`,
    `ScheduledAction`, `Notification`, `SmartScore`, `RankedPlan`,
    `LifecycleFunnel`, `PlanLifecycle`, `AssigneeLoad`). One method per
    endpoint; unset query parameters are omitted rather than sent blank.
  - `/prioritisation` — the Smart Score queue with an **Explain**
    toggle showing each component's weight, value, and points, plus the
    per-plan evidence coverage. Unscored plans read as `unscored`, never
    as a low score.
  - `/lifecycle` — the challenge funnel: live and stalled counts for
    every phase (empty phases included) and any items in an
    unrecognised phase.
  - `/reviews` — delegate an item to an internal or external expert,
    accept/decline, submit a verdict, withdraw, and read the consensus
    (which shows outstanding invitations and reports a tie as a tie).
  - `/automations` — configure rules (trigger + action, with the action
    fields switching on the chosen action kind), enable/disable/delete,
    the pending deadline queue with a manual sweep, and the run log
    including skipped and failed runs.
  - Nav entries for the four new pages; 6 vitest cases pinning the
    client's endpoint paths.
  - **Not built:** these four pages are English-first (the 13-locale
    catalogues are not yet extended to them, matching the other PPM
    catalogue views), and there is no dedicated notifications inbox page
    — the inbox endpoint is wired in the client only.

- 2026-07-22 — **Five new `PlanKind` values: `Practice`, `Process`,
  `Purpose`, `Pathway`, `Proposal`** in `src/lib/api/types.ts`, so the
  `PlanForm` kind `<select>` (driven by `ALL_KINDS`) offers them. The
  legacy `COLLECTIONS` segments gain `practices` / `processes` /
  `purposes` / `pathways` for the proposal / idea / report kind-target
  selects; `proposals` is deliberately **not** added there, because that
  segment already names the separate proposals catalogue route.

### Changed — unify the four collections into one recursive `/api/plans` collection + rename to `plan` (2026-07-20)

- **One recursive collection.** The four PPM REST collections
  (portfolios / projects / products / programs) are unified into a single
  recursive `/api/plans` collection: every record is a **plan** under
  `/api/plans`, and matching runs across the whole collection rather than
  being gated by kind.
- **Entity renamed `work item` → `plan`.** `src/lib/api/types.ts` renames
  `WorkItem` → `Plan` (and `WorkItemKind` → `PlanKind`, `WorkItemEvent` →
  `PlanEvent`, `WorkItemIdentifier` → `PlanIdentifier`, `WorkItemRef` →
  `PlanRef`); `kind` is now **optional** (a descriptive label —
  Portfolio / Project / Product / Program — not a collection selector);
  `portfolio_ref` → `parent_ref` (any plan may contain any other).
- **Routes.** The dynamic `[collection]` route directory is collapsed to a
  static `plans/` directory (`/plans`, `/plans/new`, `/plans/[pid]`,
  `/plans/[pid]/{edit,board,schedule,governance}`). The nav's collection
  switcher is gone — there is a single **Plans** destination.
- **API client.** `src/lib/api/work-items.ts` → `src/lib/api/plans.ts`
  exporting `PlanRepository` (no longer collection-bound; all paths under
  `/api/plans`, with the `?parent=` child roll-up and
  `/api/plans/{pid}/schedule`).
- **Form.** `WorkItemForm.svelte` → `PlanForm.svelte`; `kind` is now an
  optional `<select>` (a plan may have no kind); `parent_ref` is an
  optional plan picker for any plan.
- **Tests.** `tests/unit/work-items.test.ts` → `tests/unit/plans.test.ts`
  and `tests/unit/work-item-form.test.ts` →
  `tests/unit/plan-form.test.ts`. `svelte-check` is 0 errors / 0 warnings
  and the vitest suite (45 tests) is green.

### Added — engineering moderate views (2026-07-20)

- Board gains story-point input (+pt on cards), the per-sprint
  retro/feedback notes panel with convert-to-task, and the velocity
  table (team-local note shown). `/engineering` gains the DevOps
  metrics tiles (derivation shown; unresolved incidents counted, never
  timed) and the release register. Six typed client methods + pins;
  e2e stubs + assertions.

### Added — engineering-team views (2026-07-20)

- `/{collection}/{pid}/board` — the per-item task Kanban (drag = the
  PATCH move; the service owns the flow stamps) with sprint
  create/select, the honest burndown table (server derivation shown),
  and the standup digest; linked from the item detail page.
- `/calendar` — the estate milestone calendar (SVAR Calendar) with
  kind filter; `/engineering` — blocked-work aging + MoSCoW scope +
  delivery links. Nav + keys ×13 locales; eleven typed client methods
  (ApiClient gains `patch`); path pins; three stubbed Playwright specs.

### Added — oversight areas (2026-07-20)

- `/board`, `/auditor`, `/compliance`, `/risk`, `/security`,
  `/regulator` pages over the thirteen oversight endpoints (board pack
  + investments + take-snapshot trends; findings + filterable trail +
  evidence-pack CSV; registers; heatmap matrix with honest no-appetite
  note; coarse regulator extract). Nav + `ppm.nav.*` keys ×13 locales;
  twelve typed client methods with path pins; six stubbed Playwright
  specs.

### Added — executive moderate fits (2026-07-19)

- `/executive` gains the strategic-alignment section (coverage per
  collection, unaligned spend, largest unaligned items); `/technology`
  gains the technical-debt register and delivery-flow metrics;
  `/scenarios` gains a side-by-side compare panel (pick two, see
  per-currency deltas and feasibility). Client gains four typed
  methods with vitest path pins; e2e stubs + three extended specs.

### Added — executive areas (2026-07-19)

- `/executive`, `/financials`, `/technology` — CEO / CFO / CTO views
  over the service's new insight endpoints. All numbers are
  server-derived (RAG, variance, realization ratios, radar rings); the
  client formats with `money()` and displays the server's derivation /
  no-FX notes verbatim. Nav + `ppm.nav.*` keys in all 13 locales;
  client gains seven typed methods with vitest path pins; three
  stubbed Playwright specs.

### Added

- 2026-07-19 — SVAR strong fit: new **/gantt** route (nav-linked): the selected portfolio's
  schedule (PPM-6) in the SVAR Gantt — dated items as task bars,
  dependency edges as links, the critical path tinted, undated items
  listed honestly below rather than invented onto the timeline.
  Read-only in v1. One new i18n key (`ppm.nav.gantt`) x 13.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the `/[collection]` index routes (**/portfolios, /projects,
  /products, /programs**) now render the SVAR DataGrid with a
  FilterBar above it (client-side name filtering over the loaded
  refs); row selection opens the work-item detail route.

- 2026-07-19 — Lily Design System: the hand-rolled locale `<select>` is replaced by the Lily
  **LocaleSelect** (wired to the i18n store; `applyDir` off), and
  the **Lily headless** component library is now a dependency
  alongside the existing ThemeSelect.

- 2026-07-18 — **13-locale i18n for the PPM views** (closing the
  English-first follow-up from the catalogue-views round): 98 new
  `ppm.*` keys — nav entries, view titles, table headers, buttons,
  chips, and empty-state lines for the dashboard, intake board,
  ideas, scenarios, objectives, capacity, reports, the governance
  panel, and the schedule view — translated across all 13 locales
  (en, cy, es, fr, de, ar, ru, hi, zh, bn, pt, id, ur) and wired
  through `t()`. The full-coverage parity test enforces every locale
  covers every key; new spot-checks pin PPM strings in zh/ar/hi/ur.
  Wire tokens (statuses, gate ids, currency codes, URN placeholders)
  stay verbatim by design, matching the untranslated collection
  segments in the nav.


### Added

- 2026-07-18 — **PPM catalogue views** (over service Phases A–C,
  PPM-1..12; English-first — extending the 13-locale catalogues to
  these strings is a follow-up). New `PpmClient`
  (`src/lib/api/ppm.ts`, the single source of PPM endpoint paths) and
  routes: `/dashboard` (site tiles + per-collection RAG/stage
  rollups), `/proposals` (intake board: pipeline actions,
  matcher-backed duplicate hits, promote-to-work-item), `/ideas`
  (capture/vote/convert), `/scenarios` (create/evaluate/commit with
  violation chips), `/objectives` (registry + alignment rollups),
  `/capacity` (per-person load meters, over-allocation flags),
  `/reports` (definitions, JSON preview runs, CSV download), the
  per-item **governance panel**
  (`/[collection]/[pid]/governance`: summary strip, gate journey +
  next-gate review form, risks + escalate, budget lines +
  record-actual, benefits + realize + ROI, OKR mappings, milestones,
  allocations), and the portfolio **schedule view**
  (`/portfolios/[pid]/schedule`: CSS Gantt bars, critical-path
  badges, finish-start violation banners). Nav + detail-page links.
  Tests: PpmClient path-mapping + `money` vitest suite; 3 Playwright
  PPM specs over a stubbed API. Verified live end-to-end through the
  BFF proxy against the running service.

### Fixed

- 2026-07-18 — The Playwright smoke suite was a stale copy of the
  case front-end's (asserting "Cases" headings, stubbing `title`
  instead of the work-item `name`, using the case app's detail
  routes) and had never passed against this app; rewritten against
  the real contract — all 8 e2e specs green.

### Changed

- 2026-07-18 — **Subproject renamed**: `portfolio` →
  `project-portfolio-management` (directory, crate/package name, lib
  ident, env-var prefix `PORTFOLIO_*` → `PROJECT_PORTFOLIO_MANAGEMENT_*`,
  database names). The **domain language is unchanged**: the work-item
  kinds (portfolio / project / product / program), the `work_items`
  table, the API routes, and the matcher's `WorkItem` type keep their
  names — the rename repositions the *subproject* as a project
  portfolio management (PPM) product; see the feature roadmap in
  `../spec/15-roadmap.md`.


### Changed

- **De-versioned API URLs.** Dropped the `/api/v1` segment from the work-item client and tests (now `/api/{collection}/…`); the BFF proxy negotiates the API version via the `Accepts-version: 1.0` request header instead (see `agents/share/api-versioning.md`).

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

## [0.1.0] — 2026-06-18

### Added

- **Inaugural spec + docs (spec-only).** First deliverable for
  `project-portfolio-management-front-end-with-svelte`: the living `spec/index.md` (§1–§18)
  and the doc-set (`README.md`, `CLAUDE.md`, `AGENTS.md`, `CHANGELOG.md`,
  `index.md`). No `src/` yet — code is tracked as the spec §13 build
  queue.
  - **Stack decision.** SvelteKit 2 · Svelte 5 runes only
    (`$state` / `$derived` / `$effect` / `$props` / `$bindable`; no
    `export let`, no `$:`) · TypeScript strict (`noUncheckedIndexedAccess`)
    · SPA (`ssr = false`) · SVAR Svelte DataGrid · Lily Design System
    Svelte Headless. Per-project drift accepted — own copy of
    `src/lib/api/{types,client,work-items}.ts` + form primitives; no
    shared package.
  - **Scope.** Consumes the portfolio service REST API under
    `/api/v1/{portfolios,projects,products,programs}/...` — **four
    matchable collections**, one per `WorkItemKind` (Portfolio / Project /
    Product / Program); matching is within a collection only. Identity
    surface per collection: work-item list (SVAR DataGrid), create / edit
    form, detail, name search, duplicate-check with a per-component
    **MatchBreakdown** visual (name, goals, code, owner org, portfolio,
    timeframe, keywords, relationships, tags — `kind` is a hard match
    gate, not a scored component), merge UI, and an audit timeline. A
    Portfolio detail page rolls up its child work items (by
    `portfolio_ref`).
  - **Project-management views.** Kanban task board (Todo / InProgress /
    InReview / Done / Blocked; drag = status change), issues list
    (kind / severity / status), Gantt / timeline view (goal milestones +
    task date ranges), burndown chart (remaining estimate over time), and
    a goals panel. (Posts / comments / members are not part of the v1
    portfolio sub-resource set — roadmap-only.)
  - **Layout shell.** Top navigation bar with a **leftmost hamburger**
    menu (NOT a left sidebar); full-width main content; a collection
    switcher in the chrome. Full theme catalogue via
    `lily-design-system-svelte-theme-select` (selecting a theme restyles
    the whole site). 13-locale i18n (en, cy, es, fr, de, ar, ru, hi, zh,
    bn, pt, id, ur) via `lily-design-system-svelte-locale-select`
    (selecting a locale switches the language; RTL for `ar` / `ur`).
  - **Auth.** Backend-For-Frontend (BFF) + httpOnly cookie session +
    CSRF: **Sign in** runs the central magic-link, which establishes a
    server-side session and sets an httpOnly `__Host-mxi_session` cookie;
    the browser holds **no token** (no `localStorage`, no
    `mxi_access_token`, no URL-fragment handoff). This app's SvelteKit
    server exchanges the session for a short-lived **PASETO v4.public**
    token to call the portfolio service server-side
    (`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`, off by default — family contract
    `agents/share/jwt-enforcement.md`). RS256/JWKS not used. Source of
    truth: `agents/share/authentication-sessions.md`.
  - **Testing plan.** vitest unit (client, repository per collection, BFF
    auth integration, `signInUrl`, `WorkItemForm`, i18n, and a `+layout`
    render test asserting the hamburger toggles the nav) + Playwright e2e
    smoke.
  - **Out of scope (stated).** No FHIR; no consent UI; no finance /
    budgeting UI; no posts / comments feed; no member / role panel; no
    login screen (SSO delegated).

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
- `VITE_AUTH_FRONTEND_URL` (default `http://localhost:5173`).
