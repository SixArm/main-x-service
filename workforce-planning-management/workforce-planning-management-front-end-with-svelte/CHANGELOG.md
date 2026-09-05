# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — root sign-in gate (WPM-T38)

No `+layout.server.ts` existed anywhere under `src/routes`, so a
visitor with no session reached every page — profile, payroll,
wellbeing, all of them — and only discovered they were signed out once
an API call silently failed through the BFF proxy. New root
`src/routes/+layout.server.ts` redirects to `/signin` (303) when
`locals.sessionId` is `null`, excluding `/signin`/`/verify` themselves.
Chosen over person-front-end's narrower per-mutation-page guard: WPM's
pages mix read content with embedded actions (the retention sweep on
`/privacy`, payroll lifecycle actions, …) rather than separating reads
and writes onto dedicated routes, so a root gate fits WPM's actual page
shape better than a small mutation-only subset would.

This app runs SPA mode (`ssr = false`), so the redirect never appears
in the raw document response — it surfaces through SvelteKit's
client-side data fetch during hydration, confirmed empirically via
`curl .../employees/__data.json` returning
`{"type":"redirect","location":"/signin"}`. `tests/e2e/smoke.spec.ts`
gained a `signIn()` helper (injects a fake `__Host-mxi_session` cookie
via Playwright) and a new `"sign-in gate (WPM-T38)"` describe proving
the redirect; every pre-existing smoke test moved under a
`"signed-in smoke coverage"` describe whose `beforeEach` now signs in
first, verified green (all 12 Playwright + 10 vitest tests pass). See
`../spec/tasks.md` WPM-T38.

### Added — reasonable adjustments panel (WPM-T36 / WPM-R33, 2026-07-25)

- Employee profile: a Reasonable-adjustments panel — the
  barrier / impact / change prompts ("You do not need a diagnosis to
  ask"), practical category select, request list with status chips,
  and agree (with practical note) / decline / in-place actions.
  10 i18n keys × 13 locales.

### Added — ergonomics area (WPM-T34 / WPM-R32, 2026-07-25)

- Employee profile: a DSE assessments panel (open with the default
  checklist, answer ok/issue with an equipment note, complete).
  `/workforce`: an ergonomic-issues table (rota-tier). 7 i18n keys ×
  13 locales.

### Added — notifications panel (WPM-T33 / WPM-R31, 2026-07-25)

- Employee profile: an in-app Notifications panel (unread first) with
  mark-read. 2 i18n keys × 13 locales.

### Added — privacy area (WPM-T32 / WPM-R30, 2026-07-25)

- `/privacy`: the retention report (horizon, per-table past-horizon
  counts, expired-consent candidates, derivation) with the sweep
  button and its result counts.
- Employee profile: an **Erase (anonymise)** action for terminated /
  retired employees (confirm-gated; the record 404s afterwards, so
  the page navigates back to the list). 8 i18n keys × 13 locales.

### Added — subject-access download (WPM-T30 / WPM-R30, 2026-07-25)

- Employee profile: a "Download my data" link opening the audited
  subject-access JSON export. 1 i18n key × 13 locales.

### Added — "My 360 requests" (WPM-T29 / WPM-R29, 2026-07-25)

- Employee profile: a rater-side panel listing pending 360 requests
  (subject + group) with inline per-competency scoring and a comment;
  responding clears the request. 1 i18n key × 13 locales.

### Added — 360° appraisals panel (WPM-T28 / WPM-R29, 2026-07-25)

- Employee profile: a 360° panel — create a draft (default
  competencies), nominate raters by group, drive the lifecycle,
  submit per-competency responses for nominated raters, and read the
  group-floored report (withheld cells shown as "Withheld below 3
  responses"). 8 i18n keys × 13 locales (parity green).

### Added — wellbeing pulse (WPM-T27 / WPM-R28, 2026-07-25)

- Employee profile: a pulse card listing open surveys with 1–5 score
  buttons and an anonymous-thanks state. `/wellbeing`: k-floored
  results blocks per survey (overall + per-department cells, the
  suppressed state shown as "Hidden below 5 responses", derivation
  line). 5 i18n keys × 13 locales (parity green).

### Added — working-time panel (WPM-T26 / WPM-R27, 2026-07-25)

- `/workforce` gains a Working-time panel: flagged employees with the
  over-48h average chip (hours/week) and 11-hour rest-gap breach
  chips, the derivation line, and an explicit all-clear state. 4 i18n
  keys × 13 locales (parity green).

### Added — enrolment conversion on the uptake table (WPM-T25 / WPM-R26, 2026-07-25)

- The `/wellbeing` uptake table shows "Enrolled after prompt" for
  plan-linked rules (rate with its terms; em dash when nobody has
  acknowledged). 1 i18n key × 13 locales (parity green).

### Added — benefits awareness (WPM-T24 / WPM-R26, 2026-07-25)

- The wellbeing engine now signposts benefits too: rule kind
  (`health | benefit`) select on the `/wellbeing` create form, kind
  chips on the rules table and the profile prompt card, and the prompt
  type carries the linked benefit plan. 2 i18n keys × 13 locales
  (parity green).

### Added — wellbeing area (WPM-T23 / WPM-R25, 2026-07-24)

- Employee profile: a **Health entitlements** card listing the
  employee's live prompts (with the one multi-dose reminder flagged)
  and the four acknowledgement actions
  (booked / done / declined / dismiss) — informational only.
- `/wellbeing` HR admin page: the configured entitlement rules
  (cohort shown as age band + departments + titles), a create form,
  soft-close, and the **aggregate-only** uptake table (counts by
  response + rate with its terms; no individual appears).
- Six client functions with path pins; nav + 11 keys in 13 locales
  (parity green); one stubbed Playwright spec (suite now 8).

### Added — learning + mentorship areas (WPM-T20, 2026-07-20)

- `/learning` (skills matrix + gap list, per-department training
  analytics, learning-path progress with a path selector) and
  `/mentorship` (mentor load, unmatched employees, stale mentorships).
  Six client functions with path pins; nav + keys in 13 locales; two
  stubbed Playwright specs.

### Added

- 2026-07-19 — SVAR strong fit: the **/requisitions** board upgrades from custom CSS columns to
  the SVAR Kanban: drag between status columns drives the pipeline
  transition endpoint (the service's state machine still refuses
  illegal moves; the reload puts the card back where the truth says
  it belongs).

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
  symlink; choice persisted to `mxi.wpm.theme`), and the **Lily
  headless** component library is a dependency.

- 2026-07-18 — WPM-T18/T19 implementation round: SvelteKit 2 +
  Svelte 5 runes SPA with same-origin BFF proxy (session → PASETO
  exchange seam; `Accepts-version: 1.0` stamped; no token in browser
  JS), 13-locale i18n (48 keys, parity-tested, RTL ar/ur), typed API
  client + honest `money()`, and the pillar views (dashboard,
  employees + profile, org chart, requisition board + pipeline,
  workforce approvals + rota, development, payroll runs + detail,
  benchmarks). svelte-check clean; 5 vitest + 4 Playwright
  (`page.route`-stubbed) tests green.

- 2026-07-18 — WPM-T0 specification round: cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is WPM-T18/T19 in the queue.
