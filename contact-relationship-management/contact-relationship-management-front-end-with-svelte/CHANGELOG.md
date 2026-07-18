# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
