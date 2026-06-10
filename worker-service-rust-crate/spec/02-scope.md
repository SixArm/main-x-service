## 2. Scope

### 2.1 In scope

- Worker identity CRUD with soft delete and full audit trail.
- Multiple identifiers per record (NPI, DEA, professional licence,
  MRN-style employee number, SSN, DL, TAX, Other).
- Identity / credential documents with type, number, issuing
  authority, issue / expiry dates, verified flag.
- Multiple addresses, telecom contacts, emergency contacts.
- Demographics (gender, birth date, marital status, multiple birth,
  deceased, photo).
- Managing organisation + per-worker links.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + phonetic search.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (Axum) + FHIR R5 Practitioner + gRPC stub.
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- Authentication / authorisation middleware (planned — §15).
- Production Fluvio publisher / consumers (today: in-memory stub).
- FHIR Organization resource and capability statement / bundles
  (Practitioner ✔; supporting resources partial).
- ML-based match scoring.
- Credential-expiry workflow / alerting (roadmap, §15).
- Role + assignment history timeline.

