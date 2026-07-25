# Design decisions

Numbered, stable; tasks ([tasks.md](tasks.md)) trace to them.

## WPM-D1 — Consumer application, identities by URN

WPM is the operational employment layer over the family registries:
`person:` (the human), `worker:` (professional identity),
`organization:` (employer), `course:`/`courseinstance:` (training).
It stores URNs + cached display names, never demographic copies.
The registry's `employed_by` link stays identity-level; emitting it
from hire/termination is roadmap ([integrations.md](integrations.md)).

## WPM-D2 — Normalized relational schema

Employment data is constraint-heavy (unique employee numbers,
balances, pipelines, payslip totals), so the schema is normalized
SeaORM tables — not DTO-as-JSONB. All-plural table names (the loco
`create_table` pluralization lesson).

## WPM-D3 — Every lifecycle is a pure-core state machine

Requisition, application, employee status, leave, review, payroll
run: one transition table each in DB-free `rules/` modules,
exhaustively unit-tested; controllers only wire them. Illegal
transition ⇒ `422` naming the current state (the patient-flow
pattern).

## WPM-D4 — Money in minor units

`i64` minor units + ISO-4217 everywhere (salary, bands, benefit
costs, payslips, benchmarks); arithmetic refuses overflow; no floats.
Payslip reconciliation (`net = gross − Σ deductions`) is a pure-core
invariant checked before persist.

## WPM-D5 — Payroll is a derivation, not an editor

`calculate` derives payslips from upstream facts (salary, FTE,
approved overtime, benefit enrolments) with stub tax tables; humans
approve or return to draft — they never edit payslip lines. Approved
runs are immutable; corrections are a new run (roadmap: supplemental
runs).

## WPM-D6 — Personas are policy, not code

One API surface; employee/manager/HR/payroll are ABAC policy
profiles over `attrs` + record-level `resource.*` attrs + `$sub`
ownership. Salary/review/succession masking is the `mask` obligation,
not bespoke endpoints.

## WPM-D7 — Sensitive reads are audited

Beyond the family mutation-audit baseline: reads of salary-bearing
records, payslips, review content, and succession plans write audit
rows ([audit.md](audit.md)) — the GDPR-accountability substrate.

## WPM-D8 — Consent-bounded candidate pool

Candidates carry `consent_until`; expiry removes them from search
and queues them for purge. Data minimisation is a design property,
not a policy afterthought.

## WPM-D9 — Transactional integrity

Hire (application → employee), leave approval (decision + balance),
payroll approval, and every audit/outbox write share the mutation's
transaction; approval races serialize on row locks (`FOR UPDATE`).

## WPM-D10 — LMS = enrollments over course-service

No course content in WPM: TrainingEnrollment references the family
course registry; WPM adds only status, completion, and certificate
expiry — the thinnest LMS that satisfies WPM-R11.

## WPM-D11 — Stub-first upstream clients

Display-name lookups behind traits with `http` + `stub`
implementations, config-selected, cached, best-effort — the service
boots and tests with no siblings running (patient-flow WPM-D
precedent).

## WPM-D12 — Family fixtures from day one

Loco-idiomatic layout, forbid-unsafe + clippy-pedantic, OpenAPI +
Swagger, `Accepts-version`, OTLP + `/metrics.prom`, Podman, input
caps, `404` mapping at `find_by_pid` call sites, enforcement tests
in their own binary (the OnceLock lesson), 13-locale i18n in the
front-end from the start (the PPM lesson).

## WPM-D13 — Assessment categories are a closed vocabulary with one deliberate overlap

The category↔scale mapping is data
(`rules::assessment::category_permits`), not convention: a result on a
scale the category does not measure is a `422`. The single exception
is `psychometric`, which accepts aptitude and personality scales
because a psychometric test covers both by definition. Without this
rule the profile views would silently mix families and the "what has
not been assessed" list would be meaningless.

Assessment scores are **integers** (percentile 0–100; whole raw
points out of a whole maximum) — the same discipline as money, for
the same reason: a stored float invites rounding drift into a record
someone is judged by. The only float is a reported *mean*, and it is
always accompanied by its numerator and denominator.

## WPM-D14 — Development plans report claimed and verified progress separately

Marking a plan item `achieved` is a claim by a manager; the
employee's **declared proficiency** reaching the target is evidence.
WPM computes and returns both
(`rules::talent::plan_progress` / `verified_progress`) rather than
letting one stand in for the other, and completing an item never
mutates the declared proficiency — that stays a separate, evidenced
act.

## WPM-D15 — Regulated-programme obligations are state-machine preconditions

An apprenticeship's off-the-job training hours are a legal
requirement, so completing a placement below the declared minimum is
**refused** (`rules::talent::may_complete_placement`), in the same way
activation is gated on onboarding items (WPM-D5). Recording the
obligation and then not enforcing it would make the record worse than
useless. Withdrawal forces the `withdrawn` outcome so a withdrawn
placement can never be counted as a conversion.

## WPM-D16 — Analytics carry their terms

Every ratio the workforce-intelligence layer returns is
`{numerator, denominator, value}` and is `null` — never `0` — when
the denominator is zero (`rules::talent::ratio`). Denominators are
chosen to be honest rather than flattering: conversion divides by
*completed* placements (a running one has not had the chance to
convert); plan progress keeps *abandoned* items in the denominator.
Every payload also carries a `derivation` string, because a coverage
number computed from declarations is not a measurement of ability and
must not read like one.

## WPM-D17 — Wellbeing prompts hold acknowledgements, never health data

