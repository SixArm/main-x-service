# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (PF-D*) and requirement (PF-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] PF-T0 Cross-cutting spec round: topic files + SDD trio, both
  edition doc scaffolds, root AGENTS.md wiring. (all PF-D*, PF-R*)
  — landed 2026-07-17.

## Phase 1 — service skeleton & topology (PF-R1, PF-R15)

- [x] PF-T1 Scaffold `patient-flow-service-with-rust`: loco app,
  config, migration crate (CI workflows deferred — nested
  workflows don't run in the monorepo), family
  fixtures (forbid-unsafe, thiserror, tracing/OTLP, `/metrics.prom`,
  OpenAPI + Swagger, `Accepts-version` middleware, health routes).
  (PF-D12)
- [x] PF-T2 Migrations + models + CRUD controllers for Site / Ward /
  Bay / Bed with validation → 422, soft delete, audit + event
  emission seam (`PATIENT_FLOW_EVENT_TRANSPORT=memory`). (PF-D2,
  PF-D3, PF-D9; PF-R1, PF-R13)
- [x] PF-T3 Upstream client seam: `src/clients.rs` — one generic
  `EntityRef`-keyed resolver (not per-service traits/modules) with
  `http` + `stub` modes selected by `PATIENT_FLOW_UPSTREAM_MODE`;
  stub-mode boot test; display-name cache. (PF-D11; PF-R15)
- [x] PF-T4 Seed task: synthetic demo hospital — as landed, 1 site
  ("St Elsewhere General"), 5 wards (2 inpatient, 1 assessment, 1
  escalation, 1 virtual), 76 beds (`cargo loco task seed`). (PF-R15)

## Phase 2 — bed states & journey (PF-R2, PF-R4–R6)

- [x] PF-T5 Pure `flow/` core: bed state machine + transition table
  + deep-clean propagation; exhaustive unit tests. (PF-D4; PF-R2)
- [x] PF-T6 Bed state transition endpoints wiring the pure core:
  `FOR UPDATE` locking, `state_since`, audit + event in-tx (the
  double-placement race test moved to PF-T17). (PF-D9; PF-R2, PF-R13)
- [x] PF-T7 Stay lifecycle: admit / transfer / discharge-ready /
  discharge controllers + Transfer rows + LOS derivation (request
  tests moved to PF-T17). (PF-D5; PF-R4, PF-R5, PF-R6)
- [x] PF-T8 SAFER fields + Red2Green journal (day rows, ≤2 coded
  reasons, same-day edit window) + DTOC clock in the pure core.
  (PF-D4; PF-R6, PF-R7)

## Phase 3 — demand, reads & capacity (PF-R3, PF-R8–R10)

- [x] PF-T9 BedRequest queue + pure-core allocation eligibility +
  ranking + override-with-reason; eligible-count on open requests.
  (PF-D4, PF-D7; PF-R3)
- [x] PF-T10 Whiteboard + stay-detail + locate reads (`as_of` stamp;
  locate read-audit; ETag conditional GET deferred to PF-T17).
  (PF-D6; PF-R8, PF-R9, PF-R13)
- [x] PF-T11 At-a-glance + capacity metrics + Prometheus gauges +
  handover audit query (`?ward=&since=`). (PF-D6; PF-R10, PF-R13)

## Phase 4 — infection control & virtual ward (PF-R11, PF-R12)

- [x] PF-T12 InfectionFlag CRUD + transfer restriction + deep-clean
  gate + bay/ward `closed_to_admissions` + reopen precondition +
  IPC counts in capacity. (PF-R11)
- [x] PF-T13 Virtual ward: virtual slots, step-up / step-down /
  direct-admission flows, no-clean cycle, census. (PF-D8; PF-R12)

## Phase 5 — auth activation surface (PF-R14)

- [x] PF-T14 `auth.rs`: offline PASETO verify + blanket
  `PATIENT_FLOW_REQUIRE_AUTH` guard (guard-all / deny-unless-public)
  + ABAC + `resource.ward` record-level checks + `mask` obligation
  on whiteboard/locate; full family auth test matrix. (PF-D10)

> Phases 1–5 landed 2026-07-18 in one implementation round
> (`patient-flow-service-with-rust`): 24 src modules, 5 migrations,
> 64 DB-free unit tests (state machine, allocation rules, journey,
> auth matrix), clippy-pedantic clean, verified end-to-end against a
> live Postgres 18 (migrate → seed → admit → red-green → flag →
> discharge-ready → discharge → deep-clean gate → locate → audits →
> events → version negotiation → Prometheus). Notes: concurrency
> pinning relies on `FOR UPDATE` locking (dedicated race test is in
> PF-T17); `/events/recent` reads the ring or the outbox per
> transport.

- [x] PF-T17 Request-test suite + conditional board reads — landed
  2026-07-18. `tests/requests/{topology,flows,boards}.rs` (8 tests,
  `--ignored`, Postgres per family convention): topology CRUD +
  422s, the full request→allocate→admit→Red2Green→ready→discharge→
  deep-clean journey, the **double-placement race** (two concurrent
  admits ⇒ exactly one 200), whiteboard/at-a-glance shape, locate +
  its `locate_read` audit pin; `tests/enforcement.rs` (own process)
  runs the auth-activation matrix over live routes including the
  **masked whiteboard** under an allow-with-`mask` policy. The
  whiteboard and at-a-glance reads gained weak-**ETag conditional
  GETs** (`If-None-Match` ⇒ `304`, tag excludes `as_of`; delivered
  in place of the `updated_since` alternative), consumed by the
  front-end poll and forwarded by the BFF proxy. CI workflows stay
  deferred (nested workflows don't run in the monorepo). (PF-D9;
  spec testing.md, whiteboard.md)

## Phase 6 — front-end (PF-R8 UI)

- [x] PF-T15 Scaffold `patient-flow-front-end-with-svelte`
  (SvelteKit 2 + Svelte 5 runes + TS strict, SPA mode + same-origin
  BFF proxy, copy-adapted from the case front-end — the closest
  dependency-light sibling; drift accepted), routes: home ward list,
  whiteboard (+ chrome-less kiosk route with `?masked=1`), stay
  detail with SAFER/Red2Green/flag/transfer/discharge actions,
  at-a-glance, bed-request board with ranked eligible beds +
  allocate, locate, audits + handover filter. Landed 2026-07-18.
- [x] PF-T16 vitest `BedCard` state × flags matrix (22 tests) +
  Playwright e2e (7 specs) with the API stubbed via `page.route`
  mirroring the endpoint contract (case-front-end precedent — fails
  loud on contract drift, no Rust service needed); `svelte-check`
  clean. Landed 2026-07-18.
- [x] PF-T15a Follow-on UI, landed 2026-07-19 (no separate PF-R/PF-D;
  same-day extension of PF-T15's route set — see
  [front-end CHANGELOG](../patient-flow-front-end-with-svelte/CHANGELOG.md)):
  Lily Design System chrome (`ThemePicker`/`LocalePicker` in the
  non-kiosk nav, 45-theme catalogue incl. NHS themes, RTL-aware
  locale); a `/wards` index route (SVAR DataGrid + FilterBar); a
  `/edd` route (SVAR Calendar, month view, read-only expected-
  discharge overview). `@svar-ui/svelte-{kanban,gantt,filemanager}`
  are installed as candidate-feature seams with no route yet.
- [x] PF-T18 BFF session + PASETO exchange — landed 2026-07-18,
  copy-adapted from the case front-end: `lib/server/{session,auth}.ts`
  (httpOnly `__Host-mxi_session` cookie helpers; magic-link request /
  verify / signout / token-exchange calls), `/signin` (server action),
  `/verify` (server-side token exchange → cookie → redirect),
  `/signout`, and the proxy now exchanges `locals.sessionId` for a
  short-lived PASETO and injects the bearer. Inert until
  `PATIENT_FLOW_REQUIRE_AUTH` + the auth service are deployed; no
  token ever reaches browser JS. (spec auth.md)

## Phase 7 — cross-service journey (time-based analysis)

- [x] PF-T19 `GET /api/stays/{pid}/time-analysis` — landed 2026-08-24
  ("Add time-based analysis, and the journey links it needs").
  Serves the stitched-journey timeline contract that
  `care-pathway-service`'s `continues_as` link follows across a
  service boundary (its `src/journey.rs`): clock bounds, elapsed
  span, and value-adding time, so a journey that begins on a care
  pathway and continues into an inpatient stay is measured end to
  end. Value-adding time is derived straight from the stay's
  `Red2Green` day classifications — a green day is the value-adding
  time, unclassified days count as non-value-adding (matching the
  denominator rule in
  [`agents/share/time-based-analysis.md`](../../agents/share/time-based-analysis.md)) —
  and the response carries `coverage`/`confidence` so an
  under-classified stay (classification only starts once the board
  is in use) reads as "we do not know" rather than "inefficient". A
  sensitive read like the MDT view: record-level ABAC, audited
  (`stay_time_analysis_read`); the `mask` obligation is deliberately
  not applied since the response carries only durations, no
  identifiers. Covered by `tests/requests/flows.rs`
  (`full_journey_request_to_deep_clean`).
  (src/controllers/stays.rs, src/flow/journey.rs)

## Staff utilisation (permitted 2026-08-25, blocked on inputs)

- [ ] PF-T-U1 **Roster / staff-capacity source**: who is on shift, for
  how long, with non-working time (leave, study leave, non-clinical
  duty) that **subtracts from the denominator** rather than counting as
  idle. Distinct from the `allocation` module, which is **beds**.
- [ ] PF-T-U2 **Effort source**, labelled **asserted** — never inferred
  from bed moves or Red2Green day classifications, which are by-products
  of the work and must stay trustworthy.
- [ ] PF-T-U3 The utilisation figure with the five obligations in
  [`agents/share/time-based-analysis.md` §7.1](../../agents/share/time-based-analysis.md).
  (capacity.md)
  — **Acceptance:** a nurse off for the whole window reports `null` with
  a reason, **not 0%**; a window below the suppression floor is `null`,
  justified as a re-identification control; the response carries
  numerator, denominator and the configuration behind them, and ships
  beside occupancy and the open-request escalation signal; **no endpoint
  returns per-person cycle time, throughput or flow efficiency**, and
  none is derivable by arithmetic from what is returned.

## Follow-on hardening (found 2026-09-04)

- [ ] PF-T20 **OpenAPI doc missing `/api/stays/{pid}/time-analysis`.**
  `src/openapi.rs` is hand-written and its own `spec_shape` test only
  spot-checks a fixed path list; PF-T19 (landed 2026-08-24) added the
  route to `app.rs` and `stays.rs` but never added a `paths` entry here
  — *(verified: `grep -c time-analysis src/openapi.rs` returns `0`)*. A
  FHIR-agnostic client discovering the API via `/api-docs/openapi.json`
  cannot see this endpoint exists. Three-part: add the `paths` entry
  (mirroring the `/api/locate/{person_ref}` sensitive-read shape already
  documented there) and extend `openapi.rs`'s `spec_shape` test to assert
  `/api/stays/{pid}/time-analysis` is present, closing the exact gap that
  let this drift happen unnoticed.
- [ ] PF-T21 **No allowlist test on the time-analysis response shape.**
  `tests/requests/flows.rs::full_journey_request_to_deep_clean` asserts
  the documented fields (`lead_time_ms`, `value_time_ms`, `span_days`,
  `classified_days`, `clock.*`, `confidence`) are *present*, but nothing
  asserts the top-level key set is *exactly* that — so a future change
  could add a per-person rate/throughput field to the response without
  any test failing, silently breaking the family-wide refusal in
  [`agents/share/time-based-analysis.md` §7](../../agents/share/time-based-analysis.md)
  — *(verified by reading the full assertion block in `flows.rs`, lines
  ~99–125: every check is presence-only, none enumerates the object's
  keys)*. This is the same acceptance bar **PF-T-U3** above already
  states for the not-yet-built utilisation endpoint; PF-T19's endpoint
  shipped first and never got the equivalent pin. **Acceptance:** the
  test asserts `timeline.as_object().unwrap().keys()` is a subset of a
  named allowlist (or an exact `BTreeSet` match), so adding an
  undocumented field fails CI rather than shipping silently.
- [ ] PF-T22 **Front-end has no page-visit sign-in guard.**
  `src/hooks.server.ts` (PF-T18) stashes `locals.sessionId` from the
  cookie but nothing redirects an anonymous visitor away from a
  clinical/PII route (whiteboard, stay detail, bed-request board,
  locate) — *(verified: `grep -rn requireSignedIn src` returns zero
  hits, and no `+layout.server.ts` exists anywhere under `src/routes`)*.
  This matches repo `tasks.md` **WEB-1**'s finding that the
  `requireSignedIn` pattern PRO-H10 added to the ten entity front-ends
  never reached any of the five consumer apps. Today's only real gate is
  `PATIENT_FLOW_REQUIRE_AUTH` on the API, which defaults off (per
  `agents/share/security.md` §4), so an anonymous visitor with front-end
  access today sees live patient data. **Acceptance:** a `requireSignedIn`
  helper (or `+layout.server.ts` check on `locals.sessionId`) redirects an
  anonymous visitor to `/signin` on protected routes, preserving the
  existing `?masked=1` kiosk exemption for the whiteboard; one Playwright
  test pins the anonymous-303 behaviour, matching WEB-1's fix pattern
  (a stubbed session cookie makes guarded pages render in the existing
  suite rather than weakening the guard).
- [ ] PF-T23 **Zero test coverage on the BFF session/token-exchange
  code.** `src/lib/server/{session,auth,config}.ts` (PF-T18's
  httpOnly-cookie helpers, magic-link request/verify, and the
  session→PASETO `exchangeToken` call) have no unit tests, and
  `tests/e2e/board.spec.ts` — the only e2e spec file — stubs the proxy
  wholesale (`page.route("**/api/proxy/**", …)` at line 180) rather than
  routing through `src/routes/api/proxy/[...path]/+server.ts` — *(verified:
  `find tests/unit` lists only `bed-card.test.ts`; `grep -rn
  exchangeToken tests/` matches nothing outside the mocked route
  handler)*. The BFF pattern is this app's whole "no token in browser
  JS" security property (`agents/share/authentication-sessions.md` §6);
  it is currently unverified by any test, inert or not. **Acceptance:**
  a vitest suite exercises `session.ts`'s cookie helpers directly and
  `auth.ts`'s magic-link functions against a mocked fetch, and one e2e
  test hits the real `/api/proxy/[...path]` route (auth-service stubbed
  or skipped when unset) instead of intercepting it.

## Production gates (P0 — design-only until a real deployment)

- [ ] PF-T-G1 Clinical safety case (DCB0129/0160) + named CSO.
- [ ] PF-T-G2 IG: DPIA, DSP Toolkit, retention schedule,
  subject-access slice.
- [ ] PF-T-G3 Security activation: REQUIRE_AUTH on, ABAC policy
  mounted, TLS, masked corridor mode verified.
- [ ] PF-T-G4 Resilience: HA deployment + failover test + paper
  fallback procedure.
