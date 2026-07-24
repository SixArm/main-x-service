# Human Capital Management — Specification

This directory is the **single source of truth** for the cross-cutting
Human Capital Management (HCM) specification, shared by both editions.
Each subproject's own `spec/` adds stack-specific detail and links back
here.

> ⚠️ **Demo software, not a production HR system.** This project models
> HCM practice for demonstration and integration purposes. It is not
> a payroll system of record, not employment-law advice, and holds no
> real personal data. See [regulatory.md](regulatory.md).

## What this project is

An **all-in-one HR platform** managing an organization's workforce
through the whole employee lifecycle — hiring to retirement. Where
traditional HR software handles administrative record-keeping, HCM
adds **strategic workforce optimization and talent development**:

1. **Talent acquisition & onboarding** — requisitions, an applicant
   tracking pipeline, a candidate pool, digitized onboarding.
2. **Workforce management** — time & attendance, absence/leave,
   shift scheduling.
3. **HR service delivery** — the employee record as the single source
   of employment truth, org charts, self-service, benefits.
4. **Talent management & development** — performance reviews,
   training via the family's course registry, succession planning.
5. **Payroll & compensation** — payroll runs, payslips, salary
   benchmarking.

It is a **consumer application** (the case-folder / patient-flow /
project-portfolio-management shape): it does not register identities
itself. A human is a [person-service](../../person/person-service-with-loco/)
record; their professional identity is a
[worker-service](../../worker/worker-service-with-loco/) record; the
employer is an [organization-service](../../organization/organization-service-with-loco/)
record; training courses live in the
[course-service](../../course/course-service-with-loco/). HCM owns only
the **employment relationship and its operational state**: employee
records, requisitions, applications, time, leave, shifts, benefits,
reviews, enrollments, succession, payroll — always referencing
identities by `EntityRef` URN, never duplicating them.

## Two editions

| Subproject | Role | Stack |
|---|---|---|
| [human-capital-management-service-with-rust](../human-capital-management-service-with-rust/) | Back-end JSON API | Rust, Loco (Axum + SeaORM), PostgreSQL |
| [human-capital-management-front-end-with-svelte](../human-capital-management-front-end-with-svelte/) | HR / manager / self-service UI | SvelteKit 2, Svelte 5 runes, TypeScript |

## Specification (topic files)

| File | Covers |
|---|---|
| [purpose.md](purpose.md) | Problem statement, goals, the five pillars |
| [scope.md](scope.md) | In/out of scope; the boundary with the identity services |
| [domain-model.md](domain-model.md) | Employee, Requisition, Application, TimeEntry, LeaveRequest, Shift, Review, PayrollRun, … |
| [talent-acquisition.md](talent-acquisition.md) | Pillar 1: ATS pipeline, candidate pool, onboarding checklists |
| [workforce-management.md](workforce-management.md) | Pillar 2: time & attendance, absence, scheduling |
| [hr-core.md](hr-core.md) | Pillar 3: the employee record, org chart, self-service, benefits |
| [talent-development.md](talent-development.md) | Pillar 4: reviews, LMS via course-service, succession |
| [payroll-compensation.md](payroll-compensation.md) | Pillar 5: payroll runs, payslips, benchmarking |
| [integrations.md](integrations.md) | Upstream family services; EntityRef URNs; `employed_by` links |
| [auth.md](auth.md) | SSO, ABAC personas (employee / manager / HR / payroll), masking |
| [audit.md](audit.md) | Audit trail, events, sensitive-read logging |
| [architecture.md](architecture.md) | Editions, layering, pure-core rules, persistence |
| [testing.md](testing.md) | Test strategy per edition |
| [regulatory.md](regulatory.md) | Demo status; UK GDPR / employment-records posture |
| [roadmap.md](roadmap.md) | Beyond the v1 queue |
| [glossary.md](glossary.md) | ATS, FTE, LMS, requisition, accrual, … |

## Specification-driven delivery (SDD)

Three lock-step files drive delivery:

- [requirements.md](requirements.md) — numbered requirements (`HCM-R*`)
  with user stories and acceptance criteria.
- [design.md](design.md) — numbered design decisions (`HCM-D*`).
- [tasks.md](tasks.md) — **the live delivery checklist** (`HCM-T*`),
  phased; every task traces to design and requirement ids.

A change starts in `requirements.md`, is shaped in `design.md`, is
queued in `tasks.md`, and only then lands as code in a subproject.
**No code lands without the spec describing it.**

## References

- Sibling consumer apps (the shape this follows):
  [patient-flow](../../patient-flow/spec/index.md),
  [case-folder](../../case-folder/spec/index.md),
  [project-portfolio-management](../../project-portfolio-management/spec/index.md)
- Family contracts: [cross-service-linking](../../agents/share/cross-service-linking.md)
  (the `employed_by` worker→organization edge is a registry v1 kind),
  [authentication-sessions](../../agents/share/authentication-sessions.md),
  [authorization-attributes](../../agents/share/authorization-attributes.md),
  [security](../../agents/share/security.md)
