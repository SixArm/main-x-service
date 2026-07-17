# Patient journey — admission to discharge

## Flows

### Admit

1. (Usually) a **BedRequest** exists and has an allocated (reserved)
   bed.
2. `POST /api/stays` with `person_ref`, `source`, and the bed —
   validates the person URN, checks the bed is `reserved` for this
   request or `available`, creates the Stay (`status = admitted`),
   flips the bed to `occupied`, writes the placement Transfer,
   fulfils the request, audits, emits `stay_admitted`.
3. SAFER "A" nudge: the create response flags a missing `edd`/`ccd`;
   the whiteboard shows an EDD-missing chip until set.

### Transfer

`POST /api/stays/{pid}/transfer` with the destination bed and a
`reason` — checks destination eligibility (same allocation rules),
moves the stay, vacates the old bed (`awaiting_clean`), occupies the
new one, writes a Transfer row, emits `stay_transferred`. Ward-level
moves answer *"visibility of a patient's current location"*: locate
is one indexed query over active stays.

### Discharge-ready

`POST /api/stays/{pid}/discharge-ready` — requires `edd` set and
`ccd_met = true`; stamps `discharge_ready_at`, sets the discharge
`pathway` (P0–P3), emits `stay_discharge_ready`. From this moment the
patient is "medically ready"; further waiting is a **delayed transfer
of care (DTOC)** and is counted in capacity views.

### Discharge

`POST /api/stays/{pid}/discharge` with `discharge_destination` —
stamps `discharged_at`, vacates the bed, closes any open infection
flags as `cleared` (bed keeps `deep_clean_required` if set), emits
`stay_discharged`. Length of stay and DTOC duration become final.

## SAFER patient flow bundle mapping

The five SAFER elements and where they live:

| SAFER | Meaning | Patient Flow support |
|---|---|---|
| **S** — Senior review | every patient reviewed by a decision-capable senior before midday | `senior_review_at` stamp; whiteboard chip when today's review is missing by noon |
| **A** — All patients have an EDD + CCD | set on admission, assuming ideal recovery | `edd`, `ccd`, `ccd_met` fields; missing-EDD chip; EDD-today drives predicted discharges |
| **F** — Flow | first ward admission from assessment units by 10:00 | assessment-ward `kind`; transfer timestamps make the 10am first-arrival measurable |
| **E** — Early discharge | 33% of discharges before midday | `discharged_at` distribution in capacity metrics |
| **R** — Review | patients with extended stays get a systematic review | long-stay list (LOS > 6 days, > 20 days) in at-a-glance |

## Red2Green day journal

Every stay-day starts **red**. It turns **green** only when the day
moved the patient measurably toward discharge (senior review
happened, EDD/CCD set, planned actions done). A day left red records
**up to two coded delay reasons** (see
[domain-model.md](domain-model.md) `RedGreenDay.delay_reasons`) — the
aggregated reason counts are the improvement signal ("this week we
lost 41 bed-days to awaiting_transport").

The journal is append-per-day, editable same-day, frozen after.
Whiteboard bed cards show today's colour; the stay view shows the
full run (e.g. `🟢🟢🔴🔴🟢`).

## DTOC

A stay is a **delayed transfer of care** when `discharge_ready_at` is
set, the patient is still in a bed, and the configured grace period
(default: midnight of the ready day) has passed. DTOC stays are
listed with their pathway and current delay reasons; the count and
the bed-days lost are headline at-a-glance metrics. This is the
operational lever the G-Cloud benefit "helps prevent DTOC" describes:
making the queue visible, attributable, and discussable at the daily
MDT huddle.

## MDT & handover support

- The **whiteboard** is the shared artefact for board rounds: every
  field an MDT huddle needs (EDD, CCD-met, pathway, today's colour,
  delay reasons, alerts) is on the bed card.
- **Handover** is served by the audit trail: a shift can review every
  change on the ward since a timestamp (`GET /api/audits?ward=…&since=…`)
  rather than relying on a paper sheet.
