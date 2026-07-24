# Authentication & authorization

The family stack unchanged
([authentication-sessions](../../agents/share/authentication-sessions.md),
[authorization-attributes](../../agents/share/authorization-attributes.md)):
cookie sessions + BFF for humans, offline PASETO v4.public for
services, blanket guard `WPM_REQUIRE_AUTH` (default **off** — the
family activation gate; any real deployment MUST activate before
exposure, and HR data makes that non-negotiable), shared ABAC engine.

## Personas as policy (not code)

One API; four typical policy personas expressed over `attrs`:

| Persona | Typical rules |
|---|---|
| **employee** | read own record/payslips/reviews (`resource.person = $sub` ownership template); write own leave requests |
| **manager** | read team records (salary masked), approve team leave, run team reviews — scoped by `resource.manager` / department |
| **hr** | full employee lifecycle in their remit (`resource.department`), reviews, succession |
| **payroll** | payroll runs + payslips + salary fields |

## Record-level attributes

Handlers derive per-record attrs for the second ABAC pass:
`resource.department`, `resource.person` (the employee's person URN,
enabling `$sub`-style self rules), `resource.status`. The `mask`
obligation redacts **salary, payslip amounts, and review content**
while leaving employment facts visible — the corridor-screen posture
applied to HR.

## Sensitivity map

| Data | Tier |
|---|---|
| salary / payslips / benchmarks-vs-salary | highest — payroll persona + self |
| review content, succession, background-check outcomes | high — HR + involved parties |
| leave kinds (esp. sick/parental) | high — self + manager + HR |
| employment facts (title, department, dates) | medium |
| requisitions, shift plans | low |

Every read of a highest/high-tier record is **audited** (see
[audit.md](audit.md)).
