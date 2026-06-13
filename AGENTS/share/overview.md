# Project overview

The **Main X Index** family of crates implements a federated identity index — one crate per domain entity, sharing the same architecture, matching algorithms, and operational conventions.

### Service crates

| Crate | Entity | Purpose |
|-------|--------|---------|
| [person-service](../../person/person-service-rust-crate) | Person | General person identity registry |
| [worker-service](../../worker/worker-service-rust-crate) | Worker | Workforce / professional identity registry |
| [place-service](../../place/place-service-rust-crate) | Place | Geographic place registry (schema.org/Place) |
| [thing-service](../../thing/thing-service-rust-crate) | Thing | Generic thing / asset registry (schema.org/Thing) |
| [event-service](../../event/event-service-rust-crate) | Event | Time-bounded event registry (schema.org/Event) |
| [course-service](../../course/course-service-rust-crate) | Course | Course-identity registry (schema.org/Course) — template + `CourseInstance` sub-resource for specific offerings |
| [authentication-service](../../authentication/authentication-service-rust-crate) | User | Central single sign-on provider — passwordless email magic-link auth, RS256 JWT issuance, JWKS for offline verification by peers. The first real loco.rs crate and the reference for converting the others. |
| [organization-service](../../organization/organization-service-rust-crate) | Organization | loco.rs registry for schema.org/Organization — CRUD + matching (embeds organization-matcher; API DTO is the matcher's Organization type) + name search + audit log + event streaming + OpenAPI/Swagger + record merge + offline RS256 JWT verification. Deferred: Tantivy full-text, privacy, blanket JWT enforcement. |
| [care-pathway-service](../../care-pathway/care-pathway-service-rust-crate) | Care pathway | loco.rs registry for clinical care pathways — CRUD + ILIKE name search + matching (embeds care-pathway-matcher; API DTO is the matcher's CarePathway type) + condition-code validation + OpenAPI/Swagger + audit log + in-memory event streaming + offline RS256 JWT verification + record merge. Deferred: Tantivy full-text search, durable event bus, privacy, front-end merge action, blanket JWT enforcement. |
| [case-service](../../case/case-service-rust-crate) | Case | loco.rs registry for governmental cases (case tracking — benefits, legal, social-services, complaints, appeals, investigations) — CRUD + ILIKE title search + matching (embeds case-matcher; API DTO is the matcher's Case type) + validation + OpenAPI/Swagger + audit log + in-memory event streaming + offline RS256 JWT verification + record merge. Case data is personal data; deferred: per-field privacy masking + GDPR export, Tantivy full-text, durable event bus, blanket JWT enforcement. |

### Matcher crates

Reusable, dependency-light Rust libraries for pairwise record
comparison. Each is usable standalone and is the canonical reference
implementation embedded in the corresponding service crate's
`src/matching/` layer. Their per-crate `spec.md` follows a distinct
§1–§25 SDD shape (research basis, algorithm specifications, normalization
specifications, …) tailored to library-style work.

| Crate | Entity | Purpose |
|-------|--------|---------|
| [person-matcher](../../person/person-matcher-rust-crate) | Person | Demographic + multinational national-identifier matching |
| [worker-matcher](../../worker/worker-matcher-rust-crate) | Worker | Workforce / professional identity matching |
| [place-matcher](../../place/place-matcher-rust-crate) | Place | Geographic / postal-address / venue matching |
| [thing-matcher](../../thing/thing-matcher-rust-crate) | Thing | Generic thing / asset matching |
| [event-matcher](../../event/event-matcher-rust-crate) | Event | Time-bounded event matching with window-overlap |
| [course-matcher](../../course/course-matcher-rust-crate) | Course | Course matching — name (Jaro-Winkler), provider-scoped course code, educational level, keywords / teaches Jaccard, deterministic short-circuits on DOI / Wikidata / OER / LOM / URI / UUID |
| [organization-matcher](../../organization/organization-matcher-rust-crate) | Organization | Organization matching — legal-suffix-aware name, postal address, url/domain, jurisdiction, founding date, keywords; deterministic short-circuits on LEI / DUNS / ISO 6523 / GLN / Wikidata / ROR / ISNI / VAT, same-jurisdiction tax id, sameAs URL |
| [care-pathway-matcher](../../care-pathway/care-pathway-matcher-rust-crate) | Care pathway | Clinical care-pathway matching — name (Jaro-Winkler), target condition codes (ICD/SNOMED Jaccard), provider-scoped pathway code, care setting, interventions / keywords Jaccard; deterministic short-circuits on DOI / Wikidata / guideline-id / URI / UUID, same-provider pathway code, sameAs URL |
| [case-matcher](../../case/case-matcher-rust-crate) | Case | Governmental case matching — title (Jaro-Winkler + Soundex), subjects / keywords Jaccard, agency-scoped case number, case type / status; deterministic short-circuits on Docket / external-case-id / URI / UUID, same-agency case number, sameAs URL |

### Library crates

Peer-side support libraries — not services, not matchers. Dependency-light
and published to crates.io for downstream consumers.

| Crate | Entity | Purpose |
|-------|--------|---------|
| [authentication-verifier](../../authentication/authentication-verifier-rust-crate) | User | Peer-side **offline RS256 JWT verification** for the [authentication-service](../../authentication/authentication-service-rust-crate). Fetches/holds the service's JWKS (`Verifier::from_jwks_value` / `from_jwks_url` behind the `fetch` feature), mirrors the `Claims` shape, and verifies `kid` / `iss` / `aud` / `exp` with no shared secret and no introspection hop. Published to crates.io as `authentication-verifier` (0.1); embedded by the sibling services' `src/auth.rs`. |

### Front-end projects

Operator-facing web UIs — one independent SvelteKit SPA per entity,
calling the sibling service's REST API. Stack: SvelteKit 2, Svelte 5
runes, SVAR Svelte DataGrid, Lily Design System Svelte Headless,
TypeScript strict. Per-project `spec.md` follows the same §1–§18 SDD
shape as the service crates.

| Project | Consumes | Purpose |
|---|---|---|
| [person-front-end-with-svelte](../../person/person-front-end-with-svelte) | person-service | Operator UI for Person CRUD / search / match / merge / audit |
| [worker-front-end-with-svelte](../../worker/worker-front-end-with-svelte) | worker-service | Operator UI for Worker CRUD / search / match / merge / audit |
| [place-front-end-with-svelte](../../place/place-front-end-with-svelte) | place-service | Operator UI for Place CRUD / search / match / merge / audit (PostalAddress + GeoCoordinates + GLN) |
| [thing-front-end-with-svelte](../../thing/thing-front-end-with-svelte) | thing-service | Operator UI for Thing CRUD / search / match / merge / audit (PropertyValue identifiers — DOI / ISBN / GTIN / …) |
| [event-front-end-with-svelte](../../event/event-front-end-with-svelte) | event-service | Operator UI for Event CRUD / search / match / merge / audit (time window + Location union + Party / Offer) — calls under `/api/v1/` |
| [course-front-end-with-svelte](../../course/course-front-end-with-svelte) | course-service | Operator UI for Course CRUD / search / match / merge / audit (schema.org/Course: course code, educational level, keywords, teaches, syllabus sections, instances sub-resource) |
| [authentication-front-end-with-svelte](../../authentication/authentication-front-end-with-svelte) | authentication-service | Operator UI for passwordless magic-link sign up / sign in / sign out (no data grid; deliberately dependency-light) |
| [organization-front-end-with-svelte](../../organization/organization-front-end-with-svelte) | organization-service | Operator UI for Organization CRUD + duplicate-check (schema.org/Organization: identifiers, address, jurisdiction; dependency-light, no data grid) |
| [care-pathway-front-end-with-svelte](../../care-pathway/care-pathway-front-end-with-svelte) | care-pathway-service | Operator UI for clinical care-pathway CRUD + duplicate-check (condition codes, care setting, interventions; dependency-light, no data grid) |
| [case-front-end-with-svelte](../../case/case-front-end-with-svelte) | case-service | Operator UI for governmental case CRUD + duplicate-check (title, agency, case number, type/status/priority, subjects, identifiers; dependency-light, no data grid; vitest + Playwright tests) |

Per-project decision (2026-06-02): drift between front-ends is accepted; there is no shared `mxi-svelte-core` package. Copy-adapt from a sibling when scaffolding a new front-end.

## What every crate provides

- **CRUD** on the domain entity with soft-delete and full audit trail
- **Identifier management** (multiple identifiers per record; type + system + value)
- **Identity document management** (passport, driver's license, etc., where relevant)
- **Contact information management** (telecom / address / email)
- **Probabilistic matching** with weighted, configurable scoring
- **Deterministic matching** with short-circuit rules (tax ID, document, GLN, …)
- **Full-text search** via Tantivy with fuzzy and phonetic variants
- **Duplicate detection** (real-time on create, batch via deduplicate scan)
- **Record merging** with link tracking and transferred-data snapshots
- **Data quality validation** (required fields, format checks, ranges)
- **Address & phone normalization** at the boundary
- **Privacy controls**: per-field masking, GDPR data export, consent records
- **Event streaming** of every CRUD operation
- **Audit logging** (HIPAA-style trail for who/what/when)
- **REST API** (Axum) with OpenAPI / Swagger
- **gRPC API** stub (Tonic) for high-throughput callers
- **Observability** (tracing + OpenTelemetry OTLP)
- **PostgreSQL persistence** via SeaORM with migrations

See [rust-loco-stack.md](rust-loco-stack.md) for the dependency stack.

## Running

Every subproject ships the same entry points:

```bash
# REST API server
cargo run --release

# Tests
cargo test --lib

# Benchmarks (where available)
cargo bench
```
