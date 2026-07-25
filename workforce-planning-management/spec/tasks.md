# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (WPM-D*) and requirement (WPM-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] WPM-T0 Cross-cutting spec round: topic files + SDD trio, both
      edition doc scaffolds, root AGENTS.md wiring. (all WPM-D*, WPM-R*)
      — landed 2026-07-18. No code.

## Phase 1 — service skeleton & employee core (WPM-R7, WPM-R17)

- [x] WPM-T1 Scaffold `workforce-planning-management-service-with-rust`:
      loco app, config, migration crate, family fixtures (forbid-unsafe,
      tracing/OTLP, `/metrics.prom`, OpenAPI + Swagger, `Accepts-version`
      middleware, health routes). (WPM-D12)
- [x] WPM-T2 Employee migrations + models + CRUD: URN validation,
      unique employee number per organization, status state machine in
      the pure core, org-chart derivation + cycle refusal, salary in
      minor units, audit + event seam (`WPM_EVENT_TRANSPORT=memory`).
      (WPM-D1–D4, WPM-D9; WPM-R7, WPM-R16)
- [x] WPM-T3 Upstream client seam: person / worker / organization /
      course traits + `http` + `stub`, config-selected; display-name
      cache; stub-mode boot test. (WPM-D11)
- [x] WPM-T4 Seed task: synthetic org (~40 employees across
      departments, managers, salaries) — synthetic data only. (WPM-R17)

## Phase 2 — talent acquisition & onboarding (WPM-R1–R3)

- [x] WPM-T5 Requisition + Candidate + Application + Interview
      migrations/models/controllers; pipeline state machines in the pure
      core; consent-expiry exclusion + purge list; hire-creates-employee
      in one transaction. (WPM-D3, WPM-D8, WPM-D9; WPM-R1, WPM-R2)
- [x] WPM-T6 Onboarding checklists: templates → OnboardingItem
      instantiation, mandatory-complete-or-waived activation gate with
      recorded reasons. (WPM-D3; WPM-R3)

## Phase 3 — workforce management (WPM-R4–R6)

- [x] WPM-T7 TimeEntry + approval flow; >24h refusal; FTE-scaled
      overtime derivation in the pure core. (WPM-D3; WPM-R4)
- [x] WPM-T8 LeaveEntitlement + LeaveRequest: balance arithmetic
      (annual refusal, sick negative-flag), approval decrements balance
      in-tx, `FOR UPDATE` race serialization. (WPM-D3, WPM-D9; WPM-R5)
- [x] WPM-T9 Shift + ShiftAssignment: double-booking + leave-conflict
      refusal in the pure core; department day-rota view. (WPM-D3;
      WPM-R6)

## Phase 4 — HR service delivery (WPM-R8, WPM-R9)

- [x] WPM-T10 BenefitPlan + BenefitEnrollment: minor-unit costs,
      eligibility window, double-enrolment refusal. (WPM-D4; WPM-R9)
- [x] WPM-T11 Self-service surface: ownership (`$sub`) policy pins —
      own record/payslips/balances/shared reviews readable, own
      leave/time writable, others' refused. (WPM-D6; WPM-R8, WPM-R15)

## Phase 5 — talent development (WPM-R10–R12)

- [x] WPM-T12 ReviewCycle / Review / Goal / FeedbackEntry: review
      state machine (draft → submitted → calibrated → shared),
      author/subject/HR visibility, content-read audit. (WPM-D3, WPM-D7;
      WPM-R10)
- [x] WPM-T13 TrainingEnrollment over course URNs + the
      expiring-certificates report. (WPM-D10; WPM-R11)
- [x] WPM-T14 SuccessionPlan + SuccessionCandidate + the gap report
      (criticality ≥ 4 without `ready_now`); read audit. (WPM-D7;
      WPM-R12)

## Phase 6 — payroll & compensation (WPM-R13, WPM-R14)

