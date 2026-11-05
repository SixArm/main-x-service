## 1. Purpose and Vision

### 1.1 Purpose

The Event Service is a centralised registry of **time-bounded events**:
appointments, encounters, shifts, sessions, deliveries, incidents,
scheduled tasks — anything that can be canonicalised as "a thing
happening, between a start time and an end time, involving parties and
a place." The domain model is aligned with
[schema.org/Event](https://schema.org/Event).

### 1.2 Vision

A single trustworthy view of each event regardless of how many
scheduling, EHR, CRM, calendar, or operational systems hold a shard
of that event:

- Match probabilistically and deterministically against arbitrary input
  (party + approximate time, identifier + organisation, partial title +
  venue) and return ranked candidates with per-component score breakdowns.
- Detect duplicates in real time on create *and* in batch on demand —
  for example, the same booking created by both a self-service portal
  and the operational system of record.
- Expose a stable cross-system identifier so downstream analytics,
  billing, and notifications refer to one event ID per real-world
  occurrence.
- Emit audit logs and event-streaming records for every CRUD / merge /
  link operation. ("Event streaming" here is the Fluvio pipe for
  *index-level* changes, not the modelled domain events themselves.)

### 1.3 Non-goals

- **Not** a calendaring engine — RFC 5545 recurrence is a roadmap item
  (§15), not a current capability.
- **Not** a scheduler — events are recorded, not allocated against
  resources.
- **Not** a notification / reminder system — downstream consumers may
  build that on top of the event stream.
- **Not** an authentication / authorisation provider — the central
  authentication-service owns identity; this service only verifies its
  PASETO v4.public tokens offline (blanket `/api/*`+`/fhir/*`
  enforcement shipped 2026-07-04, default off via `EVENT_REQUIRE_AUTH`
  — an operational activation decision, not a build gap; §13 T-8).

