# Contact Relationship Management — Specification

This directory is the **single source of truth** for the cross-cutting
Contact Relationship Management (CRM) specification, shared by both
editions. Each subproject's own `spec/` adds stack-specific detail and
links back here.

> ⚠️ **Demo software, not a production CRM.** This project models CRM
> practice for demonstration and integration purposes; it holds no
> real personal data and sends no real email. See
> [regulatory.md](regulatory.md).

## What this project is

A **central hub for managing interactions with customers and
prospects** across the whole relationship lifecycle — first touch to
renewal — organized as four modules:

1. **Sales automation (SFA)** — contact & account management, lead
   capture and scoring, deal pipelines (Kanban), forecasting.
2. **Marketing automation** — email campaigns, segments, lead
   nurturing (drip sequences), campaign tracking & ROI.
3. **Customer service & support** — ticketing, a knowledge base,
   SLA tracking.
4. **Analytics & reporting** — real-time dashboards (win rates,
   pipeline value, customer lifetime value), activity tracking.

It is a **consumer application** (the case-folder / patient-flow /
workforce-planning-management shape): it does not register identities
itself. A contact is a [person-service](../../person/person-service-with-loco/)
record; an account is an
[organization-service](../../organization/organization-service-with-loco/)
record; a sales rep or support agent is a
[worker-service](../../worker/worker-service-with-loco/) record. CRM
owns only the **relationship and its operational state**: leads,
deals, activities, campaigns, consent, tickets, articles, SLAs —
always referencing identities by `EntityRef` URN, never duplicating
them.

## Two editions

| Subproject                                                                                                         | Role                           | Stack                                   |
| ------------------------------------------------------------------------------------------------------------------ | ------------------------------ | --------------------------------------- |
| [contact-relationship-management-service-with-rust](../contact-relationship-management-service-with-rust/)         | Back-end JSON API              | Rust, Loco (Axum + SeaORM), PostgreSQL  |
| [contact-relationship-management-front-end-with-svelte](../contact-relationship-management-front-end-with-svelte/) | Sales / marketing / support UI | SvelteKit 2, Svelte 5 runes, TypeScript |

## Specification (topic files)

| File                                               | Covers                                                                  |
| -------------------------------------------------- | ----------------------------------------------------------------------- |
| [purpose.md](purpose.md)                           | Problem statement, goals, the four modules                              |
| [scope.md](scope.md)                               | In/out of scope; the boundary with the identity services                |
| [domain-model.md](domain-model.md)                 | Contact, Account, Lead, Deal, Activity, Campaign, Ticket, SlaPolicy, …  |
| [sales-automation.md](sales-automation.md)         | Module 1: contacts/accounts, lead scoring, pipelines, forecasting       |
| [marketing-automation.md](marketing-automation.md) | Module 2: campaigns, segments, nurture sequences, ROI                   |
| [service-support.md](service-support.md)           | Module 3: tickets, knowledge base, SLA tracking                         |
| [analytics-reporting.md](analytics-reporting.md)   | Module 4: dashboards, KPIs, activity tracking                           |
| [integrations.md](integrations.md)                 | Upstream family services; EntityRef URNs                                |
| [auth.md](auth.md)                                 | SSO, ABAC personas (rep / sales manager / marketing / support), masking |
| [audit.md](audit.md)                               | Audit trail, events, consent history                                    |
| [architecture.md](architecture.md)                 | Editions, layering, pure-core rules, persistence                        |
| [testing.md](testing.md)                           | Test strategy per edition                                               |
| [regulatory.md](regulatory.md)                     | Demo status; GDPR / e-privacy (marketing consent) posture               |
| [roadmap.md](roadmap.md)                           | Beyond the v1 queue                                                     |
| [glossary.md](glossary.md)                         | SFA, MQL/SQL, drip, CLV, SLA, …                                         |

## Specification-driven delivery (SDD)

Three lock-step files drive delivery:

- [requirements.md](requirements.md) — numbered requirements (`CRM-R*`)
  with user stories and acceptance criteria.
- [design.md](design.md) — numbered design decisions (`CRM-D*`).
- [tasks.md](tasks.md) — **the live delivery checklist** (`CRM-T*`),
  phased; every task traces to design and requirement ids.

A change starts in `requirements.md`, is shaped in `design.md`, is
queued in `tasks.md`, and only then lands as code in a subproject.
**No code lands without the spec describing it.**

## References

- Sibling consumer apps (the shape this follows):
  [workforce-planning-management](../../workforce-planning-management/spec/index.md),
  [patient-flow](../../patient-flow/spec/index.md),
  [case-folder](../../case-folder/spec/index.md),
  [project-portfolio-management](../../project-portfolio-management/spec/index.md)
- Family contracts:
  [authentication-sessions](../../agents/share/authentication-sessions.md),
  [authorization-attributes](../../agents/share/authorization-attributes.md),
  [security](../../agents/share/security.md),
  [privacy](../../agents/share/privacy.md) (the person-service consent
  model CRM's marketing consent mirrors)
