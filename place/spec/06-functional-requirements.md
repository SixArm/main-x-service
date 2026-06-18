## 6. Functional Requirements

Entity-level requirements, each mapped to the owning subproject.
Detailed behaviour lives in the owner's spec — links below.

| # | Requirement | Owner | Detail |
|---|---|---|---|
| FR-1 | Place CRUD with soft delete and full audit trail | service | [spec §6.1](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-2 | Multiple identifiers per place (GLN, FIPS, GNIS, OSM ID, branch code, custom); GLN validated as 13 digits + check digit | service | [spec §6.5](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-3 | Structured `PostalAddress` management with normalisation (title-case locality, uppercase region/country, abbreviation expansion) | service | [spec §6.5](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-4 | `GeoCoordinates` management with bounds validation (WGS 84 decimal degrees) | service | [spec §5.3](../place-service-with-loco/spec/05-domain-model.md) |
| FR-5 | Place hierarchy (`contained_in_place` / `contains_place`) with cycle rejection | service | [spec §6.1](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-6 | Probabilistic matching — weighted name (Jaro-Winkler + Soundex), geo (Haversine decay), address, type, identifier; weight renormalisation for missing fields | matcher (canonical), service (embeds) | matcher [spec §5–§6](../place-matcher-rust-crate/spec/05-matching-pipeline.md); service [spec §6.2](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-7 | Deterministic matching — GLN exact match short-circuits to 1.0; matcher rule: shared `(scheme, value)` place-id OR identical normalised name + postcode | matcher + service | matcher [spec §5](../place-matcher-rust-crate/spec/05-matching-pipeline.md) |
| FR-8 | Explainable matching — every match returns a per-component `MatchBreakdown` over the API and in the UI | matcher → service → front-end | matcher [spec §3](../place-matcher-rust-crate/spec/03-data-model.md) |
| FR-9 | Full-text + fuzzy + boolean search (Tantivy) over names, identifiers, address components, place type | service | [spec §6.3](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-10 | Geo-radius search — `GET /api/places/nearby?lat=&lon=&radius_km=` (Haversine + bbox pre-filter) | service | [spec §6.3](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-11 | Duplicate detection — real-time `409` on create, explicit check endpoint, batch deduplicate scan | service (UI: front-end) | [spec §6.4](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-12 | Merge — transfer, alternate-name aliasing, `Replaces` link, soft delete, JSON snapshot, `Merged` event | service (UI: front-end) | [spec §6.4](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-13 | Review queue — `Pending` / `Confirmed` / `Rejected` / `AutoMerged` | service | [spec §6.4](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-14 | Privacy — phone/fax masking, coordinate rounding to 2 dp, GDPR Art. 15 export, consent records | service (UI deferred: front-end §13) | [spec §6.6](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-15 | Audit — every CRUD / merge / link writes `audit_log` (old + new JSON, user, IP, agent, timestamp) + audit query API | service (view: front-end) | [spec §6.7](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-16 | Event streaming — automatic event publish on every CRUD / merge | service | [spec §6.1](../place-service-with-loco/spec/06-functional-requirements.md) |
| FR-17 | Operator UI — list/search, create with 409 surfacing, detail/edit/delete, match check, merge, per-place audit view | front-end | [spec §6](../place-front-end-with-svelte/spec/06-functional-requirements.md) |
| FR-18 | API documentation — OpenAPI 3.0 + Swagger UI at `/swagger-ui` | service | [spec §9](../place-service-with-loco/spec/09-api-surface.md) |

### 6.1 Integration contract requirements (owned here)

- **FR-19 — Adapter fidelity.** The service MUST score duplicate
  candidates through the canonical matcher via
  `adapter::to_matcher_place`; the routing rules in
  [§5.3](05-domain-model.md) are normative and MUST stay pinned by the
  bridge test suite.
- **FR-20 — Wire-type fidelity.** Front-end `src/lib/api/types.ts`
  MUST mirror the service wire format; a service field change and the
  corresponding front-end type change land in the same change cycle.
- **FR-21 — Score transparency end to end.** The per-component
  breakdown produced by the matcher MUST survive the service API
  response and be rendered by the front-end match/merge views — no
  layer may collapse it to a bare score.
