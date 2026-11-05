## 2. Scope

### 2.1 In scope

- Thing identity CRUD with soft delete and full audit trail.
- schema.org/Thing canonical properties (`name`, `alternateName`,
  `description`, `disambiguatingDescription`, `additionalType`, `url`,
  `identifier`, `image`, `mainEntityOfPage`, `owner`, `sameAs`,
  `subjectOf`, `potentialAction`).
- Typed identifiers via `PropertyValue` shape.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + boolean search.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (Axum). No gRPC — see §2.2.
- HL7 FHIR R5 surface mapping a Thing to the FHIR `Device` resource
  (`medium` fidelity; T-9, landed 2026-07-07) — supersedes an earlier
  "not a FHIR-resource concern" call; see
  [`agents/share/fhir.md`](../../../agents/share/fhir.md).
- Durable event bus: transactional outbox (Phase 2) + relay/retention
  loop (Phase 3) + real-broker `FluvioSink` behind the `fluvio` Cargo
  feature (T-10, BUS-3) — the sink exists and is tested but is not
  wired to a live deployment target; see §2.2.
- Row-level integrity digests (SHA-256/SHA-3/optional MAC) + verify
  endpoints (2026-07-28).
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- gRPC — the Tonic dependency and `GRPC_PORT` setting are unwired
  scaffolding; no `.proto` or server code exists (T-3).
- Wiring the landed `FluvioSink` to a real broker in a deployment —
  only case-service's producer is live today (`agents/share/overview.md`
  footnote 4); this crate's sink is idle until `THING_FLUVIO_ENDPOINT`
  points at one.
- Bulk import / export (T-8).
- ML-based match scoring.
- File / blob storage for image URLs (`image[]` holds URLs, not bytes).

