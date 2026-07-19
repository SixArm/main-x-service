# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 2026-07-19 — SVAR strong fit: the **/deals** board upgrades from custom CSS columns to the SVAR
  Kanban: columns are the pipeline's stage rows (probability
  labelled), drag = the stage-move API (a lost target carries a
  reason), and the forecast strip still re-reads the derived number
  after every move.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: SVAR index routes: **/contacts** and **/leads** and **/tickets**
  upgrade from plain tables to the SVAR DataGrid with FilterBars
  (the lead score breakdown and ticket status actions move to a
  selection panel under each grid; the breach banner keeps its
  testid), and a new **/accounts** index route (name / tier /
  industry) joins the nav.

- 2026-07-19 — Lily Design System: the Lily Design System lands in the chrome: the hand-rolled
  locale `<select>` is replaced by **LocaleSelect** (wired to the
  i18n store; `applyDir` off), a **ThemeSelect** offers the full
  45-theme catalogue (stylesheets via the `static/assets/themes`
  symlink; choice persisted to `mxi.crm.theme`), and the **Lily
  headless** component library is a dependency.

- 2026-07-18 — CRM-T17/T18 implementation round: SvelteKit 2 +
  Svelte 5 runes SPA with same-origin BFF proxy, 13-locale i18n
  (45 keys, parity-tested, RTL ar/ur), typed API client + honest
  `money()`, and the module views (KPI dashboard, contacts +
  consent + timeline, lead queue + score breakdown, deal board +
  forecast, campaigns + funnel/ROI, tickets + breach flags, KB).
  svelte-check clean; 5 vitest + 4 Playwright tests green.

- 2026-07-18 — CRM-T0 specification round: cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is CRM-T17/T18 in the queue.
