# Whiteboard & views

The read surface. All views are **derived** from the operational
tables — no separate whiteboard store to drift. Each is one API
endpoint plus a front-end page; the front-end targets three form
factors: **ward touchscreen** (large-format, wall-mounted, tap-first),
desktop, and mobile.

## Ward whiteboard — `GET /api/whiteboard/{ward_pid}`

The digital replacement for the marker board: one **bed card** per
bed, in bay order.

Each bed card shows:

- bed number and state (colour-coded; empty states show
  available / reserved-for / awaiting clean / cleaning / closed+reason)
- for occupied beds: patient display name (masked when the caller's
  ABAC decision carries the `mask` obligation), named nurse and
  consultant (worker display names)
- **EDD** (or a missing-EDD chip), **CCD met** tick,
  discharge-pathway badge (P0–P3), **discharge-ready** highlight
- today's **Red2Green** colour
- **infection precaution** icon (contact/droplet/airborne + organism,
  suspected vs confirmed)
- **alert chips** (falls risk, dementia, DNAR present, …)
- senior-review-done-today tick (SAFER "S")

Interactive (touchscreen) actions from a card, each a thin wrapper
over the API: set EDD, mark senior review, mark CCD met, record
red/green + reason, start/complete clean, request transfer, mark
discharge-ready, discharge. Tapping a card opens the full stay view.

## Bed card / stay detail — `GET /api/stays/{pid}`

The full journey for one patient: current location, admission
source and time, LOS, transfer history, the full Red2Green run,
infection flags, delay reasons, EDD/CCD, pathway, alerts, and the
stay's audit slice. This is the MDT discussion page.

## Hospital at a glance — `GET /api/at-a-glance`

One row per ward (grouped by site):

| Column | Source |
|---|---|
| beds total / occupied / available / reserved / cleaning / closed | bed states |
| occupancy % | derived |
| expected discharges today | active stays with `edd = today` |
| discharge-ready now / DTOC | stay status + grace rule |
| open bed requests targeting this ward | request queue |
| infection closures | closed beds + `closed_to_admissions` bays |
| escalation flag | ward attribute |

Plus site-level headline tiles: total available now, predicted
available by midnight (available + EDD-today), open requests by
priority, DTOC count, virtual-ward census. This is the site-manager
and bed-meeting view.

## Patient locate — `GET /api/locate/{person_ref}`

*"Where is patient X right now?"* Given a `person:<pid>` URN, returns
the active stay's site / ward / bay / bed (or virtual ward + home
note), or the most recent discharged stay. Access to locate is
ABAC-gated and audited — location of a patient is personal data.

## Bed request board — `GET /api/bed-requests?status=open`

The demand queue for flow coordinators: open requests ranked by
priority and wait, each with its eligible-bed count (a live
feasibility signal — an `emergency` request with zero eligible beds
is the escalation trigger).

## Freshness

Whiteboards poll (ETag/`updated_since`) in v1; server-push (SSE from
the event stream) is roadmap. Every view carries an `as_of` timestamp
so a wall screen is honest about staleness.
