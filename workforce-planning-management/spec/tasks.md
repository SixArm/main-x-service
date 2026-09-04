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
- [x] WPM-T3 Upstream client seam: one generic `EntityRef`-keyed
      resolver over person / worker / organization / course, `http` +
      `stub` via a single `Mode` enum, config-selected; display-name
      + birth-date caches; stub-mode boot test. (WPM-D11)
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

- [~] WPM-G1 Activate `WPM_REQUIRE_AUTH` + mount a real ABAC policy;
      verify the persona matrix against the deployment's attributes.
      **Code side landed as WPM-T31 (2026-07-25)** — the shipped
      reference policy, the activation runbook (spec `auth.md`), and
      the enforcement matrix verifying the reference file itself.
      The remaining act — setting the flag and attributes on a real
      deployment — is operational by design.
- [~] WPM-G2 Retention schedules + subject-access/erasure flows;
      jurisdiction-correct payroll tables; equality-law review of any
      scoring ([regulatory.md](regulatory.md)). **Code side landed as
      WPM-T30 (2026-07-25)**; the remaining items — lawful-basis
      mapping, jurisdiction payroll tables, equality-law review, and
      coordination of subject rights with the upstream identity
      services — are operational/legal work, not code.

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

- [x] WPM-T26 (2026-07-25) **Working-time guardrails.** No new stored
      state: `rules/working_time.rs` (pure — 17-week/48-hour average
      as an integer boundary comparison with WPM-D16 terms, 11-hour
      rest-gap detection over sorted shift intervals with overlap
      clamping and malformed-interval skipping; leap-safe, panic-free)
      + `GET /api/workforce/working-time?department=&as_of=` in the
      workforce controller: per-employee flags over **recorded** (not
      merely approved) minutes in the trailing 17 weeks and rest-gap
      breaches across recent **and planned** assignments (±28 days).
      Advisory only — nothing is refused (new WPM-D19); visibility
      equals the rota's. Front-end: a Working-time panel on
      `/workforce` (flags + all-clear state), 4 i18n keys × 13
      locales. **Acceptance:** 3 pure pins (terms incl. null-not-zero,
      exact 48 h boundary, rest-gap matrix) + the DB-gated
      `working_time_guardrails` request test (over-average flag with
      its terms, 10 h turnaround = one 600-min breach, modest week
      unflagged, department scoping) — full `--ignored` suite 14/14
      vs Postgres 18 (120 unit); clippy pedantic clean; svelte-check
      0; vitest 10; Playwright 8. (WPM-D16, WPM-D19; WPM-R27)

- [x] WPM-T27 (2026-07-25) **Anonymous wellbeing pulse.** Migration
      `m20260725_000013_pulse` (`pulse_surveys` + `pulse_responses` —
      the response row has **no author column**, by design).
      `rules/pulse.rs` (pure): the 1–5 scale, the inclusive survey
      window, and the k-floored aggregation (`K_ANONYMITY = 5`, a
      constant not configuration; a suppressed cell withholds its
      count; clamped, panic-free). Endpoints: survey create/list,
      `POST /api/pulse-surveys/{pid}/responses` ($sub-owned submit;
      identity used to derive the department and enforce ownership,
      then dropped; **actor-less** audit row; no handle returned;
      window-gated `422`), and `GET …/results` (per-department +
      overall cells, suppressed or disclosed with count/distribution/
      mean; derivation states counts are responses, not respondents —
      new WPM-D20). Front-end: pulse submit card (1–5 + thanks state)
      on the profile, k-floored results blocks on `/wellbeing`;
      5 i18n keys × 13 locales. **Acceptance:** 4 pure pins (scale,
      window, k-floor incl. count-withholding, bad-row clamping) +
      the DB-gated `pulse_round_trip` (closed-survey 422, bad-score
      422, 4-response suppression, 5th response discloses, small
      finance cell stays suppressed with count withheld, no employee
      pid anywhere in results, actor-less audit rows) — full
      `--ignored` suite 15/15 vs Postgres 18 (124 unit); clippy
      pedantic clean; svelte-check 0; vitest 10; Playwright 8.
      (WPM-D16, WPM-D20; WPM-R28)

