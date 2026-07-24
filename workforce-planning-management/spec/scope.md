# Scope

## In scope (v1)

- **Employee records** — the employment relationship (hire →
  active → leave → offboarding → terminated/retired), employment
  facts (type, FTE, department, job title, manager chain, salary in
  minor units), and the org chart derived from the manager chain.
- **Talent acquisition** — requisitions with a pipeline, applications
  with stages + interview records, the candidate pool, offer →
  hire conversion, onboarding checklists.
- **Workforce management** — time entries with overtime derivation,
  leave entitlements/balances/requests with approval flow, shifts
  with assignment + conflict checks.
- **Benefits** — plans and per-employee enrollments.
- **Talent development** — review cycles, goals, feedback,
  appraisals; training enrollments referencing `course:` /
  `courseinstance:` URNs; succession plans with readiness ratings.
- **Payroll & compensation** — payroll runs producing payslips
  (gross/deductions/net, minor units), salary benchmarks and
  under/over-market flags.
- **Self-service + ABAC personas** — employee / manager / HR /
  payroll scopes as policy; salary + payslip masking.
- **Audit + events** — family conventions; sensitive reads audited.

## Out of scope (v1, some on the [roadmap](roadmap.md))

- Jurisdiction-complete tax/statutory calculation (stub tables only).
- External job-board posting, e-signature, and background-check
  vendor integrations (checklist items record outcomes, not vendors).
- Biometric / hardware clock integrations (time entries via API/UI).
- Payroll payment execution (bank files) — runs end at "approved/paid"
  status + payslips.
- Learning content authoring — course-service owns courses.
- Union/CBA rule engines; multi-country labor-law engines.

## Boundary with the family

| Concern | Owner | WPM holds |
|---|---|---|
| Person identity (name, contacts, demographics) | person-service | `person:<pid>` EntityRef |
| Professional identity, registrations | worker-service | `worker:<pid>` EntityRef |
| Employer organization | organization-service | `organization:<pid>` EntityRef |
| Courses / course instances | course-service | `course:` / `courseinstance:` EntityRefs on enrollments |
| Sign-on, tokens, ABAC attrs | authentication-service | verified PASETO claims |
| Employment relationship + all operational HR state | **WPM** | its own PostgreSQL tables |

The registry's `employed_by` (worker → organization) cross-service
edge is the *identity-level* assertion of employment; WPM's Employee
record is the *operational* record. WPM can emit/refresh the
`employed_by` edge on hire/termination (roadmap), but the two are
deliberately separate layers — see
[integrations.md](integrations.md).
