# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

## [0.1.0] — 2026-06-18

### Added

- **Inaugural spec + docs (spec-only).** First deliverable for
  `portfolio-front-end-with-svelte`: the living `spec/index.md` (§1–§18)
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
    (`PORTFOLIO_REQUIRE_AUTH`, off by default — family contract
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
