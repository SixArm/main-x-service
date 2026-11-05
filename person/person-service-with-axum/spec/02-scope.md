## 2. Scope

### 2.1 In scope

- Person identity CRUD with soft delete and full audit trail.
- Multiple identifiers per record (MRN, SSN, DL, NPI, PPN, TAX, Other).
- Identity documents (passport, driver's licence, national ID, …).
- Multiple addresses, telecom contacts, emergency contacts.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + phonetic search.
- Real-time + batch duplicate detection with review queue + auto-merge.
- Record merging with link tracking and transferred-data snapshots.
- Data validation + phone / address normalisation at the boundary.
- Per-field privacy masking, GDPR Article 15 export, consent model.
- REST API (Axum) + FHIR R5 Person + gRPC (Create/Get/List/Delete Person).
- PostgreSQL persistence via SeaORM, with migrations.
- Observability (tracing + OpenTelemetry OTLP).

### 2.2 Out of scope (today)

- Authentication / authorisation middleware (planned — §15).
- Production Fluvio publisher / consumers (today: in-memory stub).
- Complete FHIR bundle handling (Person resource ✔; bundles partial).
- ML-based match scoring.
- Person photo storage and retrieval.

