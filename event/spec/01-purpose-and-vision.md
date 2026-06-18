## 1. Purpose and Vision

### 1.1 Purpose

The **event entity** of the Main X Index is a registry of
**time-bounded events** — public hearings, civic programmes,
consultations, elections-adjacent events (registration drives,
candidate forums), conferences, performances, appointments,
encounters, shifts, incidents — anything that can be canonicalised
as "a thing happening, between a start time and an end time,
involving parties and a place." The domain model is aligned with
[schema.org/Event](https://schema.org/Event).

The entity ships as a trio of subprojects:

| Subproject | Role |
|---|---|
| [event-service-with-loco](../event-service-with-loco/) | System of record — CRUD, search, matching, merge, audit, privacy, REST API |
| [event-matcher-rust-crate](../event-matcher-rust-crate/) | Canonical pairwise comparison library, embedded by the service |
| [event-front-end-with-svelte](../event-front-end-with-svelte/) | Operator UI over the service's REST API |

### 1.2 Vision

**One canonical record per real-world event, at national and
international scale.** The Main X Index serves a worldwide public
governmental system with millions of users; agencies, scheduling
systems, EHRs, and civic platforms each hold a shard of the same
event. The entity must:

- Match probabilistically and deterministically against arbitrary
  input (party + approximate time, identifier + organisation,
  partial title + venue) and return ranked candidates with
  per-component score breakdowns — explainable to auditors and
  regulators, never a black box.
- Detect duplicates in real time on create *and* in batch on demand —
  e.g. the same hearing registered by both a self-service portal and
  the agency's operational system of record.
- Expose a stable cross-system identifier so downstream analytics,
  billing, notifications, and public transparency portals refer to
  one event ID per real-world occurrence.
- Emit audit logs and event-streaming records for every CRUD / merge /
  link operation. ("Event streaming" here is the pipe for
  *index-level* changes, not the modelled domain events themselves —
  see [§4 Glossary](04-glossary.md).)
- Serve a multi-locale public: locale-aware names and descriptions
  (`in_language`), IANA time zones for display over UTC storage, and
  the locale set in
  [`agents/share/locales.md`](../../agents/share/locales.md).

### 1.3 Non-goals

- **Not** a calendaring engine — RFC 5545 recurrence is a roadmap
  item (§15), not a current capability.
- **Not** a booking or scheduling system — events are recorded, not
  allocated against resources, rooms, or staff.
- **Not** a ticketing platform — `Offer` records describe pricing and
  availability for matching purposes; the entity sells nothing.
- **Not** a notification / reminder system — downstream consumers may
  build that on top of the event stream.
