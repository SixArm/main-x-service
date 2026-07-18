# Design decisions

Numbered, stable; tasks ([tasks.md](tasks.md)) trace to them.

## HCM-D1 — Consumer application, identities by URN

HCM is the operational employment layer over the family registries:
`person:` (the human), `worker:` (professional identity),
`organization:` (employer), `course:`/`courseinstance:` (training).
It stores URNs + cached display names, never demographic copies.
The registry's `employed_by` link stays identity-level; emitting it
from hire/termination is roadmap ([integrations.md](integrations.md)).

## HCM-D2 — Normalized relational schema

Employment data is constraint-heavy (unique employee numbers,
balances, pipelines, payslip totals), so the schema is normalized
SeaORM tables — not DTO-as-JSONB. All-plural table names (the loco
`create_table` pluralization lesson).

## HCM-D3 — Every lifecycle is a pure-core state machine

Requisition, application, employee status, leave, review, payroll
run: one transition table each in DB-free `rules/` modules,
exhaustively unit-tested; controllers only wire them. Illegal
transition ⇒ `422` naming the current state (the patient-flow
pattern).

## HCM-D4 — Money in minor units

`i64` minor units + ISO-4217 everywhere (salary, bands, benefit
costs, payslips, benchmarks); arithmetic refuses overflow; no floats.
Payslip reconciliation (`net = gross − Σ deductions`) is a pure-core
invariant checked before persist.

## HCM-D5 — Payroll is a derivation, not an editor

`calculate` derives payslips from upstream facts (salary, FTE,
approved overtime, benefit enrolments) with stub tax tables; humans
approve or return to draft — they never edit payslip lines. Approved
runs are immutable; corrections are a new run (roadmap: supplemental
runs).

## HCM-D6 — Personas are policy, not code

One API surface; employee/manager/HR/payroll are ABAC policy
profiles over `attrs` + record-level `resource.*` attrs + `$sub`
ownership. Salary/review/succession masking is the `mask` obligation,
not bespoke endpoints.

## HCM-D7 — Sensitive reads are audited

Beyond the family mutation-audit baseline: reads of salary-bearing
records, payslips, review content, and succession plans write audit
rows ([audit.md](audit.md)) — the GDPR-accountability substrate.

## HCM-D8 — Consent-bounded candidate pool

Candidates carry `consent_until`; expiry removes them from search
and queues them for purge. Data minimisation is a design property,
not a policy afterthought.

## HCM-D9 — Transactional integrity

Hire (application → employee), leave approval (decision + balance),
payroll approval, and every audit/outbox write share the mutation's
transaction; approval races serialize on row locks (`FOR UPDATE`).

## HCM-D10 — LMS = enrollments over course-service

No course content in HCM: TrainingEnrollment references the family
course registry; HCM adds only status, completion, and certificate
expiry — the thinnest LMS that satisfies HCM-R11.

## HCM-D11 — Stub-first upstream clients

Display-name lookups behind traits with `http` + `stub`
implementations, config-selected, cached, best-effort — the service
boots and tests with no siblings running (patient-flow HCM-D
precedent).

## HCM-D12 — Family fixtures from day one

Loco-idiomatic layout, forbid-unsafe + clippy-pedantic, OpenAPI +
Swagger, `Accepts-version`, OTLP + `/metrics.prom`, Podman, input
caps, `404` mapping at `find_by_pid` call sites, enforcement tests
in their own binary (the OnceLock lesson), 13-locale i18n in the
front-end from the start (the PPM lesson).
