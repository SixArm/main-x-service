# Module 4 — Analytics & reporting

## Principle

Every number is **derived by pure-core arithmetic from recorded
facts** — the family posture (patient-flow capacity, PPM burndown).
No KPI is stored as an editable field; dashboards are conditional
reads (ETag; tag excludes `as_of`) stamped with `as_of`.

## Dashboards

| View | Derivation |
|---|---|
| **Win rate** | won ÷ (won + lost) over closed deals, filterable by period / owner / pipeline |
| **Pipeline by stage** | open-deal count + Σ amount per stage, per currency |
| **Forecast** | stage-weighted Σ (amount × probability) by close period and owner ([sales-automation](sales-automation.md)) |
| **Campaign ROI** | funnel + (attributed won revenue − cost) ÷ cost ([marketing-automation](marketing-automation.md)) |
| **SLA health** | open tickets by priority × breach state; first-response and resolution attainment over a period |
| **CLV** | per account: Σ won-deal `amount_minor` per currency (v1 revenue-sum definition; margin/discounting is roadmap) |
| **Activity feed** | recent activities by team / owner / kind — the coaching and audit surface |

## Activity tracking

Activities are first-class rows ([domain-model](domain-model.md))
attached to any relationship object; the feed and per-rep counts
(calls/meetings/notes per period) come from them. Activity reads on
another rep's pipeline follow the persona scoping in
[auth.md](auth.md).

## Honesty rules

- Mixed currencies never silently sum — per-currency lines always.
- Every ratio reports its numerator/denominator; zero denominators
  render as `null` + the absolute figures, not as 0% or 100%.
- Dashboards carry `as_of` and the filter that produced them.
