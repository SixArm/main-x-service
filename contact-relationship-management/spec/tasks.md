# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (CRM-D*) and requirement (CRM-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] CRM-T0 Cross-cutting spec round: topic files + SDD trio, both
  edition doc scaffolds, root AGENTS.md wiring. (all CRM-D*, CRM-R*)
  — landed 2026-07-18. No code.

## Phase 1 — service skeleton & relationship layer (CRM-R1, CRM-R2, CRM-R17)

- [x] CRM-T1 Scaffold `contact-relationship-management-service-with-rust`:
  loco app, config, migration crate, family fixtures (forbid-unsafe,
  tracing/OTLP, `/metrics.prom`, OpenAPI + Swagger, `Accepts-version`
  middleware, health routes). (CRM-D12)
- [x] CRM-T2 Contact + Account + Activity migrations/models/CRUD:
  URN validation, ownership, soft delete, the merged timeline read,
  the manual repoint endpoint, audit + event seam
  (`CRM_EVENT_TRANSPORT=memory`). (CRM-D1, CRM-D2, CRM-D9; CRM-R1,
  CRM-R2, CRM-R16)
- [x] CRM-T3 Upstream client seam: person / organization / worker
  traits + `http` + `stub`, config-selected; display-name cache;
  stub-mode boot test. (CRM-D11)
- [x] CRM-T4 Seed task: synthetic book of business (~50 contacts,
  ~15 accounts, 2 pipelines, ~30 deals, leads, a campaign, ~20
  tickets) — synthetic data only. (CRM-R17)

## Phase 2 — sales automation (CRM-R3–R5)

- [x] CRM-T5 Lead lifecycle + pure-core scoring with per-rule
  breakdown + hot/warm labels + score-sorted queue; conversion
  (contact + optional deal) in one transaction. (CRM-D3, CRM-D5,
  CRM-D9; CRM-R3)
- [x] CRM-T6 Pipeline + PipelineStage + Deal: stage-move validation,
  Kanban ordering with `FOR UPDATE` serialization, terminal
  close/lost-reason/reasoned-reopen, stalled derivation. (CRM-D3,
  CRM-D7, CRM-D9; CRM-R4)
- [x] CRM-T7 Forecast: pure-core stage-weighted arithmetic,
  per-currency grouping, overflow refusal; month-end
  ForecastSnapshot job. (CRM-D4, CRM-D7, CRM-D8; CRM-R5)

## Phase 3 — marketing automation (CRM-R6–R9)

- [x] CRM-T8 Consent: `marketing_consent` + append-only ConsentEvent
  history + unsubscribe cascade (exit nurture, block sends);
  consent-history read audit. (CRM-D6; CRM-R6, CRM-R16)
- [x] CRM-T9 Segments: declarative filter model + pure-core
  evaluation with the structural consent AND-gate + preview
  (count + sample). (CRM-D6; CRM-R7)
- [x] CRM-T10 Campaigns: lifecycle machine, simulated send `bg_pg`
  job behind the ESP trait seam, touch activities + engagement
  counters, funnel + ROI derivation (zero-cost `null`). (CRM-D3,
  CRM-D4, CRM-D7, CRM-D8; CRM-R8)
- [x] CRM-T11 Nurture: sequence/step/enrollment models + the
  idempotent scheduler job (advance due steps, complete, exit
  rules). (CRM-D8; CRM-R9)

## Phase 4 — service & support (CRM-R10–R12)

- [x] CRM-T12 Tickets: lifecycle machine, assignment,
  first-response stamping from assignee activities. (CRM-D3;
  CRM-R10)
- [x] CRM-T13 SLA: policy model, pure-core deadline derivation +
  re-derivation on audited priority change, breach computation on
  read + the once-per-breach sweep job. (CRM-D4, CRM-D8; CRM-R11)
- [x] CRM-T14 Knowledge base: article lifecycle + versioning on
  published edits + ILIKE search + ticket-link activity. (CRM-D3;
  CRM-R12)

## Phase 5 — analytics (CRM-R13, CRM-R14)