- [x] WPM-T28 (2026-07-25) **360° appraisals.** Migration
      `m20260725_000014_appraisals` (`appraisals` + nominations +
      responses; the response row links to its nomination **by
      design** — procedural anonymity, new WPM-D21). `rules/
      appraisal.rs` (pure): the one-way lifecycle
      (draft → collecting → shared), the group vocabulary
      (`self | manager | peer | report`; external deferred), rater
      bounds (≤ 12; ≥ 3 non-self to collect), score-completeness
      (every declared competency, 1–5, nothing undeclared), the
      WPM-D21 group floor (`peer`/`report` disclose at 3;
      `manager`/`self` at 1 by convention), and the count-carrying
      mean (empty ⇒ `None`, never 0).
      `controllers/appraisals.rs`: create (auto self nomination, in
      one tx), nominate (draft-only, frozen at collecting), status
      gates, `$sub`-owned once-per-rater responses (collecting only),
      the detail view (who responded, never what), and the shared-only
      **report** (group × competency count + mean, group-pooled
      comments sorted alphabetically so ordering reveals no
      submission sequence, withheld cells hide their count; reads
      audited per the WPM-R10 posture; development-facing, not a
      payroll input). Front-end: a 360° panel on the employee profile
      (create / nominate / lifecycle / respond / report), 8 i18n keys
      × 13 locales. **Acceptance:** 4 pure pins (lifecycle, floor
      matrix, score matrix, count-carrying mean) + the DB-gated
      `appraisal_round_trip` (auto-self, closed groups, subject-only-
      self, min-rater gate, frozen nominations, completeness 422s,
      once-per-rater, no rater content on the detail view,
      shared-only report, manager-discloses-at-1, peer-withheld-at-2
      with count hidden, responses closed once shared, audited report
      read) — full `--ignored` suite 16/16 vs Postgres 18 (128 unit);
      clippy pedantic clean; svelte-check 0; vitest 10; Playwright 8.
      (WPM-D3, WPM-D7, WPM-D21; WPM-R29)

- [x] WPM-T29 (2026-07-25) **Rater self-service for 360s.**
      `GET /api/employees/{pid}/appraisal-requests` — the rater's own
      pending requests: `collecting` appraisals where they are
      nominated and unanswered, with subject / group / competencies
      (`$sub`-owned; discloses only that they were invited). Front-end:
      a "My 360 requests" panel on the profile with inline scoring +
      comment (responding clears the request). **Acceptance:** the
      round-trip pins one pending request naming subject/group/
      competencies, an empty list for the non-nominated, and
      responded ⇒ no longer pending — suite 16/16 vs Postgres 18
      (128 unit); clippy pedantic clean; svelte-check 0; vitest 10;
      Playwright 8. (WPM-D21; WPM-R29)

