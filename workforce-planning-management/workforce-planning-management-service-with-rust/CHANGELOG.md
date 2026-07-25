# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — reasonable adjustments (WPM-T36 / WPM-R33, 2026-07-25)

- Neurodiversity-inclusive adjustment requests (new WPM-D25): the
  **barrier** faced, its **impact** on the work, and the **change**
  that would reduce it — all required, in the requester's own words.
  **No diagnosis, condition, or medical-evidence column exists**: a
  diagnosis is never required to ask, and the schema has nowhere to
  put one. Practical suggestion categories (quieter workspace,
  written instructions, agendas in advance, flexible breaks, clear
  priorities, equipment, schedule, other).
- Lifecycle `requested → agreed|declined|withdrawn`;
  `agreed → in_place|withdrawn`; decisions carry a practical note,
  are audited, and notify the employee in-app (category + state,
  never the words). Content-tier masking: masked reads withhold the
  words; unmasked reads audited. Erasure scrubs; subject access
  includes requests verbatim; the sweep list grows to 41. No
  aggregate reporting surface exists, deliberately.

### Added — ergonomic (DSE) assessments + cognitive testing (WPM-T34/T35, 2026-07-25)

- **Ergonomics (WPM-R32, new WPM-D24)**: workstation assessments with
  the default 8-item DSE checklist (no item names a symptom — the
  workstation, never the body; a test pins it), `ok`/`issue` answers
  with equipment notes, an every-item-answered completion gate, and a
  rota-tier department issues report. Erasure scrubs and closes them;
  subject access includes them; both tables join the sweep (40).
- **Cognitive testing (WPM-R20)**: a fifth assessment category with
  standard index scales (verbal comprehension, working memory,
  processing speed, spatial reasoning, fluid reasoning). No composite
  "IQ number" exists; `selection` instruments refuse cognitive scales
  (no silent hiring use); `psychometric` batteries span them; masking,
  audited reads, and distribution-only analytics apply unchanged.

### Added — 360° notifications (WPM-T33 / WPM-R31, 2026-07-25)

- In-app notifications (new WPM-D23: in-app, reference-only,
  event-born — WPM holds no contact details, so outbound channels are
  a deployment integration, stated not simulated). Moving an
  appraisal to `collecting` notifies every rater (self included);
  `shared` notifies the subject. Bodies carry a neutral line and the
  appraisal reference — never scores or comments.
- `GET /api/employees/{pid}/notifications` (unread first,
  `$sub`-owned) + `POST /api/notifications/{pid}/read` (owner-only).
  Erasure deletes them; the subject-access export includes them.

### Added — auth activation surface (WPM-T31 / WPM-G1, 2026-07-25)

- `config/abac-policy.reference.json` — the `auth.md` personas as a
  shipped, verified policy: svc/admin everything, payroll unmasked
  read, HR write + masked read, `$sub` self-read, masked fallback.
  The enforcement binary now mounts **this file** via
  `WPM_ABAC_POLICY_FILE`, so the runbook and the verification cannot
  drift. Activation runbook added to spec `auth.md` (with the engine's
  known limits stated).
- **Fixed** two masking gaps found during verification:
  `GET …/subject-access` refused (`403`) to masked callers — a full
  export cannot be "masked"; the 360 report withholds comments
  (review-content tier) from masked callers while keeping the numeric
  aggregates (`comments_withheld` flag).
- Matrix extensions: payroll vs HR read masking, subject-access
  self/masked split, `/erase` + `/sweep` destructive gating, and
  svc-erase of an active employment still refused (the lawful basis
  holds regardless of privilege).

### Added — subject rights & retention (WPM-T30 / WPM-R30, 2026-07-25)

