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
- REST API (Axum) + gRPC stub.
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- FHIR R5 — Things are not a FHIR-resource concern.
- Production Fluvio publisher / consumers.
- ML-based match scoring.
- File / blob storage for image URLs (`image[]` holds URLs, not bytes).

