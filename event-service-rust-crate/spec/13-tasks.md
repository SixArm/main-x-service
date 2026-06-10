## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — FHIR R5 mapping decision + implementation.**
  - [ ] Decide Encounter vs Appointment vs other event-pattern (OQ-1).
  - [ ] Implement bidirectional conversion for the chosen resource.
  - **Acceptance:** `POST /fhir/Event` round-trips through the chosen
    resource; OperationOutcome on errors.
- [ ] **T-2 — Time-zone-aware fuzzy matching.**
  - [ ] Replace naive UTC offsets with `chrono-tz` conversions in the
    date-proximity scorer.
  - **Acceptance:** unit test where one event in `America/New_York`
    matches another in `UTC` at the same wall-clock instant.
- [ ] **T-3 — RFC 5545 RRULE recurrence support.**
  - [ ] Add `recurrence_rule: Option<String>` to `Event`.
  - [ ] Implement expansion for search + dedup.
  - **Acceptance:** weekly RRULE expanded into 52 occurrences for
    range queries.
- [ ] **T-4 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes an `EventCreated`
    record end-to-end.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `EventService.GetEvent`
    round-trips a record.
- [ ] **T-7 — iCalendar import / export.**
  - [ ] `POST /api/v1/events/import.ics`, `GET /api/v1/events/{id}.ics`.
  - **Acceptance:** Apple Calendar imports the exported `.ics`
    without warnings.
- [ ] **T-8 — Authentication / authorisation.**
  - [ ] JWT middleware on `/api/v1/*` with scheduler / admin /
    read-only / service roles.
  - **Acceptance:** unauthenticated requests get `401`; valid token
    + role gets `2xx`.