Public-health eligibility can derive from special-category data (an
immunosuppressed cohort qualifies for the NHS shingles vaccine at
50+). WPM refuses that branch by design: entitlement rules may
predicate only on non-clinical facts the platform already holds or
can resolve — age from the upstream person record, role, department —
so a prompt can never disclose *why* someone qualifies beyond
age/role, and the rule vocabulary has no place to put a diagnosis.
The stored artifact is the employee's **acknowledgement** of a prompt
(`booked | done | declined | dismissed`) — an HR workflow fact — not
a vaccination status, which is clinical and stays out of WPM
entirely. Acknowledgements are `$sub`-owned; HR sees aggregate counts
only (WPM-D16 terms); managers see nothing.

## WPM-D18 — Awareness prompts signpost; they never enrol

The benefits-awareness generalisation (WPM-R26) is one engine with a
`kind` label, not a second prompting mechanism — the predicate and
acknowledgement vocabularies stay closed and shared. A rule tied to a
benefit plan gains no "enrol" action: enrolment stays the WPM-R9
endpoint with its own refusals, and the prompt merely carries the plan
reference. The reverse dependency is **derived, never stored**: an
employee with a live enrolment in the linked plan stops seeing the
prompt, computed per request from `benefit_enrollments`, so awareness
state cannot drift from enrolment truth.

## WPM-D19 — Working-time guardrails advise; they never block

The WPM-R27 signals surface in a read-only view and gate nothing: a
shift assignment that would breach the 11-hour rest gap, or a week
that pushes the 17-week average over 48 hours, is **flagged, not
refused**. Blocking would encode legal judgement WPM cannot make — the
regulations carry opt-outs, sector carve-outs, and compensatory-rest
rules that are a deployment's call, not the platform's. The same
honesty rules as every derived view apply: WPM-D16 terms on the
average, recorded (not merely approved) time in the numerator — a
safety signal must not wait for a manager's approval — and the
derivation named in the payload.

## WPM-D20 — Pulse responses are anonymous by construction, k-floored on read

A pulse answer is only honest if it cannot come back to the author, so
anonymity is structural, not procedural: the response row has **no
author column** — not a nullable one, not a hash. A hashed author
would be pseudonymous (linkable by anyone holding the table), and a
"we promise not to look" column is not a control. Two consequences are
accepted and stated rather than hidden: duplicate submissions cannot
be prevented (the results view counts *responses*, never
*respondents*, and its derivation says so), and the submission audit
row carries **no actor** — the WPM-R16 audit invariant records that a
submission happened, not who made it, because an actor-stamped row
would silently defeat the whole design. On the read side a
**k-anonymity floor (k = 5)** guards the small-cell attack: a
department with fewer than 5 responses would make answers guessable by
elimination, so the cell is marked suppressed and its statistics —
including its response count — are withheld; the overall block obeys
the same floor. The floor is a constant in the pure rules, not
configuration: a deployment that could quietly set k = 1 has no floor.

## WPM-D21 — 360° rater anonymity is procedural, not structural — and says so

The pulse (WPM-D20) achieves anonymity by storing no author. A 360°
appraisal cannot: it must enforce **one response per invited rater**,
show **who has not yet responded** (chasing is half the process), and
scope responses to a nomination's group — all of which require the
response row to link to the nomination. So the guarantee is different
in kind and stated plainly: the link **exists in the store** but no
API surface ever discloses rater-level content — the detail view
shows *who* responded, never *what*; the report shows only group ×
competency aggregates and group-pooled comments. The small-group
attack is answered with a **group floor of 3** (a pure-rules
constant) on `peer` and `report` cells — count withheld below it,
same posture as WPM-D20 — while `manager` and `self` disclose at
n = 1 by convention: a manager's feedback is accountable feedback,
and the self view is the subject's own words. Choosing the pulse's
structural anonymity here would have quietly traded away response
enforcement and completion tracking; choosing silence about the
stored link would have overclaimed. Neither is acceptable — the
trade-off is the design.

## WPM-D22 — Erasure anonymises; retention deletes; neither pretends

Two different rights, two different mechanics. **Erasure** cannot be
deletion in an HR system: payroll and right-to-work records carry
multi-year statutory duties, so deleting them to honour one right
would breach another. Erasure therefore **anonymises**: identity
fields are scrubbed in place (display name, the `person:`/`worker:`
URN links, salary), authored free text is scrubbed, the subject's
appraisals are closed, and the employee row is soft-deleted — while
payslips and payroll rows survive, keyed to an internal pid that no
longer resolves to a person. It is refused while employment is
active: the relationship is the lawful basis. **Retention** is the
opposite mechanic: soft-deleted rows past the horizon are
hard-deleted, and expired-consent candidates are scrubbed — the
candidate pool's duty is to *lose* data. The horizon has a floor
(30 days) precisely because a configurable horizon of zero would
silently turn every soft-delete into a hard-delete. And one thing is
refused honestly: WPM cannot erase what the upstream identity
services hold — the export and erasure cover WPM's own store, and
coordination with person/worker services is the deployment's duty
(stated, not simulated).

## WPM-D23 — Notifications are in-app, reference-only, and event-born

WPM stores no email address or phone number — identities are URNs and
demographics stay upstream — so a "send an email" feature would
either invent a contact-details store (violating the scope boundary)
or silently do nothing. Notifications are therefore **in-app**: rows
written by WPM's own lifecycle transitions in the same handler that
makes the change, listed on the employee's own surface
(`$sub`-owned), marked read by their owner. Two rules keep them
safe: they are **reference-only** — a kind, a neutral body, and pids/
names, never scores, comments, salary, or any masked-tier value
(a notification list is a read path like any other, and the WPM-D21
"no rater-level content on any endpoint" guarantee must survive it) —
and they are **owned data**, so erasure deletes them and the
subject-access export includes them. Outbound channels (email, push,
chat) are a deployment integration over the upstream person service's
contact details, deliberately out of WPM.
