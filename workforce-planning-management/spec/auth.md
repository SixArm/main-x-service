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

## Activation runbook (WPM-G1)

The shipped default is **wide open** (family posture,
`agents/share/security.md` §4). Activation is a release gate, not a
config tweak:

1. **Mount a policy.** Start from the shipped reference,
   [`config/abac-policy.reference.json`](../workforce-planning-management-service-with-rust/config/abac-policy.reference.json)
   — svc/admin do everything, `payroll=true` reads unmasked,
   `hr=true` writes and reads **masked** (salary stays payroll+self),
   `resource.person = $sub` reads the own record unmasked, and every
   other authenticated caller gets the masked-read fallback. Point
   `WPM_ABAC_POLICY_FILE` at your copy (it hot-reloads on change) or
   inline it via `WPM_ABAC_POLICY`.
2. **Point at the keys.** `WPM_PASETO_KEYS_URL` (boot-fetched +
   refreshed) or `WPM_PASETO_KEYS`; set `WPM_TOKEN_ISSUER` /
   `WPM_TOKEN_AUDIENCE` if they differ from the defaults.
3. **Flip the flag.** `WPM_REQUIRE_AUTH=1` (read once at boot —
   restart to change).
4. **Verify.** `cargo test --test enforcement -- --ignored` runs the
   persona matrix against the reference policy shape: public paths
   open; 401 without a token; masked vs self vs payroll reads;
   destructive ops (`delete`, `/erase`, `/sweep`) admin-only; the
   subject-access export refused to masked callers; 360 report
   comments withheld from masked callers.

Known engine limits a deployment must plan around (stated, not
hidden): employee **self-service writes** need a coarse `write` allow
to pass the blanket guard (the guard evaluates without record
attributes), so grant them via a subject attribute (e.g.
`access=write` for staff) and rely on the record-level `$sub` checks
on ownership-enforcing handlers; department-scoped manager rules are
written per department (`{"manager": ["true"],
"resource.department": ["engineering"]}`) because subject-vs-resource
attribute equality has no template.