- `GET /api/employees/{pid}/subject-access` — everything WPM holds for
  one employee in a single audited JSON document, with exclusions
  named rather than hidden (pulse responses are structurally
  impossible to attribute; other raters' 360 content is third-party;
  upstream identity records are the deployment's coordination duty).
- `POST /api/employees/{pid}/erase` — erasure **as anonymisation**
  (new WPM-D22): identity fields scrubbed (tombstone `person:` URN),
  authored free text scrubbed, appraisals-as-subject closed,
  acknowledgements deleted, row soft-deleted; payroll/financial rows
  remain under statutory retention keyed to a pid that identifies no
  one. Refused while employment is open. Audited with counts.
- `GET /api/retention` + `POST /api/retention/sweep` — soft-deleted
  rows past the horizon are hard-deleted across all 38 soft-deleting
  tables and expired-consent candidates are scrubbed;
  `WPM_RETENTION_DAYS` defaults 365 and **floors at 30** (a zero
  horizon would turn soft-delete into hard-delete).
- `/erase` and `/sweep` are destructive-classified
  (`DESTRUCTIVE_POST_SUFFIXES` grew to 5) ⇒ `access=admin` under
  enforcement. Closes the code side of gate WPM-G2.

### Added — rater self-service for 360s (WPM-T29 / WPM-R29, 2026-07-25)

- `GET /api/employees/{pid}/appraisal-requests` — the rater's own
  pending requests (`collecting`, nominated, not yet responded) with
  subject, group, and competencies; `$sub`-owned; responding clears
  the request. Discloses only what the rater already knows.

### Added — 360° appraisals (WPM-T28 / WPM-R29, 2026-07-25)

- Multi-rater appraisals around a subject employee: declared
  competencies, nominations by group (`self | manager | peer |
  report`; self automatic; ≤ 12 raters; ≥ 3 non-self to start
  collecting), a one-way lifecycle (draft → collecting → shared;
  nominations freeze, responses close at shared), and `$sub`-owned
  once-per-rater responses (every declared competency scored 1–5).
- Rater anonymity is **procedural** (new WPM-D21): the store links a
  response to its nomination (once-per-rater + completion tracking
  need it) but no endpoint serves rater-level content — the detail
  view shows who responded, the shared-only report shows group ×
  competency count + mean and group-pooled alphabetised comments,
  with `peer`/`report` cells withheld below 3 responses (count
  included); `manager`/`self` disclose at 1 by convention. Report
  reads audited; development-facing, not a payroll input.

### Added — anonymous wellbeing pulse (WPM-T27 / WPM-R28, 2026-07-25)

- Surveys (`/api/pulse-surveys`: name, one question, active window)
  and anonymous 1–5 responses. **Anonymous by construction**
  (WPM-D20): the stored row is survey + department + score + date —
  no author column exists; the submission audit row is actor-less; no
  handle is returned to the submitter. Submitting is `$sub`-owned and
  window-gated (`422` on a closed survey).
- `GET /api/pulse-surveys/{pid}/results` — k-floored aggregate
  (k = 5, a pure-rules constant): per-department + overall cells,
  each suppressed (count withheld) or disclosed (count, distribution,
  mean); counts are responses, never respondents, and the derivation
  says so.

### Added — working-time guardrails (WPM-T26 / WPM-R27, 2026-07-25)

- `GET /api/workforce/working-time?department=&as_of=` — advisory
  Working Time Regulations signals derived entirely from data WPM
  already holds: the 17-week average of **recorded** (not merely
  approved) minutes with WPM-D16 terms and the 48-hour flag (integer
  boundary comparison), plus 11-hour rest-gap breaches across recent
  and planned shift assignments (±28 days). Flags only — nothing is
  refused (new WPM-D19); visibility equals the rota's.
- `rules/working_time.rs` (pure): panic-free average/boundary/rest-gap
  arithmetic; overlaps clamp to 0, malformed intervals are skipped.

### Added — enrolment conversion in the uptake view (WPM-T25 / WPM-R26, 2026-07-25)

- `GET /api/wellbeing/uptake` rows for plan-linked rules gain
  `enrolment_conversion`: distinct acknowledgers now live-enrolled in
  the linked plan / distinct acknowledgers, with WPM-D16 terms
  (`null`, never `0`); `null` for rules with no linked plan; derived
  per request, never stored (WPM-D18); still aggregate-only.

### Added — benefits-awareness engine (WPM-T24 / WPM-R26, 2026-07-25)

- `wellbeing_entitlements` generalises with a closed `kind`
  (`health | benefit`; existing rows default `health`) and an optional
  `benefit_plan_pid` (must name a live plan; refused on a `health`
  rule). Predicate + acknowledgement vocabularies unchanged (WPM-D17).
- A plan-linked prompt carries the plan reference and goes quiet
  automatically for an employee with a live enrolment in that plan —
  derived per request from `benefit_enrollments`, never stored
  (WPM-D18); enrolment remains `POST …/benefit-enrollments`.
- `GET /api/wellbeing-entitlements?kind=` filters (unknown kind
  `422`); uptake rows carry the kind.
- Tests: kind-vocabulary pin + the DB-gated
  `benefits_awareness_round_trip` (kind gate, dead-plan 404,
  enrolment-quietens, filter, null-not-zero rate).

### Added — wellbeing health-entitlement prompts (WPM-T23 / WPM-R25, 2026-07-24)

- Migration `m20260724_000011_wellbeing`: `wellbeing_entitlements`
  (configurable rules — name, description, info URL, age band,
  department / job-title lists, dose count, active window; there is
  deliberately no column a health-status cohort could be expressed in,
  per WPM-D17) + `entitlement_acknowledgements` (one row per
  employee + entitlement, `booked | done | declined | dismissed`).
- `rules/wellbeing.rs` (pure): panic-free whole-year age arithmetic
  (leap-day birthdays included), age-band / department / job-title /
  active-window eligibility where an **unknown age fails a banded
  rule**, and the prompt machine (unacknowledged ⇒ prompt;
  `booked`/`done` on a multi-dose course ⇒ exactly one reminder;
  declining is final).
- `controllers/wellbeing.rs`: rule CRUD
  (`/api/wellbeing-entitlements`), the self-service prompt view
  (`GET /api/employees/{pid}/wellbeing-prompts`, employee-owned via
  the `$sub` ownership attrs; serving a reminder stamps it), the
  acknowledgement upsert (audited), and the HR
  `GET /api/wellbeing/uptake` view — **aggregate counts only** with
  WPM-D16 `{numerator, denominator, value}` terms; no individual
  appears and no manager view exists.
- `clients.rs`: best-effort `birth_date` lookup for `person:` refs
  (stub-first, cached, never stored in a WPM table) so age bands can
  evaluate; `prime_birth_date` for tests.
- Tests: 12 pure pins + the DB-gated `wellbeing_round_trip`
  (cohort scoping, unknown-age honesty, primed-DOB age match,
  closed response vocabulary, decline-is-final, the single reminder,
  aggregate-only uptake, audit row, soft-close). OpenAPI covers the
  five new paths (`spec_shape` extended).

### Changed — renamed to workforce planning management (`HCM` → `WPM`, 2026-07-23)

The project, its crate, its env prefix, its ABAC entity, and its
database names moved from *human capital management* / `HCM` to
**workforce planning management** / `WPM`. Compatibility shims cover
everything a running deployment cannot change atomically:

- **Env prefix.** `WPM_*` is read first, falling back to the legacy
  `HCM_*` spelling with a one-off deprecation warning naming the
  replacement (`src/compat.rs::env_var`, wired through every read in
  `auth.rs` / `streaming.rs` / `clients.rs`). This is a safety fix as
  much as a convenience: an `HCM_REQUIRE_AUTH=1` that stopped being
  read would turn **authentication off**.
- **ABAC entity** `"hcm"` → `"wpm"`. A mounted policy whose rules key on
  `entity: "hcm"` is rewritten at load and warn-logged
  (`compat::migrate_policy_entity`). Same reasoning: a stale entity
  condition fails *silently* — the rule stops matching and the decision
  falls through to the default, so a policy that used to deny could
  start allowing with nothing in the logs. Only the `entity` key is
  touched; a subject attribute whose value happens to be `"hcm"` is left
  alone.
- **Front-end storage.** `mxi.hcm.theme` / `mxi.hcm.locale` are adopted
  under the `mxi.wpm.*` keys on a returning user's next visit, so nobody
  loses their theme or language.
- **Database names** changed with no automatic migration — see
  "Upgrading across the 2026-07-23 rename" in the README for the
  `ALTER DATABASE` statements. A deployment that sets `DATABASE_URL`
  explicitly is unaffected.

Both shims are transitional and documented with their removal
condition. Pinned by 5 unit tests in `compat.rs` (prefix mapping,
entity migration in both JSON shapes, narrowness, idempotence,
malformed-policy safety) and 5 front-end tests
(`tests/unit/rename-migration.test.ts`).

### Added — talent strategy: succession, upskilling, reskilling, pipelines, apprenticeships, internships, workforce intelligence (WPM-T22, 2026-07-23)

- **Development plans** (`development_plans` + `development_plan_items`):
  `upskill` (deepen the current role — no target role) vs `reskill`
  (build toward a named different role), enforced rather than
  conventional. Items pair a catalog skill with a
  `current_level -> target_level` step (1-5, strictly increasing), a
  method, and a due date. Plans report **declared** progress (items
  marked achieved) *and* **verified** progress (declared proficiency
  actually reaching the target) — a claim never stands in for the
  outcome.
- **Talent pipelines** (`talent_pipelines` + `pipeline_members`) for
  succession / hiring / early careers / internal mobility. Stages
  `identified -> assessing -> developing -> ready -> placed`, `exited`
  from any open stage, and a deliberate `ready -> developing`
  regression so the bench cannot be overstated. Health counts the live
  pool only.
- **Early careers** (`early_career_programs` + `program_placements`):
  apprenticeships, internships, and graduate schemes. An
  apprenticeship must declare its off-the-job training hours, only an
  `active` placement accrues them, and **a placement cannot be
  completed below the minimum** — the refusal names both numbers.
  Withdrawal forces the `withdrawn` outcome, so it can never be
  counted as a conversion; conversion rate divides by *completed*
  placements only.
- **Succession planning** deepened: `risk_of_loss` and
  `vacancy_expected_on` on the plan, `PUT /api/succession-plans/{pid}`
  and `PUT /api/succession-candidates/{pid}` (readiness may go
  **down**), bench-coverage classification, and the
  single-point-of-failure rule (uncovered at criticality >= 4, or >= 3
  when the incumbent is a high flight risk).
- **Workforce intelligence** (`/api/workforce-intelligence/*`):
  `overview` (headcount, FTE, tenure buckets, spans of control),
  `capability` (declared skill coverage + gaps, plans in flight,
  assessment coverage), `succession` (bench strength + single points
  of failure), `pipelines` (funnel + early-career conversion). Every
  rate carries `{numerator, denominator, value}` and is `null` — never
  `0` — when there is nothing to divide; nothing is imputed; every
  payload names its derivation; no individual's sensitive data
  appears.
- Pure core `rules/talent.rs` (14 tests) + migration
  `m20260723_000010_talent`; DB-gated request suites
  `development_plans_track_claimed_and_verified_progress` and
  `pipelines_apprenticeships_and_intelligence`.

### Added — assessments: aptitude, personality, psychometric, selection (WPM-T21, 2026-07-23)

- **Instrument catalog** (`assessment_instruments`): a named test, its
  category, the scales it reports, its duration and validity. A scale
  outside its category is a `422` — except `psychometric`, which spans
  aptitude **and** personality by definition.
- **Sittings** (`assessments`) against a candidate or an employee,
  optionally tied to an application (a mismatched application /
  candidate pair is refused). Lifecycle
  `scheduled -> in_progress -> completed -> expired` (+ `cancelled`);
  completing requires at least one result and derives `expires_on`
  from the instrument's validity.
- **Per-scale results** (`assessment_results`): whole-number raw /
  max / percentile (0-100) with the band derived
  (`low` < 10, `below_average` < 30, `average` < 70,
  `above_average` < 90, `high` >= 90). Scores are integers — the same
  discipline as money.
- **Derived views**: per-subject profile (current reading per scale,
  the scales *not* assessed, selection-suitability mean), the hiring
  view `GET /api/applications/{pid}/assessments`, and aggregate
  analytics with band distributions but no individual score.
- **Sensitive**: the ABAC `mask` obligation is honoured on every read
  path (scale and band survive; raw scores, percentiles, and
  narratives do not) and unmasked reads of scored results are audited.
- Pure core `rules/assessment.rs` + migration
  `m20260723_000009_assessments`; DB-gated `assessment_round_trip`.

### Added — learning & development (WPM-T20, 2026-07-20)

- Skills framework (catalog + declared employee proficiency 1-5 with
  optional target), learning paths (ordered course steps +
  per-employee enrolment with honest progress from completed training
  enrolments), and mentorships (proposed->active->completed lifecycle
  + session log). Derived views: the per-department skills matrix +
  gaps, training analytics (completion ratio + cert expiry), and the
  mentorship overview (load, unmatched, stale). Migration
  `m20260720_000008_learning`.

### Added

- 2026-07-18 — WPM-T1–T17 implementation round: full Loco service
  (copy-adapted from patient-flow). 7 migrations / 25 tables, pure
  `rules/` core (lifecycle machines for employee / requisition /
  application / leave / review / payroll; leave balances; overtime;
  shift conflicts; org-chart cycle check; payslip arithmetic with
  the `net = gross − Σ deductions` persist gate and overflow
  refusal; benchmark flags), five pillar controllers (hr_core /
  acquisition / workforce / development / payroll) + audits / docs /
  metrics, offline PASETO + ABAC with `resource.person` `$sub`
  ownership and salary/payslip `mask` obligations, sensitive-read
  audits, event seam (memory/outbox), OpenAPI (57 paths) + Swagger,
  `Accepts-version` negotiation, Prometheus gauges, seed task
  (synthetic 40-employee org). 71 unit + 7 request + 1 enforcement
  tests green against Postgres 18; clippy-pedantic clean.

- 2026-07-18 — WPM-T0 specification round: cross-cutting spec
  (`../spec/`) with the five-pillar domain, SDD trio
  (requirements WPM-R1–R17, design WPM-D1–D12, tasks WPM-T*), and
  this edition's doc scaffold. No code yet.
