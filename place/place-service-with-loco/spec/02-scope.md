## 2. Scope

### 2.1 In scope

- Place identity CRUD with soft delete and full audit trail.
- schema.org/Place properties (`name`, `alternate_name`,
  `description`, `place_type`, `address`, `geo`, `telephone`,
  `fax_number`, `url`, opening hours, amenities, accessibility flags).
- Multiple identifiers (GLN, FIPS, GNIS, OSM ID, branch_code, custom).
- Hierarchy (`contained_in_place` / `contains_place`).
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + boolean search.
- **Geo-radius search** with Haversine + bounding-box pre-filter.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking (phone / fax / coordinate rounding),
  GDPR Article 15 export, consent records.
- REST API (Axum) + gRPC stub.
- HL7 FHIR R5 API (`Location` resource) — see §9, §13 T-11.
- Keyed integrity verification (SHA-256 + SHA3-256 digests + an
  HMAC-SHA256 MAC, default-off) over place records and the audit log —
  see §12, §13.
- PostgreSQL persistence via SeaORM, with PostGIS for spatial.

### 2.2 Out of scope (today)

- Production Fluvio publisher / consumers.
- PostGIS-backed spatial queries (currently fallback to Haversine +
  bounding-box in app code).
- Recursive CTEs for place-hierarchy depth queries.
- OSM bulk import pipeline.
- Tile-server integration.
- Reverse-geocoding endpoint.

