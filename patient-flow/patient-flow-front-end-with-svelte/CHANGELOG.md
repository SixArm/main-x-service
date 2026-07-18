# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 2026-07-18 — PF-T17/PF-T18 follow-through: the whiteboard poll is
  now an **ETag conditional GET** (`If-None-Match`; a `304` keeps the
  current render — an idle wall screen costs no body bandwidth; the
  proxy forwards `etag` both ways), and the **BFF session flow**
  landed (copy-adapted from the case front-end): `/signin` magic-link
  request, `/verify` server-side token exchange → httpOnly
  `__Host-mxi_session` cookie, `/signout`, and the proxy exchanges
  the session for a short-lived PASETO bearer. Inert until
  `PATIENT_FLOW_REQUIRE_AUTH` + the auth service are deployed; no
  token ever reaches browser JS.
- 2026-07-18 — PF-T15/T16 implementation round: SvelteKit 2 + Svelte
  5 runes SPA (drift-accepted, copy-adapted from the case front-end)
  with a same-origin BFF proxy (`/api/proxy/*`, `Accepts-version`
  stamped, PF-T18 seam for the PASETO exchange). Routes: home ward
  list, ward whiteboard with polled bed cards (state colours, EDD /
  CCD / pathway / Red2Green / DTOC / infection / alert chips,
  clean-cycle actions), chrome-less kiosk mode (`?masked=1` corridor
  rendering), stay detail (SAFER fields, Red2Green recording,
  infection flags, transfer, discharge-ready, discharge), hospital
  at-a-glance tiles + ward table, bed-request board (queue, ranked
  eligible beds, allocate, cancel, new-request form), patient
  locate, audits + ward handover filter. Tests: 22 vitest `BedCard`
  state × flags cases + 7 Playwright e2e specs over a
  `page.route`-stubbed API; `svelte-check` 0 errors.
- 2026-07-17 — PF-T0 specification round: edition doc scaffold;
  target routes and the bed-card contract fixed in the cross-cutting
  spec ([../spec/whiteboard.md](../spec/whiteboard.md)). No code yet
  — implementation is PF-T15/T16 after the service phases.
