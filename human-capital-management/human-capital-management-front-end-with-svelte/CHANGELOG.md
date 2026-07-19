# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the **/employees** index upgrades from a plain table to the SVAR
  DataGrid with a FilterBar (number / name / title / department /
  status); masked salaries still render the translated Hidden token;
  row selection opens the profile.

- 2026-07-19 — Lily Design System: the Lily Design System lands in the chrome: the hand-rolled
  locale `<select>` is replaced by **LocaleSelect** (wired to the
  i18n store; `applyDir` off), a **ThemeSelect** offers the full
  45-theme catalogue (stylesheets via the `static/assets/themes`
  symlink; choice persisted to `mxi.hcm.theme`), and the **Lily
  headless** component library is a dependency.

- 2026-07-18 — HCM-T18/T19 implementation round: SvelteKit 2 +
  Svelte 5 runes SPA with same-origin BFF proxy (session → PASETO
  exchange seam; `Accepts-version: 1.0` stamped; no token in browser
  JS), 13-locale i18n (48 keys, parity-tested, RTL ar/ur), typed API
  client + honest `money()`, and the pillar views (dashboard,
  employees + profile, org chart, requisition board + pipeline,
  workforce approvals + rota, development, payroll runs + detail,
  benchmarks). svelte-check clean; 5 vitest + 4 Playwright
  (`page.route`-stubbed) tests green.

- 2026-07-18 — HCM-T0 specification round: cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is HCM-T18/T19 in the queue.