- [x] WPM-T30 (2026-07-25) **Subject rights & retention (the code
      side of WPM-G2).** `rules/privacy.rs` (pure): `erasable`
      (terminated/retired only — the open relationship is the lawful
      basis), the floored retention horizon (`WPM_RETENTION_DAYS`,
      default 365, **floor 30** — a zero horizon would turn soft-
      delete into hard-delete), and the 38-table sweep list (sorted,
      deduped, pinned). `controllers/privacy.rs`:
      `GET /api/employees/{pid}/subject-access` (one audited JSON
      document across every table keyed to the employee, including
      their authored 360 responses; **exclusions named in the
      payload** — pulse responses are structurally impossible, other
      raters' 360 content is third-party, upstream identity records
      are the deployment's coordination duty);
      `POST …/erase` (anonymise per WPM-D22: identity fields scrubbed
      + tombstone `person:` URN, salary nulled, row soft-deleted,
      authored notes/comments/session-notes scrubbed, appraisals-as-
      subject closed, acknowledgements deleted — payroll rows remain;
      refused `422` on an open employment; audited with counts);
      `GET /api/retention` + `POST /api/retention/sweep` (hard-delete
      past-horizon soft-deletes, scrub expired-consent candidates;
      audited with counts). `/erase` and `/sweep` join
      `DESTRUCTIVE_POST_SUFFIXES` (⇒ `access=admin` under
      enforcement). Front-end: a "Download my data" link on the
      profile (1 i18n key × 13 locales). **Acceptance:** 3 pure pins
      (erasable matrix, horizon default/floor, sweep-list soundness) +
      the DB-gated `subject_rights_round_trip` (export gathers the
      footprint + names exclusions + audited; erase refused while
      active, then anonymises via offboarding→terminated with counts
      in the audit snapshot; report floored; empty sweep audited) —
      full `--ignored` suite 17/17 vs Postgres 18 (131 unit); clippy
      pedantic clean; svelte-check 0; vitest 10; Playwright 8.
      (WPM-D7, WPM-D22; WPM-R30, WPM-G2)

- [x] WPM-T31 (2026-07-25) **Auth activation surface (the code side
      of WPM-G1).** Ships
      `config/abac-policy.reference.json` — the spec `auth.md`
      personas as policy: svc/admin everything; `payroll=true`
      unmasked read; `hr=true` write + **masked** read (salary stays
      payroll + self); `resource.person = $sub` self-read unmasked;
      masked-read fallback — plus the **activation runbook** in
      `auth.md` (mount → keys → flag → verify; known engine limits
      stated: self-service writes need a coarse write allow;
      manager scoping is per-department). Two masking gaps found and
      fixed during verification: `subject-access` now **refuses** a
      masked caller (`403` — a full export cannot be "masked"), and
      the 360 report withholds comments (review-content tier) from
      masked callers while keeping the numeric aggregates. The
      enforcement binary now mounts **the shipped reference file
      itself** (`WPM_ABAC_POLICY_FILE`) and extends the matrix:
      payroll-unmasked vs hr-masked reads, subject-access self-200 /
      masked-403, `/erase`+`/sweep` destructive (hr 403; svc sweep
      200; svc erase of an **active** employment still 422 — the
      lawful basis holds regardless of privilege), and the
      masked-report comment withholding. **Acceptance:** enforcement
      matrix green first run; full `--ignored` suite 17/17 + 131 unit
      vs Postgres 18; clippy pedantic clean. (WPM-D6, WPM-D7,
      WPM-D21, WPM-D22; WPM-R15, WPM-G1)

- [x] WPM-T32 (2026-07-25) **Privacy front-end.** The Svelte app
      gains the ops surface for WPM-R30: a `/privacy` page (retention
      report — horizon, per-table past-horizon counts, expired-consent
      candidates, derivation — with the sweep button and its result
      counts) and an **Erase (anonymise)** action on the employee
      profile, shown only for `terminated`/`retired` employees,
      confirm-gated, navigating away after the record 404s. Three
      client functions with a path pin; nav link; 8 i18n keys × 13
      locales. **Acceptance:** svelte-check 0; vitest 10 (path map
      extended); Playwright 9 (new stubbed `/privacy` spec: report
      renders, sweep posts and shows counts). (WPM-R30)

- [x] WPM-T33 (2026-07-25) **360° notifications.** Migration
      `m20260725_000015_notifications` (`notifications`: employee,
      kind, neutral body, reference data, `read_at`). `rules/notify.rs`
      (pure): the closed kinds and the fan-out — `collecting` ⇒ every
      rater (self included; the self-assessment is a task),
      `shared` ⇒ the subject. The appraisal status handler pushes the
      rows (reference-only bodies — never scores or comments; the
      WPM-D21 guarantee survives the bell, new WPM-D23);
      `GET /api/employees/{pid}/notifications` (unread first,
      `$sub`-owned) + `POST /api/notifications/{pid}/read`
      (owner-only). Erasure deletes the employee's notifications
      (`notifications_deleted` in the audit snapshot); the
      subject-access export includes them. Outbound channels are
      stated out of scope — WPM holds no contact details. Front-end:
      a Notifications panel on the profile with mark-read; 2 i18n
      keys × 13 locales. **Acceptance:** fan-out pure pins; the 360
      round-trip pins rater + self request bells, the subject's
      shared bell, no rater content in any bell, and mark-read —
      suite 17/17 vs Postgres 18 (133 unit); clippy pedantic clean;
      svelte-check 0; vitest 10; Playwright 9. (WPM-D21, WPM-D23;
      WPM-R31)

- [x] WPM-T34 (2026-07-25) **Ergonomic (DSE) workstation
      assessments.** Migration `m20260725_000016_ergonomics`
      (`ergonomic_assessments` + `ergonomic_items`).
      `rules/ergonomics.rs` (pure): the default 8-item DSE checklist
      (a test pins that **no item names a symptom** — WPM-D24: the
      workstation, never the body), the every-item-answered completion
      gate (WPM-D15 posture), and the open-issue count.
      `controllers/ergonomics.rs`: create (default or custom
      checklist), answer (`ok`/`issue` + equipment note; open
      assessments only — completed ones freeze), complete (stamps the
      date; audited with the issue count), and
      `GET /api/ergonomics/issues` (rota-tier department report).
      Erasure scrubs item notes + soft-deletes the employee's
      assessments; subject access includes them; both tables join the
      sweep list (now 40, pinned). Front-end: an Ergonomics panel on
      the profile (create / answer / complete) and an issues table on
      `/workforce`; 7 i18n keys × 13 locales. **Acceptance:** 3 pure
      pins + the DB-gated `ergonomics_round_trip` (default checklist,
      completion gate, freeze-after-complete, department issue report
      with note, custom checklist) — suite 18/18 vs Postgres 18
      (136 unit); clippy pedantic clean; svelte-check 0; vitest 10;
      Playwright 9. (WPM-D15, WPM-D24; WPM-R32)

- [x] WPM-T35 (2026-07-25) **Cognitive (IQ-style) testing.** The
      assessment vocabulary (WPM-R20) gains a fifth category,
      `cognitive`, with standard index scales (verbal comprehension,
      working memory, processing speed, spatial reasoning, fluid
      reasoning) — per-scale readings through the existing engine
      (masked reads, audited unmasked reads, distribution-only
      analytics all apply unchanged), **no composite score exists**,
      and `category_permits` refuses cognitive scales on `selection`
      instruments — an IQ scale cannot ride into hiring unreviewed;
      `psychometric` batteries span it. **Acceptance:** the
      `cognitive_category_scales_and_overlap` pin (own scales permit,
      psychometric spans, selection refuses, aptitude refuses) + the
      existing exhaustiveness pins re-derive over the widened
      vocabulary — 137 unit green; clippy pedantic clean. (WPM-D13;
      WPM-R20)

- [x] WPM-T36 (2026-07-25) **Reasonable adjustments
      (neurodiversity-inclusive).** Migration
      `m20260725_000017_adjustments` (`adjustment_requests`: category,
      barrier, impact, adjustment, decision note — **no diagnosis,
      condition, or medical-evidence column exists**, new WPM-D25).
      `rules/adjustments.rs` (pure): the practical suggestion
      categories (a test pins no entry names a condition) and the
      lifecycle (`requested → agreed|declined|withdrawn`;
      `agreed → in_place|withdrawn`; declined is terminal — ask anew,
      each ask stays on the record). `controllers/adjustments.rs`:
      create (`$sub`-owned; all three texts required; the audit row
      records the category, never the words), list (content-tier:
      masked reads keep category + status and withhold the words;
      unmasked reads audited), decide (practical note, audited,
      in-app `adjustment_update` notification carrying category +
      state only). Erasure scrubs the words + soft-deletes; subject
      access includes requests verbatim ("save a copy"); the table
      joins the sweep (41, pinned). **No aggregate reporting surface
      exists** — stated in WPM-R33. Front-end: a Reasonable-
      adjustments panel on the profile with the barrier/impact/change
      prompts and decision actions; 10 i18n keys × 13 locales.
      **Acceptance:** 2 pure pins + the DB-gated
      `adjustments_round_trip` (barrier required, closed practical
      categories, audited unmasked read, agree-with-note → in-place,
      declined-terminal via the machine pin, two notification bells
      with no words, export carries the words verbatim) — suite 19/19
      vs Postgres 18 (139 unit; the assessment category pin updated
      to 5); clippy pedantic clean; svelte-check 0; vitest 10;
      Playwright 9. (WPM-D23, WPM-D25; WPM-R33)

- [ ] WPM-T37 **`require_ref` EntityType coverage.** `validation.rs`'s
      shared `require_ref`/`ref_opt` helper is exercised by
      `controllers/hr_core.rs`, `acquisition.rs`, `development.rs`,
      `payroll.rs`, and `talent.rs` against `EntityType::Worker`,
      `::Organization`, `::Course`, and `::CourseInstance`, but its
      own unit test (`ref_rules`) only ever passes `EntityType::Person`
      (verified: `grep -n "EntityType::" src/validation.rs` shows every
      call in the test using `Person`, while `grep -rn
      "entity_ref::EntityType::" src/controllers/*.rs` shows `Worker`/
      `Organization`/`Course`/`CourseInstance` all in live use). The
      wrong-type/malformed/bad-uuid branches are therefore proven for
      exactly one of the five accepted types. **Acceptance:**
      `ref_rules` (or a new parametrised test) exercises the
      wrong-type-detected branch for at least `Worker`, `Organization`,
      and `Course` in addition to `Person`; `cargo test` green; clippy
      pedantic clean. (WPM-D9)

- [ ] WPM-T38 **Front-end sign-in gate.** No `+layout.server.ts`
      exists anywhere under `src/routes`, and `hooks.server.ts` only
      reads the session cookie into `locals.sessionId` without ever
      redirecting; `api/proxy/[...path]/+server.ts` forwards
      unauthenticated when `WPM_REQUIRE_AUTH` is off and otherwise
      just relays whatever the upstream 401 is (verified: `find
      src/routes -iname "+layout.server.ts"` returns nothing; no
      `redirect(` call exists in any `.server.ts` under `src/routes`).
      A visitor with no session therefore reaches every profile/
      payroll/wellbeing page and only discovers they are signed out
      when an API call silently fails, rather than being redirected to
      sign-in — the page-visit auth guard the repo root `tasks.md`
      WEB-1 finding found on only 5 of 16 family front-ends.
      **Acceptance:** a root `+layout.server.ts` (or per-protected-
      route guard) redirects a visitor with no `locals.sessionId` to
      `/signin` (excluding the public sign-in/verify routes); a
      Playwright spec pins the redirect; svelte-check 0; existing
      vitest/Playwright suites stay green. (WPM-D12)

- [ ] WPM-T39 **Front-end honesty-format module.** The null-ratio /
      no-data rendering rules (training completion %, conversion
      rate, benchmark below/above-min flags, pulse means, appraisal
      count-carrying means) are inlined per-route rather than
      centralised: `src/lib/` has no `format.ts` counterpart to the
      CMS front-end's `$lib/format` (verified: `find src/lib -maxdepth
      2 -type f` lists only `api/`, `components/`, `config.ts`,
      `i18n.svelte.ts`, and `server/`; the em-dash/no-data pattern
      recurs inline in `routes/learning`, `routes/mentorship`, and
      `routes/employees/[pid]`), and `tests/unit/` carries only
      `wpm.test.ts` (money/i18n/paths) and `rename-migration.test.ts`
      — none of the ratio/no-data rendering paths are unit-tested.
      **Acceptance:** a `$lib/format.ts` (or equivalent) centralises
      the null-not-zero ratio/percentage rendering with its own vitest
      suite (zero-denominator, present-value, negative-flag cases);
      the routes that currently inline the logic call it instead;
      svelte-check 0; Playwright green. (WPM-D16)

- [ ] WPM-T40 **List pagination headers.** No controller emits
      `X-Total-Count`/`X-Limit`/`X-Offset`, and list handlers cap with
      a hardcoded `.limit(...)` (e.g. `hr_core.rs`'s `.limit(500)` and
      `.limit(200)`) rather than accepting `?limit=&offset=` (verified:
      `grep -rln "X-Total-Count" src/` is empty; `grep -n "\.limit("
      src/controllers/hr_core.rs` shows the two hardcoded caps) — the
      family-wide contract in `agents/share/restful.md` (headers
      report the true total, `limit` clamps rather than rejects,
      `offset` is bounded per SEC-G7). **Acceptance:** at least the
      employee list and one other high-traffic list endpoint accept
      `?limit=&offset=`, clamp `limit` to a documented `MAX_LIMIT`,
      bound `offset` (400 past it), and set the three response
      headers; a request test pins clamping and the offset bound;
      clippy pedantic clean. (WPM-D9, WPM-R7)
