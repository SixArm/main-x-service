## 6. Functional Requirements

Entity-level requirements with the owning subproject. Detail lives in
the owner's spec; this table is the contract that the trio, taken
together, must satisfy. (Terminology: the domain entity is the
capital-E **Event**; the CRUD-change records on the stream are
"index-level events" — see [§4](04-glossary.md).)

| # | Requirement | Owner | Detail |
|---|---|---|---|
| FR-1 | Create / read / update / soft-delete Event records, with `422` on validation failure | Service | [spec §6.1, §6.5](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-2 | Multiple typed, system-qualified identifiers per Event | Service | [AGENTS/models.md](../event-service-with-loco/AGENTS/models.md) |
| FR-3 | Time-window handling: required UTC `start_date`, optional `end_date` / `door_time` / `duration`, IANA `time_zone` for display, reschedule tracking via `previous_start_date` | Service | [spec §5](../event-service-with-loco/spec/05-domain-model.md) |
| FR-4 | Probabilistic matching with per-component score breakdown, including time-window components (start/end-date decay, window overlap as Jaccard of `[start, end)`) | Service + Matcher | [AGENTS/matching.md](../event-service-with-loco/AGENTS/matching.md) |
| FR-5 | Deterministic matching with short-circuits: strong-identifier exact match (service); shared `(scheme, value)` event ID or same normalised name + same `start_date` instant (matcher) | Service + Matcher | [matcher AGENTS.md](../event-matcher-rust-crate/AGENTS.md) |
| FR-6 | The service MUST be able to score any persisted pair through the canonical matcher via `to_matcher_event`; the projection rules of §5.3 MUST hold | Service (adapter) | [§5.3](05-domain-model.md) |
| FR-7 | Full-text + fuzzy search with date-range filter, facets, pagination, optional masking | Service | [spec §6.3](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-8 | Real-time duplicate detection on create (`409` with candidates), explicit check, and batch scan | Service | [spec §6.4](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-9 | Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`) for borderline duplicates | Service | [spec §6.4](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-10 | Merge: survivor + duplicate, data transfer, `alternate_name` alias, `Replaces` link, soft-delete, JSON snapshot, `Merged` index-level event | Service | [spec §6.4](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-11 | Privacy: masking of identifier values + party emails, masked view, GDPR Article 15 export, consent records | Service | [spec §6.6](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-12 | Audit: every CRUD / merge / link writes `audit_log` with old + new JSON, user ID, IP, user agent, timestamp; audit query endpoints | Service | [spec §6.7](../event-service-with-loco/spec/06-functional-requirements.md) |
| FR-13 | Index-level event streaming: `Created` / `Updated` / `Deleted` / `Merged` / `Linked` / `Unlinked` published on every change | Service | [agents/share/auditability.md](../../agents/share/auditability.md) |
| FR-14 | Operator UI: dashboard, list/search grid, create with inline 409-duplicate surfacing, detail / edit / soft-delete, per-record audit view, match check, merge with preview | Front-end | [spec §6](../event-front-end-with-svelte/spec/06-functional-requirements.md) |
| FR-15 | The front-end binds only to the service's `/api/v1` REST surface — never to the database or the matcher directly | Front-end | [spec §9](../event-front-end-with-svelte/spec/09-api-consumption.md) |
| FR-16 | Pure-library guarantee: the matcher performs no IO, no logging, no clock/RNG access; same inputs ⇒ same outputs | Matcher | [spec §8](../event-matcher-rust-crate/spec/08-determinism-and-safety.md) |
| FR-17 | Explainability: every probabilistic result carries a per-field breakdown surfaced through the REST API to the operator UI | All three | [matcher README](../event-matcher-rust-crate/README.md) |

### 6.1 Two matching algorithms — current ruling

The service today carries an in-service matcher (weights: name 0.20,
start 0.20, end 0.10, location 0.15, organizer 0.10, performer 0.10,
attendee 0.05, identifier 0.10) **and** embeds the canonical matcher
crate (weights: name 0.20, start 0.25, end 0.05, location 0.15,
category 0.08, country 0.04, event_ids 0.15, organizer 0.04,
performers 0.02, url 0.02). The matcher crate is the **canonical
reference algorithm**; the in-service matcher powers the live REST
endpoints. Convergence is an open question (§16 EOQ-1, task ET-4).
Until resolved, both weight tables are normative for their own
surface.
