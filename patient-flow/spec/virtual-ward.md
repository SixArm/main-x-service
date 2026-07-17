# Virtual ward

A **virtual ward** (hospital-at-home) delivers acute-level monitoring
and care to patients in their own homes. Patient Flow models it as a
first-class ward rather than a bolt-on, so every whiteboard, capacity,
and journey mechanism works unchanged.

## Model

- A Ward with `kind = virtual`. It has one implicit bay; its Beds are
  **virtual slots** (`virtual = true`) — they exist so census,
  occupancy, and capacity arithmetic stay uniform ("the respiratory
  virtual ward has 20 slots, 14 occupied").
- A Stay on a virtual ward has `bed_pid` pointing at a virtual slot
  and carries `home_location_note` (free text; the authoritative
  address stays in person-service). `source = virtual_admission` for
  direct step-up from the community.
- Virtual slots skip the cleaning cycle: `vacate` returns them
  directly to `available`; they are never `awaiting_clean`.

## Flows

| Flow | Mechanism |
|---|---|
| **Step-down** (hospital → home) | transfer from a physical bed to a virtual slot (`reason = step_down`); the physical bed vacates normally |
| **Step-up** (deterioration at home) | a BedRequest with `origin = virtual_step_up`, priority `urgent`/`emergency`; on allocation, transfer virtual → physical (`reason = step_up`) |
| **Direct virtual admission** | admit straight onto the virtual ward |
| **Virtual discharge** | normal discharge; destination usually `home` |

## Whiteboard

The virtual ward's whiteboard is the same bed-card view: name, EDD,
Red2Green, alerts, named nurse — with the home-location note where a
bay/bed label would be. Remote-monitoring integration (pulse-ox
feeds, alerts from monitoring platforms) is **out of scope**; an
`alerts` chip set manually by the monitoring team is the v1 hook.

## Why it matters

Virtual wards discharge patients earlier (freeing physical beds)
while keeping them on a ward list with an EDD and senior review — the
same SAFER discipline. At-a-glance shows virtual census alongside
physical occupancy so the site meeting sees total managed demand.
