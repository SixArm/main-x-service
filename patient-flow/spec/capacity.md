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

## Staff utilisation — permitted, not yet buildable

**Decision 2026-08-25.** Per-person utilisation — recorded effort
against declared available capacity — is **permitted** in this service,
extending the exception in
[`agents/share/time-based-analysis.md` §7.1](../../agents/share/time-based-analysis.md).
The family refusal to compute **per-person cycle time, throughput or
efficiency** is **unchanged** and still binds: this reverses one narrow
thing.

**Note what this is not.** Everything else on this page is capacity of
**beds**, and the service's `allocation` module is **bed** allocation.
Staff utilisation is a different denominator entirely, and the service
holds neither half of it today: **no roster** (who is on shift, for how
long) and **no recorded effort**. Both are new inputs, so the figure is
absent rather than zero.

When built it adopts the five §7.1 obligations, with the same two
clinical sharpenings care-pathway carries:

- **Suppression is a privacy control.** A bay staffed by three nurses
  makes a per-person figure identifying at almost any aggregation.
- **A high reading is a warning.** Utilisation near 100% is what a
  queueing system looks like just before it stops coping — on a ward
  that is a safety observation, not a productivity win. It belongs
  **beside** occupancy and the open-request escalation signal, which
  this page already treats the same way.

One thing to hold onto: this page's honesty rule — that escalation beds
are shown separately so "we are running on escalation capacity" is one
glance — applies to staff exactly as it does to beds. A ward running at
safe occupancy on unsustainable staff utilisation should not read as
green.

## Observability

The same snapshot feeds `/metrics.prom` gauges (occupancy, available,
DTOC, open requests…) per the family observability conventions, so
site dashboards and alerting ride the existing Prometheus path.
