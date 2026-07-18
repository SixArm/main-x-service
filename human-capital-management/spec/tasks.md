# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (HCM-D*) and requirement (HCM-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] HCM-T0 Cross-cutting spec round: topic files + SDD trio, both
  edition doc scaffolds, root AGENTS.md wiring. (all HCM-D*, HCM-R*)
  — landed 2026-07-18. No code.

## Phase 1 — service skeleton & employee core (HCM-R7, HCM-R17)

- [ ] HCM-T1 Scaffold `human-capital-management-service-with-rust`:
  loco app, config, migration crate, family fixtures (forbid-unsafe,
  tracing/OTLP, `/metrics.prom`, OpenAPI + Swagger, `Accepts-version`
  middleware, health routes). (HCM-D12)
- [ ] HCM-T2 Employee migrations + models + CRUD: URN validation,
  unique employee number per organization, status state machine in
  the pure core, org-chart derivation + cycle refusal, salary in
  minor units, audit + event seam (`HCM_EVENT_TRANSPORT=memory`).
  (HCM-D1–D4, HCM-D9; HCM-R7, HCM-R16)
- [ ] HCM-T3 Upstream client seam: person / worker / organization /
  course traits + `http` + `stub`, config-selected; display-name
  cache; stub-mode boot test. (HCM-D11)
- [ ] HCM-T4 Seed task: synthetic org (~40 employees across
  departments, managers, salaries) — synthetic data only. (HCM-R17)

## Phase 2 — talent acquisition & onboarding (HCM-R1–R3)

- [ ] HCM-T5 Requisition + Candidate + Application + Interview
  migrations/models/controllers; pipeline state machines in the pure
  core; consent-expiry exclusion + purge list; hire-creates-employee
  in one transaction. (HCM-D3, HCM-D8, HCM-D9; HCM-R1, HCM-R2)
- [ ] HCM-T6 Onboarding checklists: templates → OnboardingItem
  instantiation, mandatory-complete-or-waived activation gate with
  recorded reasons. (HCM-D3; HCM-R3)

## Phase 3 — workforce management (HCM-R4–R6)

- [ ] HCM-T7 TimeEntry + approval flow; >24h refusal; FTE-scaled
  overtime derivation in the pure core. (HCM-D3; HCM-R4)
- [ ] HCM-T8 LeaveEntitlement + LeaveRequest: balance arithmetic
  (annual refusal, sick negative-flag), approval decrements balance
  in-tx, `FOR UPDATE` race serialization. (HCM-D3, HCM-D9; HCM-R5)
- [ ] HCM-T9 Shift + ShiftAssignment: double-booking + leave-conflict
  refusal in the pure core; department day-rota view. (HCM-D3;
  HCM-R6)

## Phase 4 — HR service delivery (HCM-R8, HCM-R9)

- [ ] HCM-T10 BenefitPlan + BenefitEnrollment: minor-unit costs,
  eligibility window, double-enrolment refusal. (HCM-D4; HCM-R9)
- [ ] HCM-T11 Self-service surface: ownership (`$sub`) policy pins —
  own record/payslips/balances/shared reviews readable, own
  leave/time writable, others' refused. (HCM-D6; HCM-R8, HCM-R15)

## Phase 5 — talent development (HCM-R10–R12)

- [ ] HCM-T12 ReviewCycle / Review / Goal / FeedbackEntry: review
  state machine (draft → submitted → calibrated → shared),
  author/subject/HR visibility, content-read audit. (HCM-D3, HCM-D7;
  HCM-R10)
- [ ] HCM-T13 TrainingEnrollment over course URNs + the
  expiring-certificates report. (HCM-D10; HCM-R11)
- [ ] HCM-T14 SuccessionPlan + SuccessionCandidate + the gap report
  (criticality ≥ 4 without `ready_now`); read audit. (HCM-D7;
  HCM-R12)

## Phase 6 — payroll & compensation (HCM-R13, HCM-R14)

- [ ] HCM-T15 PayrollRun + Payslip: run state machine, pure-core
  payslip derivation (salary × FTE pro-rating + approved overtime −
  benefit deductions, stub tax tables), `net = gross − Σ deductions`
  invariant, overflow refusal, approved-run immutability. (HCM-D3,
  HCM-D4, HCM-D5, HCM-D9; HCM-R13)
- [ ] HCM-T16 Benchmark rows + comparison view with `below_min` /
  `above_max` flags, payroll/HR-persona gated. (HCM-D4, HCM-D6;
  HCM-R14)

## Phase 7 — auth activation surface (HCM-R15, HCM-R16)

- [ ] HCM-T17 `auth.rs`: offline PASETO verify + blanket
  `HCM_REQUIRE_AUTH` guard (guard-all / deny-unless-public) + ABAC +
  record-level `resource.person`/`department`/`status` attrs + `mask`
  obligation on salary/payslips/reviews; sensitive-read audit wiring;
  persona test matrix in its own enforcement binary. (HCM-D6, HCM-D7,
  HCM-D12)

## Phase 8 — front-end (all HCM-R*)

- [ ] HCM-T18 Scaffold `human-capital-management-front-end-with-svelte`:
  SvelteKit 2 + Svelte 5 runes SPA, BFF proxy + session flow,
  13-locale i18n from the start, typed API client + `money()`.
  (HCM-D12)
- [ ] HCM-T19 Views: requisition/application boards, onboarding
  tracker, team calendar + rota, employee profile + org chart,
  review + enrollment panels, payroll run screen, benchmarking
  table, HR dashboard; vitest + `page.route`-stubbed Playwright.
  (HCM-D6, HCM-D12)

## Production gates (before any non-demo exposure)

- [ ] HCM-G1 Activate `HCM_REQUIRE_AUTH` + mount a real ABAC policy;
  verify the persona matrix against the deployment's attributes.
- [ ] HCM-G2 Retention schedules + subject-access/erasure flows;
  jurisdiction-correct payroll tables; equality-law review of any
  scoring ([regulatory.md](regulatory.md)).
