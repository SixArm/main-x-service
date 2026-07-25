# Pillar 2 — Workforce management

## Time & attendance

Time entries record worked minutes per employee per date (a clock
in/out pair or a direct minutes value), kinded `regular` / `remote` /
`overtime` / `on_call`. The pure core derives **overtime** as minutes
beyond the contracted day (from `fte_percent` × the org's standard
day), so payroll and dashboards read one consistent number. Entries
are bounded (≤ 24h/day, no future dates) and edits are audited.

## Absence management

- **Entitlements** per employee, leave kind, and year (minutes), with
  the balance derived: total − minutes of approved requests.
- **Requests** carry kind, date range, and an approval flow
  (`requested → approved | rejected → cancelled`). Approval
  (a manager/HR action under ABAC) decrements the balance; annual
  leave cannot go negative (refused with the balance named), sick
  leave may (flagged, not blocked) — the documented rule set lives in
  the pure core.
- Approved leave feeds conflict checks in scheduling and shows on
  the team calendar.

## Scheduling

Shifts declare date, start/end, department, location, and required
headcount. Assignment binds employees to shifts; the pure core
refuses **double-booking** (overlapping assignments) and
**leave conflicts** (assignment over approved leave), and warns on
under-filled shifts. Views: a week board per department (shifts ×
assignees, gaps highlighted) and a per-employee rota.

## Working-time guardrails (WPM-R27 — WPM-D19)

Advisory Working Time Regulations signals derived entirely from data
this pillar already holds (`GET /api/workforce/working-time`): the
**17-week / 48-hour average** over *recorded* (not merely approved)
minutes, with WPM-D16 terms, and **11-hour rest-gap** breaches across
recent *and planned* shift assignments (±28 days). Flags only —
nothing is refused; the regulations' opt-outs and compensatory-rest
rules are a deployment's call. Visibility equals the rota's.

## Rules summary (pure core, exhaustively unit-tested)

| Rule | Outcome |
|---|---|
| time entry > 24h or future-dated | 422 |
| annual leave request over balance | 422 naming the balance |
| sick leave over balance | allowed + `negative_balance` flag |
| overlapping shift assignments | 422 |
| assignment over approved leave | 422 |
| overtime derivation | minutes beyond contracted day, per entry date |
| 17-week average > 48 h (recorded minutes) | advisory flag, exact integer boundary |
| consecutive assignments < 11 h apart | advisory rest-gap flag (overlap clamps to 0) |
