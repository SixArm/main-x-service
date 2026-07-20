# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

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
