# Module 2 — Marketing automation

## Consent is the gate

No marketing send — campaign or nurture — without
`marketing_consent = granted` **at send time**. Consent changes are
append-only `ConsentEvent` rows; `unsubscribed` withdraws consent
immediately, exits all active nurture enrolments, and sticks until
an explicit re-grant. Segments implicitly AND `consent = granted`;
a segment cannot be defined that bypasses it.

## Campaigns

`draft → scheduled → running → completed | cancelled` (pure-core
machine). A campaign targets one Segment, carries `cost_minor`, and
in **demo mode simulates delivery**: the send job enumerates the
segment, writes per-contact touch activities, and advances
engagement counters via a deterministic stub (a real ESP adapter is
roadmap — the send path is a trait seam). Engagement feeds lead
scoring (campaign click) and attribution.

## Segments

Declarative JSON filters over contact fields (status, account tier,
industry, consent — always granted, activity recency). Evaluated
server-side; a segment preview returns count + sample before
scheduling.

## Lead nurturing (drip)

A NurtureSequence is ordered steps with `delay_hours`. Enrolment
(manual, by segment, or on lead capture) sets `next_due_at`; a
`bg_pg` scheduler job advances due enrolments: simulate the step
send, log the touch activity, emit `nurture_step_sent`, schedule the
next step; after the last step the enrolment completes. Exit rules:
unsubscribe (immediate), lead conversion, manual exit. The scheduler
is idempotent per (enrolment, step) — reruns never double-send.

## Campaign tracking & ROI

Attribution v1 is **source attribution**: a lead or deal carries
`source_campaign_pid`; won-deal revenue rolls up to the campaign.

`ROI = (attributed_won_revenue_minor − cost_minor) / cost_minor`
— per currency, pure-core, division-by-zero (free campaigns)
reported as `null` with the absolute figures alongside. The
campaign dashboard shows recipients → delivered → opened → clicked →
leads → deals → won revenue as a funnel.