- [x] CRM-T15 Dashboards: win rate, pipeline by stage, SLA health,
  CLV, activity feed + per-rep counts — pure-core derivations,
  ETag-conditional, `as_of`, per-currency, honest ratios. (CRM-D4,
  CRM-D7, CRM-D12; CRM-R13, CRM-R14)

## Phase 6 — auth activation surface (CRM-R15, CRM-R16)

- [x] CRM-T16 `auth.rs`: offline PASETO verify + blanket
  `CRM_REQUIRE_AUTH` guard (guard-all / deny-unless-public) + ABAC +
  record-level `resource.owner`/`status`/`tier` attrs + `$sub`
  ownership + `mask` obligation on amounts/forecast/ROI/channels;
  sensitive-read audit wiring; persona test matrix in its own
  enforcement binary. (CRM-D10, CRM-D12)

> Phases 1–6 landed 2026-07-18 in one implementation round
> (`contact-relationship-management-service-with-rust`, copy-adapted
> from the HCM service): 6 migrations (17 domain tables + audit +
> outbox), pure `rules/` core (lifecycle tables for lead / campaign /
> ticket / article, deterministic scoring with per-rule breakdown +
> linear recency decay, forecast/ROI/CLV/win-rate arithmetic with
> per-currency honesty + null-on-zero-denominator ratios, SLA
> deadline + breach derivation, segment evaluation with the
> structural consent AND-gate), five module controllers, `auth.rs`
> with `resource.owner` `$sub` ownership + `mask_json` amount
> nulling, ETag-conditional dashboards. 62 DB-free unit tests, 5
> request tests (sales journey incl. forecast-follows-stage +
> closed-immutability + reasoned reopen, 404/pipeline-membership
> pins, consent-gated marketing journey incl. withdraw-exits-nurture
> + send-time re-check, ticket SLA journey incl. assignee-only
> first-response + priority re-derivation + sweep idempotency, KB
> versioning), 1 enforcement binary — all green against Postgres 18;
> clippy-pedantic clean; live smoke verified (seed → forecast → ETag
> 304 → win rate 2/4 → OpenAPI 44 paths).
> Notes: the nurture advance + SLA sweep run via POST endpoints in
> v1 (the `bg_pg` periodic worker is the documented roadmap seam);
> deal stages are pipeline data, not a token table.

## Phase 7 — front-end (all CRM-R*)

- [x] CRM-T17 Scaffold
  `contact-relationship-management-front-end-with-svelte`:
  SvelteKit 2 + Svelte 5 runes SPA, BFF proxy + session flow,
  13-locale i18n from the start, typed API client + `money()`.
  (CRM-D12)
- [x] CRM-T18 Views: contact/account timeline, lead queue with
  score breakdown, deal Kanban, forecast table, campaign funnel +
  ROI, nurture editor, ticket queue with SLA countdowns, KB editor,
  dashboards; vitest + `page.route`-stubbed Playwright. (CRM-D10,
  CRM-D12)

> CRM-T17/T18 landed 2026-07-18: SvelteKit 2 + Svelte 5 runes SPA
> (copy-adapted from the HCM front-end: BFF proxy + session seam)
> with a 45-key × 13-locale i18n module (parity-tested, RTL ar/ur),
> typed CRM client + honest `money()`, and views: KPI dashboard
> (win rate with numerator/denominator, forecast per currency),
> contacts + consent actions + timeline, score-sorted lead queue
> with expandable per-rule breakdown, deal board (stage columns +
> forward moves + forecast strip), campaigns with run-simulated +
> funnel/ROI, ticket queue with live breach flags + status moves,
> KB list + publish + search. svelte-check 0 errors; 5 vitest +
> 4 Playwright (page.route-stubbed, unstubbed = 404-loud) green.

## Production gates (before any non-demo exposure)

- [ ] CRM-G1 Activate `CRM_REQUIRE_AUTH` + mount a real ABAC
  policy; verify the persona matrix against the deployment's
  attributes.
- [ ] CRM-G2 GDPR/PECR review of the real send path (ESP adapter,
  lawful basis, unsubscribe in-message), retention schedules,
  subject-access/erasure flows ([regulatory.md](regulatory.md)).