- [x] WPM-T15 PayrollRun + Payslip: run state machine, pure-core
      payslip derivation (salary × FTE pro-rating + approved overtime −
      benefit deductions, stub tax tables), `net = gross − Σ deductions`
      invariant, overflow refusal, approved-run immutability. (WPM-D3,
      WPM-D4, WPM-D5, WPM-D9; WPM-R13)
- [x] WPM-T16 Benchmark rows + comparison view with `below_min` /
      `above_max` flags, payroll/HR-persona gated. (WPM-D4, WPM-D6;
      WPM-R14)

## Phase 7 — auth activation surface (WPM-R15, WPM-R16)

- [x] WPM-T17 `auth.rs`: offline PASETO verify + blanket
      `WPM_REQUIRE_AUTH` guard (guard-all / deny-unless-public) + ABAC +
      record-level `resource.person`/`department`/`status` attrs + `mask`
      obligation on salary/payslips/reviews; sensitive-read audit wiring;
      persona test matrix in its own enforcement binary. (WPM-D6, WPM-D7,
      WPM-D12)

> Phases 1–7 landed 2026-07-18 in one implementation round
> (`workforce-planning-management-service-with-rust`, copy-adapted from
> patient-flow): 7 migrations (23 domain tables + audit + outbox),
> pure `rules/` core (lifecycle tables, leave/time arithmetic,
> org-cycle, payslip arithmetic incl. the net invariant + overflow
> refusal, benchmark flags), 5 pillar controllers + audits/docs/
> metrics, `auth.rs` with `resource.person` `$sub` ownership +
> salary/payslip masking. 71 DB-free unit tests, 7 request tests
> (hire journey, 404/cycle/uniqueness pins, time caps + overtime,
> leave balance journey incl. decided-race pin + cancel-restores,
> shift conflicts, payroll derivation incl. approved-run
> immutability, benchmark flags), 1 enforcement persona-matrix
> binary — all green against Postgres 18; clippy-pedantic clean;
> live smoke verified (migrate → seed 40 employees → org chart →
> version negotiation → 406 → OpenAPI 57 paths → Prometheus).
> Notes: the seed grants entitlements for reports only; `/api/shifts`
> rota attaches assignments per shift; CI workflows deferred
> (nested workflows don't run in the monorepo).

## Phase 8 — front-end (all WPM-R*)

- [x] WPM-T18 Scaffold `workforce-planning-management-front-end-with-svelte`:
      SvelteKit 2 + Svelte 5 runes SPA, BFF proxy + session flow,
      13-locale i18n from the start, typed API client + `money()`.
      (WPM-D12)
- [x] WPM-T19 Views: requisition/application boards, onboarding
      tracker, team calendar + rota, employee profile + org chart,
      review + enrollment panels, payroll run screen, benchmarking
      table, HR dashboard; vitest + `page.route`-stubbed Playwright.
      (WPM-D6, WPM-D12)

> WPM-T18/T19 landed 2026-07-18: SvelteKit 2 + Svelte 5 runes SPA
> (copy-adapted from the patient-flow front-end: BFF proxy + session
> flow + `Accepts-version` stamping) with a dependency-free 48-key ×
> 13-locale i18n module (parity-tested, RTL for ar/ur), typed WPM
> client + `money()` (masked/absent renders an em dash, never 0),
> and views: HR dashboard tiles, employee list (masked salary as
> first-class state) + profile (onboarding/balances/leave/payslips/
> reviews/training), recursive org chart, requisition status board +
> application pipeline with in-row hire, workforce (pending-leave
> approvals + rota), development (gaps/succession/expiring), payroll
> runs + run detail with lifecycle actions, benchmarks + comparison
> flags. svelte-check 0 errors; 5 vitest (money honesty, i18n
> parity, API path map) + 4 Playwright (page.route-stubbed,
> unstubbed = 404-loud) green.

## Production gates (before any non-demo exposure)

- [ ] WPM-G1 Activate `WPM_REQUIRE_AUTH` + mount a real ABAC policy;
      verify the persona matrix against the deployment's attributes.
- [ ] WPM-G2 Retention schedules + subject-access/erasure flows;
      jurisdiction-correct payroll tables; equality-law review of any
      scoring ([regulatory.md](regulatory.md)).

- [x] WPM-T20 (2026-07-20) **Learning & development.** Migration
      `m20260720_000008_learning` (skills catalog + declared
      `employee_skills`, `learning_paths` + steps + `path_enrollments`,
      `mentorships` + `mentorship_sessions`). `controllers/learning.rs`:
      the skills framework (catalog; declared proficiency 1–5 + optional
      target, upsert), learning paths (ordered course steps; idempotent
      enrolment; honest per-member **progress** — a step counts only
      against a _completed_ `training_enrollments` row for its
      `course_ref`), and mentorships (proposed→active→completed lifecycle
      in `rules::learning`; sessions only on an active pairing).
      Derived views: skills matrix + gaps by department,
      training-analytics (completion ratio = completed / non-failed +
      cert-expiry by department), mentorship overview (active pairs,
      mentor load, unmatched active employees, stale actives). Front-end:
      `/learning` (matrix + gaps + analytics + path progress) and
      `/mentorship`. **Acceptance:** proficiency / lifecycle / progress
      pure pins; the seeded L&D round-trip green first run — full
      `--ignored` suite 8/8 vs Postgres 18; clippy pedantic clean;
      svelte-check 0; vitest 5; Playwright 7.

- [x] WPM-T21 (2026-07-23) **Assessments — aptitude / personality /
      psychometric / selection.** Migration `m20260723_000009_assessments`
      (`assessment_instruments` + `assessments` + `assessment_results`).
      `rules/assessment.rs` (pure): the category↔scale map with the
      psychometric overlap (`category_permits`), the lifecycle machine,
      integer score bounds, the band split, currency
      (completed + unexpired), the mean-with-its-terms, and the
      not-assessed gap list. `controllers/assessments.rs`: the instrument
      catalog, sittings against a candidate or employee (application-linked
      sittings must match the candidate), per-scale result upsert with the
      band derived, the lifecycle move (completion requires ≥ 1 result and
      derives `expires_on` from the instrument validity), the derived
      per-subject profile, the hiring view
      (`/api/applications/{pid}/assessments`), and aggregate analytics.
      Sensitivity: `mask` obligation on every read path, unmasked scored
      reads audited, no individual score in the analytics.
      **Acceptance:** 12 DB-free pure/controller pins (mapping,
      psychometric overlap, lifecycle, bands, bounds, currency, masking,
      declared-scale gating, expiry arithmetic that cannot panic) +
      the DB-gated `assessment_round_trip` request suite; `cargo test`
      green (104 unit); clippy pedantic clean.

- [x] WPM-T22 (2026-07-23) **Talent strategy — succession, upskilling,
      reskilling, pipelines, apprenticeships, internships, workforce
      intelligence.** Migration `m20260723_000010_talent`
      (`development_plans` + items, `talent_pipelines` + `pipeline_members`,
      `early_career_programs` + `program_placements`, and
      `succession_plans.risk_of_loss` / `.vacancy_expected_on`).
      `rules/talent.rs` (pure): upskill/reskill target coherence, the plan
      and placement and pipeline machines (including the deliberate
      `ready → developing` regression), the 1–5 step rule, declared vs
      **verified** progress, off-the-job-hours completion gate,
      conversion rate over completed placements only, bench coverage,
      single-point-of-failure (criticality × risk of loss), terms-carrying
      ratios, and tenure buckets. `controllers/talent.rs` (plans,
      pipelines, early careers) + `controllers/intelligence.rs` (the four
      read-only `/api/workforce-intelligence/*` views) + succession
      updates in `controllers/development.rs`
      (`PUT /api/succession-plans/{pid}`,
      `PUT /api/succession-candidates/{pid}` — readiness may go down).
      **Acceptance:** 14 pure rules pins + 3 controller projection pins +
      the DB-gated `development_plans_track_claimed_and_verified_progress`
      and `pipelines_apprenticeships_and_intelligence` request suites;
      `cargo test` green (104 unit, 11 DB-gated); clippy pedantic clean;
      OpenAPI covers every new path (`spec_shape` extended).

- [x] WPM-T23 (2026-07-24) **Wellbeing — health entitlement prompts.**
      Migration `m20260724_000011_wellbeing` (`wellbeing_entitlements`
      rule rows — no column can express a health-status cohort, per
      WPM-D17 — + `entitlement_acknowledgements`, unique per
      employee + entitlement). `rules/wellbeing.rs` (pure): panic-free
      age arithmetic (leap-day pins), eligibility over age band /
      department / job title / active window with **unknown age failing
      a banded rule**, and the prompt machine (one reminder max for
      multi-dose `booked`/`done`; declining is final).
      `controllers/wellbeing.rs`: rule CRUD, the `$sub`-owned
      self-service prompt view (serving the reminder stamps it), the
      audited acknowledgement upsert
      (`booked | done | declined | dismissed`), and the HR
      `/api/wellbeing/uptake` view — aggregate counts only with WPM-D16
      terms; no manager surface. `clients.rs` gains a best-effort
      cached `birth_date` person lookup (never stored). Front-end:
      profile **Health entitlements** card + `/wellbeing` HR admin
      (rules + create + soft-close + uptake), 11 i18n keys × 13
      locales. **Acceptance:** 12 pure pins; DB-gated
      `wellbeing_round_trip` green first run — full `--ignored` suite
      12/12 + enforcement vs Postgres 18 (116 unit); clippy pedantic
      clean; svelte-check 0; vitest 10; Playwright 8; OpenAPI covers
      the five new paths. (WPM-D6, WPM-D7, WPM-D11, WPM-D16, WPM-D17;
      WPM-R25)

- [x] WPM-T24 (2026-07-25) **Benefits-awareness engine.** Migration
      `m20260725_000012_benefits_awareness` generalises
      `wellbeing_entitlements` with a closed `kind`
      (`health | benefit`, defaulted `health` so WPM-T23 rows are
      untouched) and an optional `benefit_plan_pid` (validated live;
      refused on a `health` rule). The predicate and acknowledgement
      vocabularies are unchanged (WPM-D17). A plan-linked prompt
      carries the plan reference, and goes **quiet automatically** for
      an employee with a live enrolment in that plan — derived per
      request from `benefit_enrollments`, never stored (WPM-D18);
      enrolment stays the WPM-R9 endpoint. Rule list gains `?kind=`
      (validated); the uptake rows carry the kind. Front-end: kind
      select + chip on `/wellbeing`, kind chip on the profile card,
      2 i18n keys × 13 locales. **Acceptance:** kind-vocabulary pin;
      DB-gated `benefits_awareness_round_trip` (kind gate, dead-plan
      404, plan-carrying prompt, enrolment-quietens pin, `?kind=`
      filter + 422, kind in uptake with null-not-zero rate) — full
      `--ignored` suite 13/13 vs Postgres 18 (117 unit); clippy
      pedantic clean; svelte-check 0; vitest 10; Playwright 8.
      (WPM-D16, WPM-D17, WPM-D18; WPM-R26)

- [x] WPM-T25 (2026-07-25) **Enrolment conversion in the uptake view.**
      For a plan-linked rule, `GET /api/wellbeing/uptake` also reports
      `enrolment_conversion` — of the **distinct** employees who
      acknowledged the prompt, how many now hold a live enrolment in
      the linked plan — WPM-D16 terms (`null`, never `0`, with nobody
      acknowledged), derived per request from `benefit_enrollments`
      and never stored (WPM-D18); `null` for a rule with no linked
      plan; the derivation string names both formulas. Front-end: a
      conversion cell on the `/wellbeing` uptake table, 1 i18n key ×
      13 locales. **Acceptance:** DB-gated pins (1/2 conversion after
      one acknowledger enrols, health-rule `null`, empty-denominator
      `null`, still no employee pid in the payload) — full suite
      13/13 vs Postgres 18; clippy pedantic clean; svelte-check 0;
      vitest 10; Playwright 8. (WPM-D16, WPM-D18; WPM-R26)
