# Hospital capacity

All capacity numbers are **arithmetic over live operational state**
— no forecasting models in v1 ([scope.md](scope.md)). Everything here
is served by `GET /api/at-a-glance` (per-ward rows + site tiles) and
`GET /api/capacity/metrics` (time-series-friendly snapshot for
dashboards/Prometheus).

## Live counts (per ward, per site)

- Beds by state: total, occupied, available, reserved,
  awaiting-clean, cleaning, closed (by closure reason).
- **Occupancy %** = occupied / (total − closed).
- Virtual-ward census, counted separately and in the total managed
  view.
- Open bed requests by priority and origin; requests with zero
  currently-eligible beds (the escalation signal).

## Flow metrics

- **Predicted discharges today**: active stays with `edd = today`
  (plus overdue EDDs listed separately — an EDD in the past is a
  planning failure worth surfacing, not silently rolled forward).
- **Predicted available by midnight** = available + predicted
  discharges − allocated reservations.
- **Discharge-ready now** and **DTOC** count + bed-days lost
  ([patient-journey.md](patient-journey.md)).
- **Turnaround**: median vacate→available time, routine vs deep
  clean.
- **Early-discharge rate**: % of discharges before midday (SAFER "E"
  target: 33%).
- **Long-stay counts**: LOS > 6 days, > 20 days (SAFER "R" review
  lists).
- **Outlier count**: stays whose ward specialty ≠ request specialty
  (allocation rule-5 overrides).
- Red2Green: red-day count by delay reason, trailing 7 days.

## Escalation

Wards flagged `escalation = true` are surge capacity: opened when
occupancy crosses the trust's threshold. The at-a-glance view shows
escalation beds separately so "we are running on escalation
capacity" is one glance — the honest alternative to quietly
procuring extra beds.

## Observability

The same snapshot feeds `/metrics.prom` gauges (occupancy, available,
DTOC, open requests…) per the family observability conventions, so
site dashboards and alerting ride the existing Prometheus path.
