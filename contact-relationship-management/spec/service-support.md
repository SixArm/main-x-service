# Module 3 — Customer service & support

## Ticketing

Lifecycle `open → pending → resolved → closed`, reopen
`resolved → open` (pure-core machine; `closed` is terminal).
`pending` means waiting-on-customer and **pauses no clock in v1**
(24×7 clocks; business-hours/pause semantics are roadmap and
documented as such). Assignment to a `worker:` URN; priority
changes re-derive SLA deadlines from the ticket's opened time and
are audited with a reason. The first outbound activity of kind
`call` / `email` by the assignee stamps `first_responded_at`.

## SLA tracking

An SlaPolicy fixes per-priority first-response and resolution
minutes. Deadlines are **derived once at open** (and on priority
change) — `first_response_due_at`, `resolution_due_at`; breach
flags flip when `now` passes an unmet deadline (computed on read
and by a sweep job that emits `sla_breached` once per breach).
Breaches are facts, not editable: they clear only by meeting the
metric (first response recorded / ticket resolved), never by
editing history.

## Knowledge base

Articles `draft → published → archived`; published edits bump
`version` (prior versions retained read-only). Keyword search
(ILIKE v1) serves both agents and — roadmap — a public portal.
Linking an article to a ticket logs a note activity, so "answered
by KB" is measurable.

## The `case` registry is not the ticket store

A CRM ticket is operational support state owned here. The family's
governmental `case` service registers deduplicated case
*identities* with matching; the two must not be conflated
([scope](scope.md)).
