## 2. Scope

### 2.1 In scope

- Event identity CRUD with soft delete and full audit trail.
- schema.org/Thing properties (`name`, `description`, `alternateName`,
  `url`, `image`, `sameAs`, `keywords`).
- Time-window fields (`start_date` required, `end_date`, `door_time`,
  `duration` ISO 8601, `previous_start_date`, `time_zone`, `all_day`).
- Status / mode / type taxonomies aligned with schema.org/Event.
- Capacity fields (total / physical / virtual / remaining).
- `Location` as a union of `Place` / `PostalAddress` / `VirtualLocation`
  / `Text`.
- Parties (organizers, performers, attendees, sponsors, funders,
  contributors).
- Offers (price, currency, availability, validity window).
- Multiple identifiers (`BookingNumber`, `ConfirmationCode`,
  `TicketNumber`, `EncounterId`, `TransactionId`, `ExternalRef`,
  `Tax`, `Other`).
- `super_event` / `sub_events` hierarchy.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy search with date-range filter.
- Real-time + batch duplicate detection + review queue.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (Axum) + gRPC (Create/Get/List/Delete Event).
- FHIR R5 `Appointment` surface (`/fhir/Appointment{,/{id}}` +
  `/fhir/metadata`) — best-effort schema.org/Event → FHIR mapping.
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- FHIR R5 `Encounter` mapping (only `Appointment` ships today) and
  full FHIR conformance (search-param breadth, `_include`, `_history`).
- Recurrence (RFC 5545 RRULE).
- Time-zone-aware fuzzy matching (uses naive UTC offsets today).
- Bulk import / export (§9.1 designs the contract; T-9 unstarted).
- A live Fluvio deployment: the real-broker relay sink (`FluvioSink`)
  itself shipped (T-11/BUS-3), behind the `fluvio` Cargo feature (off
  by default) and idle until an operator points
  `EVENT_FLUVIO_ENDPOINT` at a broker; a consumer reading this
  service's topic (the link-graph aggregator's own consumer is BUS-2,
  elsewhere) is still open.
- ML-based match scoring.

