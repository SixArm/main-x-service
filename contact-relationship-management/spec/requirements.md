# Requirements

Numbered requirements with user stories and acceptance criteria.
IDs are stable; design decisions ([design.md](design.md)) and tasks
([tasks.md](tasks.md)) trace to them. The four-module map:
CRM-R1–R5 sales automation, CRM-R6–R9 marketing automation,
CRM-R10–R12 service & support, CRM-R13–R14 analytics & reporting,
CRM-R15–R17 cross-cutting, CRM-R18–R19 insight/engagement views
(added CRM-T19/T20, 2026-07-20 — backfilled here so the "every task
traces to a requirement" rule holds for these two).

## CRM-R1 — Contacts & accounts

*As a rep I keep every person and company I talk to, with the whole
history in one place.*

- Contact CRUD wrapping a required `person:` URN (shape-validated;
  display name best-effort); Account CRUD wrapping a required
  `organization:` URN; ownership by `worker:` URN; soft delete.
- The detail read returns the merged relationship **timeline**
  (activities + deals + campaign touches + tickets, chronological).
- A manual repoint endpoint updates a wrapper's URN after an
  upstream registry merge (audited, reasoned).

## CRM-R2 — Activities

*As a rep I log calls, emails, meetings, notes, and tasks against
any relationship object.*

- Activity CRUD attached to contact / account / lead / deal /
  ticket; task kind carries `due_on` + `done`; actor is a `worker:`
  URN; feeds appear in timelines and dashboards.

## CRM-R3 — Leads & scoring

*As a rep I work the hottest leads first, and I can see why a lead
is hot.*

- Lead CRUD with source + optional campaign attribution; lifecycle
  `new → contacted → qualified → converted | disqualified` enforced
  by the pure core (`422` naming the current state on an illegal
  transition).
- Score 0–100 recomputed on every lead change from the fixed rule
  table ([sales-automation.md](sales-automation.md)); the response
  carries the per-rule breakdown; `hot`/`warm` labels at the
  configured thresholds; the lead queue sorts by score.
- Conversion creates/links the Contact and optionally opens a Deal
  in one transaction, emitting `lead_converted`.

## CRM-R4 — Pipelines & deals (Kanban)

*As a rep I move deals through stages; as a manager I see the board
truthfully.*

- Pipeline + ordered stages with probabilities and terminal
  won/lost flags; Deal CRUD with amount in minor units + ISO-4217.
- Stage moves validate pipeline membership, keep `kanban_position`,
  audit + emit; entering a terminal stage closes the deal (lost
  requires a reason); closed deals immutable except a reasoned
  reopen to the prior stage; concurrent reorders serialize (one
  winner).
- Stalled flag derived at N days (config, default 14) without stage
  move or activity.

## CRM-R5 — Forecasting

*As a sales manager I get a forecast derived from the pipeline, not
typed into it.*

- Live forecast = Σ `amount × probability` over open deals, grouped
  by expected-close period and owner, **per currency** (mixed
  currencies never sum); overflow refused.
- Month-end snapshot persists the roll-up for comparison.

## CRM-R6 — Consent

*As a compliance owner I can show every marketing touch was
consented to.*

- `marketing_consent` per contact with append-only ConsentEvent
  history; unsubscribe withdraws immediately, exits active nurture
  enrolments, and blocks all sends until an explicit re-grant.
- The send path (campaign + nurture) re-checks consent **at send
  time**; segments implicitly require `consent = granted`.

## CRM-R7 — Segments

*As a marketer I define audiences declaratively and preview them
before sending.*

- Segment CRUD (declarative JSON filter over contact fields);
  server-side evaluation; preview returns count + sample; the
  consent AND-gate cannot be expressed away.

## CRM-R8 — Campaigns & ROI

*As a marketer I run email campaigns and see what they returned.*

- Campaign lifecycle `draft → scheduled → running → completed |
  cancelled` (pure core); demo-mode simulated delivery writes
  per-contact touch activities + engagement counters via a `bg_pg`
  job behind a trait seam.
- Funnel view (recipients → delivered → opened → clicked → leads →
  deals → won revenue) + ROI = (attributed won revenue − cost) ÷
  cost, per currency; zero cost reports `null` + absolutes.

## CRM-R9 — Nurture sequences

*As a marketer I enrol contacts in drip sequences that advance
themselves.*

- Sequence CRUD (ordered steps with `delay_hours`); enrolment
  manual / by segment / on lead capture; the `bg_pg` scheduler
  advances due steps idempotently (rerun ⇒ no double-send), logs
  touches, completes after the last step; exit on unsubscribe,
  conversion, or manual exit.

## CRM-R10 — Tickets

*As a support agent I track customer issues from open to closed.*

- Ticket CRUD with contact/account, priority, `worker:` assignment,
  channel; lifecycle `open → pending → resolved → closed`, reopen
  `resolved → open` (pure core); the assignee's first outbound
  call/email activity stamps `first_responded_at`.

## CRM-R11 — SLA tracking

*As a support lead I see promised response times and every breach.*

- SlaPolicy per priority (first-response + resolution minutes, 24×7
  v1); deadlines derived at open and re-derived on audited priority
  change; breach flags computed on read and swept by a job emitting
  `sla_breached` once per breach; breaches clear only by meeting
  the metric.

## CRM-R12 — Knowledge base

*As an agent I answer from published articles and link them to
tickets.*

- Article CRUD `draft → published → archived`; published edits bump
  `version` (priors retained read-only); ILIKE keyword search;
  ticket-link logs a note activity.

## CRM-R13 — Dashboards

*As a leader I see live, honest KPIs.*

- Win rate, pipeline by stage, forecast, campaign funnel/ROI, SLA
  health, CLV per account, activity feed — all pure-core
  derivations per [analytics-reporting.md](analytics-reporting.md),
  ETag-conditional, stamped `as_of`, per-currency, ratios with
  numerator/denominator and `null` on zero denominators.

## CRM-R14 — Activity analytics

*As a manager I see per-rep activity counts by kind and period.*

- Derived from Activity rows; scoped by the persona rules
  ([auth.md](auth.md)).

## CRM-R15 — AuthN/Z & masking

- Family stack: offline PASETO verify, blanket `CRM_REQUIRE_AUTH`
  guard (default off), shared ABAC engine; record-level
  `resource.owner`/`status`/`tier` attrs with `$sub` ownership;
  `mask` obligation redacts amounts, forecasts, ROI, and contact
  channel details; the four personas of [auth.md](auth.md)
  expressible as policy.

## CRM-R16 — Audit & events

- Every mutation audited + evented (family envelope,
  `CRM_EVENT_TRANSPORT` memory/outbox); consent history + sensitive
  reads audited; reasoned actions carry their reasons.

## CRM-R17 — Family fixtures

- OpenAPI + Swagger, `Accepts-version` negotiation, `/metrics.prom`,
  OTLP tracing, health routes, Podman build,
  `#![forbid(unsafe_code)]`, clippy-pedantic, input caps → `422`,
  unknown-pid → `404`.

## CRM-R18 — Operational insight views

*As a manager or DPO I get read-only rollups that surface work
already recorded, instead of re-deriving it by hand.*

- Seven derived views (`GET /api/insights/*`): stale-deals (aging
  from stage-change audits), follow-ups (overdue + 30-day horizon
  over open activities' `due_on`), pipeline-hygiene (rule-disclosed
  findings), the executive period pack (won/lost, per-currency,
  never merged), forecast-trends (from stored `ForecastSnapshot`
  rows only, no interpolation), the SLA breach register +
  per-assignee workload, and the DPO view (consent coverage +
  withdrawals + duplicate-contact hygiene over shared `person_ref`).
  ETag-conditional, stamped `as_of`, same honesty rules as CRM-R13
  above.

## CRM-R19 — Stakeholder engagement & confederation

*As an account owner I record who matters on an account and how an
innovation partnership or membership is progressing, without the
system guessing at any of it.*

- Declared (never inferred) stakeholder typing: `stakeholder_role`
  on Contact and Account; a 1–5 power–interest grid position on
  Contact. Declared (never inferred) `sentiment` on Activity.
- Partnership CRUD per account with a forward-only lifecycle
  (`scouting → pilot → scaled`, `retired` reachable from any live
  stage); Membership (one per account, `active | lapsed` +
  renewal); WorkingGroup + roster.
- Nine derived views (cadence, engagement workload, the
  audit-derived pipeline funnel, member health, consent-by-account,
  the stakeholder register + grid, the partnership register,
  membership renewals) plus a `kind` filter on the CRM-R18
  follow-ups view for the renewals convention.

## CRM-R20 — Subject rights & retention (the code side of CRM-G2)

*As a DPO I can answer a subject-access request for one contact, honour
an erasure request once no live commercial reason remains to keep the
data, and show that data past its retention horizon does not linger.*

- `GET /api/contacts/{pid}/subject-access`: one audited JSON export of
  every table keyed to the contact (consent history, activities, leads,
  deals as primary contact, tickets, nurture enrolments), with genuine
  exclusions named in the payload rather than silently omitted.
- `POST /api/contacts/{pid}/erase`: anonymise — identity fields
  scrubbed to a tombstone `person:` URN, linked free text scrubbed,
  the row soft-deleted. Refused `422` while the contact has an open
  deal, an open support ticket, or an active nurture enrolment
  ([design.md](design.md) CRM-D14): the field that gates this is
  **not** `Contact::status`, which no endpoint ever transitions.
  Destructive-classified; audited with per-table counts.
- `GET /api/retention` / `POST /api/retention/sweep`: the floored
  horizon report (`CRM_RETENTION_DAYS`, default 365, floor 30) and the
  matching hard-delete sweep across every soft-deleting table, plus a
  read-only count of contacts whose consent has stood withdrawn since
  before the horizon (informational — never auto-scrubbed, since a
  contact always carries the erasure gate above). Destructive-classified;
  audited even when it deletes nothing.
