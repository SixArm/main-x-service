# Purpose

## Problem

Customer interactions scatter across inboxes, spreadsheets, and
heads: nobody knows which leads are hot, deals stall unseen,
marketing can't tell which campaign paid for itself, support answers
the same question twice, and the numbers management wants (win rate,
pipeline value, customer lifetime value) are assembled by hand,
late, and differently every time.

## What CRM provides

A single relationship hub over the family's identity registries:

### 1. Sales automation (SFA)

- **Contact & account management** — every person and organization
  the business talks to, with the full interaction history in one
  place (activities: calls, emails-logged, meetings, notes).
- **Lead management** — capture leads, score them by deterministic
  rules so reps work the hottest first, convert the qualified into
  contacts + deals.
- **Pipeline management** — deals as cards moving through named
  stages (Kanban); stalled-deal visibility.
- **Forecasting** — stage-weighted pipeline value per period and
  owner, from recorded amounts, in minor units.

### 2. Marketing automation

- **Email campaigns** — audience segments, send tracking
  (simulated delivery in demo mode), per-campaign engagement.
- **Lead nurturing** — drip sequences: ordered steps with delays,
  advanced by a scheduler; enrolment and exit rules.
- **Campaign tracking & ROI** — cost vs revenue attributed from won
  deals sourced to the campaign.
- **Consent first** — marketing contact requires recorded, current
  consent; unsubscribe is immediate and permanent until re-consent.

### 3. Customer service & support

- **Ticketing** — customer issues with priority, assignment, and a
  lifecycle from open to closed.
- **Knowledge base** — versioned articles, draft → published, linked
  from tickets.
- **SLA tracking** — first-response and resolution targets per
  priority; breach flags that appear on dashboards, not in excuses.

### 4. Analytics & reporting

- **Real-time dashboards** — win rate, pipeline value by stage,
  ticket backlog + SLA health, campaign ROI, customer lifetime
  value — derived by arithmetic from recorded facts, never typed in.
- **Activity tracking** — who did what with which relationship,
  feeding both coaching and audit.

## Goals

| Goal | Measure |
|---|---|
| One view per relationship | contact/account timeline joins deals + activities + campaigns + tickets |
| Reps work the right leads | scoring is explainable rules, recomputed on change |
| Honest forecasts | derived from stage probabilities × amounts; no hand-edited totals |
| Marketing is consent-clean | no send without current consent; unsubscribes stick |
| Support keeps promises | SLA clocks derived, breaches visible |
| Numbers agree | every KPI is one pure-core derivation with unit tests |

## Non-goals

- Not an identity registry — person/organization/worker services own
  who people and companies *are* (and matching/dedup of them).
- Not an email service provider — sends are simulated in demo mode;
  a real ESP integration is roadmap.
- Not a billing/ERP system — deals record amounts; invoicing is out.
- Not a governmental case system — tickets are support records, not
  the `case` registry's matchable case identities.
