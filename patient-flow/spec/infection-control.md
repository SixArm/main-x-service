# Infection control

Infection-control state must be visible at the bed, bay, ward, and
patient levels — it drives allocation, cleaning, and closure. Covid
is the motivating example but the model is organism-generic.

## Patient level — InfectionFlag

Per-stay flags ([domain-model.md](domain-model.md)): precaution class
(`contact` / `droplet` / `airborne` / `protective`), optional named
organism (`covid-19`, `c-diff`, `mrsa`, `norovirus`, …), status
(`suspected` → `confirmed` → `cleared`), and `requires_side_room`.

Effects while an uncleared flag exists:

- the bed card shows the precaution icon (suspected renders hollow,
  confirmed solid);
- transfer eligibility: the stay can only move to a side room /
  isolation-capable bed unless the flag is `protective` (which
  instead restricts who can be co-located with them);
- on vacate, the bed gets `deep_clean_required = true`.

Flags are set/cleared by explicit API action, audited, and evented
(`infection_flagged` / `infection_cleared`).

## Bed level — deep clean

`deep_clean_required` beds follow the normal cleaning cycle but
`clean-complete` requires the deep-clean confirmation, and until then
the bed is excluded from allocation. Turnaround metrics report deep
cleans separately (they take hours, not minutes).

## Bay & ward level — closure to admissions

An outbreak (e.g. norovirus, Covid cluster) closes a **bay** or a
whole **ward** to admissions: `closed_to_admissions = true`. Existing
patients stay; the allocator refuses new placements; at-a-glance
shows the closed capacity and its reason. Reopening requires the
terminal cleans to be complete (all beds in the bay out of
`awaiting_clean`/`cleaning`).

## Cohorting note

Cohorting (grouping confirmed same-organism patients in one bay) is
supported operationally — the allocator permits placing a flagged
patient into a bay whose current occupants all carry a **confirmed**
flag for the **same organism** — but automated cohort suggestion is
roadmap, not v1.

## Reporting

Capacity views expose: beds closed for infection, deep cleans
pending/in progress, bays/wards closed to admissions, and active
flag counts by organism and status. That is the daily IPC (infection
prevention & control) huddle view.
