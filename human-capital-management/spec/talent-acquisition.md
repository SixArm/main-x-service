# Pillar 1 — Talent acquisition & onboarding

## Requisitions (the opening)

A requisition is the funded opening: title, department, headcount,
salary band, hiring manager. Its pipeline is a strict state machine
(`draft → open → interviewing → offer → filled | cancelled`) —
pure-core rules, off-path actions refused with the current state
named (the PPM proposal-pipeline pattern). `filled` is reached when
hires meet headcount.

## Applications (the ATS pipeline)

Each application binds a candidate to a requisition and advances
`received → screened → interviewing → offer → hired | rejected |
withdrawn`. Interview rows record scheduling (when, kind, an
optional `worker:`/`person:` interviewer ref) and outcomes, so the
pipeline view answers "who is waiting on whom". **Hire** is the
conversion moment: an offer-stage application mints the Employee
record (`status = onboarding`) with the agreed salary, links back to
the application for provenance, and counts toward the requisition's
headcount.

## Candidate pool

Candidates persist beyond a single application (source: applied /
referral / sourced), with tags for discovery and an optional
`person_ref` when they match a person-service record. Retention is
**consent-bounded** (`consent_until`); expired-consent candidates are
excluded from search and flagged for purge (see
[regulatory.md](regulatory.md)). A new requisition searches the pool
by tags/title before advertising.

## Onboarding

Hiring seeds a per-employee checklist from a template: contract
signature, background check, right-to-work, tax forms, equipment,
induction training. Items complete or are explicitly waived (audited);
the employee can only **activate** (`onboarding → active`) when every
item is closed — the "ready before day one" guarantee as a state-machine
precondition rather than a hope.

## Views

- Requisition board: pipeline columns with application counts.
- Application list per requisition, stage-ordered, with interview
  schedules and ageing.
- Candidate pool search (tags, title, source, consent state).
- Onboarding tracker: new hires × open checklist items, due dates.
