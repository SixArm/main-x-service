# Design decisions

Numbered, stable; tasks ([tasks.md](tasks.md)) trace to them.

## CRM-D1 — Consumer application, identities by URN

Contacts wrap `person:` records, accounts wrap `organization:`
records, reps/agents are `worker:` URNs. CRM owns relationship
state only; dedup/merge of identities stays upstream, and wrappers
repoint (manual v1, event-driven roadmap) after upstream merges.

## CRM-D2 — Normalized relational schema

Pipelines, stage positions, SLA clocks, consent history, and money
sums are constraint-heavy — normalized SeaORM tables, not
DTO-as-JSONB. All-plural table names (the loco pluralization
lesson).

## CRM-D3 — Every lifecycle is a pure-core state machine

Lead, deal (incl. terminal immutability + reasoned reopen),
campaign, ticket (incl. reopen), article: one transition table each
in DB-free `rules/` modules, exhaustively unit-tested; controllers
only wire them; illegal transition ⇒ `422` naming the current
state.

## CRM-D4 — Derived numbers, never stored opinions

Score, forecast, ROI, CLV, win rate, SLA deadlines/breaches are
pure-core derivations from recorded facts. No editable KPI fields
exist; snapshots (forecast) are frozen outputs, not inputs.

## CRM-D5 — Explainable scoring

Lead scoring is a fixed deterministic rule table with per-rule
breakdown in the response (the family matcher score-breakdown
posture). Weights config-tunable, rule set fixed in v1, no ML.

## CRM-D6 — Consent gates the send path

Consent is enforced where sends happen (campaign job, nurture
scheduler), not in the UI; segments AND `consent = granted`
structurally; ConsentEvent history is append-only. Unsubscribe
exits everything immediately.

## CRM-D7 — Money in minor units, per-currency honesty

`i64` minor units + ISO-4217; overflow refused; mixed currencies
report per-currency lines, never a silent sum; no FX in v1.

## CRM-D8 — Jobs on `bg_pg`, idempotent by key

Nurture advancement (idempotent per enrolment+step), the SLA breach
sweep (one `sla_breached` per breach fact), and simulated campaign
sends run as loco Postgres-backed jobs — no external broker; the
send is a trait seam for the roadmap ESP adapter.

## CRM-D9 — Transactional integrity

Lead conversion, stage moves, consent changes, and every
audit/outbox write share the mutation's transaction; Kanban
reorders and close/reopen races serialize on the deal row lock
(`FOR UPDATE`).

## CRM-D10 — Personas are policy, not code

One API surface; rep/manager/marketing/support are ABAC profiles
over `attrs` + `resource.owner` (`$sub` ownership) + record attrs;
commercial and channel masking is the `mask` obligation.

## CRM-D11 — Stub-first upstream clients

Display-name lookups behind traits with `http` + `stub`
implementations, config-selected, cached, best-effort — boots and
tests with no siblings running.

## CRM-D12 — Family fixtures from day one

Loco-idiomatic layout, forbid-unsafe + clippy-pedantic, OpenAPI +
Swagger, `Accepts-version`, OTLP + `/metrics.prom`, Podman, input
caps, `404` mapping at `find_by_pid` call sites, enforcement tests
in their own binary (the OnceLock lesson), ETag-conditional
dashboards, 13-locale i18n in the front-end from the start (the
PPM lesson).

## CRM-D13 — Declared data is never inferred

Stakeholder role, power–interest grid position, and activity
sentiment (CRM-R19, [requirements.md](requirements.md)) are
operator-entered fields with no scoring or NLP behind them: a
rating with no entry stays `null` and renders as undeclared, never
defaulted or guessed at from other data. This mirrors CRM-D4
(derived numbers are never stored opinions) from the other
direction — where CRM-D4 keeps a computed number from being typed
in as an opinion, CRM-D13 keeps a human judgement from being
computed. The distinction matters for the insight/engagement views
(CRM-R18/R19): every one of them is either a pure derivation over
recorded facts, or a verbatim echo of a declared field — never a
blend of the two presented as one number.

## CRM-D14 — Erasure gates on live engagement, not on `Contact::status`

WPM's analogous rule (WPM-D22) gates employee erasure on the
employment `status` field, because that field genuinely transitions
through an offboarding lifecycle and *is* the lawful basis. CRM's
`Contact::status` (`active`/`inactive`) is not that: it is set once at
creation and no endpoint in this crate ever transitions it, so gating
erasure on it would refuse erasure forever. CRM-R20's `erasable`
predicate instead checks the signals that actually change over a
contact's lifetime and actually represent a live reason to keep the
data: an open deal naming the contact primary contact, an open
support ticket, or an active nurture enrolment. Erasure anonymises
(tombstone `person:` URN, scrubbed identity fields and linked free
text, soft-deleted row) rather than deletes, mirroring WPM-D22's
"neither pretends" framing: deals/tickets/consent history remain for
reporting and compliance-evidence continuity, keyed to a pid that no
longer identifies anyone, and the retention sweep (floored horizon,
`CRM_RETENTION_DAYS`) is the separate, generic hard-delete path for
soft-deleted rows past that horizon — it never bulk-anonymises a
contact, because a contact carries the erasure gate above and the
sweep has no way to evaluate it per row.
