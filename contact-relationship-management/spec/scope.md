# Scope

## In scope (v1)

- Contacts and accounts as **relationship wrappers** over `person:`
  and `organization:` URNs, with lifecycle status and ownership.
- Leads with rule-based scoring and conversion; deals with
  configurable pipelines/stages, Kanban ordering, and won/lost close;
  activities (call, email-logged, meeting, note, task) on any of
  contact/account/lead/deal/ticket.
- Stage-weighted forecasting per period/owner.
- Campaigns (email kind in v1) with segments, simulated sends,
  engagement counters; nurture sequences (ordered delayed steps)
  advanced by a `bg_pg` scheduler; marketing consent + unsubscribe.
- Tickets with priority/assignment/lifecycle, SLA policies with
  derived first-response/resolution deadlines and breach flags;
  knowledge-base articles (draft → published, versioned).
- Dashboards: win rate, pipeline by stage, forecast, campaign ROI,
  SLA health, CLV per account; activity feeds. ETag conditional.
- Family fixtures: auth (PASETO + ABAC + `CRM_REQUIRE_AUTH`), audit,
  events (memory/outbox), OpenAPI, `Accepts-version`, OTLP, Podman.

## Out of scope (v1)

- Real email delivery, deliverability, bounce handling (simulated;
  ESP adapter is roadmap).
- Quoting, CPQ, contracts, invoicing, payments.
- Telephony/CTI, chat channels, social listening.
- ML lead scoring or predictive forecasting (rules + arithmetic
  only).
- Duplicate detection of contacts/accounts — that is the person /
  organization registries' matcher job; CRM links to the surviving
  record.
- Territory management, quota plans, commission calculation.

## Boundary with the family

| Concern | Owner |
|---|---|
| Who a person/company *is* (demographics, dedup, merge) | person / organization services |
| Sales rep & support agent identity | worker-service |
| Login, sessions, tokens, attributes | authentication-service |
| The relationship, its state and history | **CRM (this project)** |

Contacts and accounts carry a **required** `person:` /
`organization:` URN. When the upstream registry merges two records,
CRM repoints its wrapper on the `merged` event (roadmap; manual
repoint endpoint in v1). The governmental **`case` registry is not
the ticket store**: a CRM ticket is operational support state and
never becomes a `case` record.
