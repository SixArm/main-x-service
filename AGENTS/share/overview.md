# Project overview

The **Main X Index** family of crates implements a federated identity index — one crate per domain entity, sharing the same architecture, matching algorithms, and operational conventions.

### Service crates

| Crate | Entity | Purpose |
|-------|--------|---------|
| [person-service](../../person/person-service-with-loco) | Person | General person identity registry |
| [worker-service](../../worker/worker-service-with-loco) | Worker | Workforce / professional identity registry; also records aptitude / personality / psychometric / selection assessments per worker (per-scale results, score bands, validity, derived profile, masked under ABAC) |
| [place-service](../../place/place-service-with-loco) | Place | Geographic place registry (schema.org/Place) |
| [thing-service](../../thing/thing-service-with-loco) | Thing | Generic thing / asset registry (schema.org/Thing) |
| [event-service](../../event/event-service-with-loco) | Event | Time-bounded event registry (schema.org/Event) |
| [course-service](../../course/course-service-with-loco) | Course | Course-identity registry (schema.org/Course) — template + `CourseInstance` sub-resource for specific offerings |
| [authentication-service](../../authentication/authentication-service-with-loco) | User | Central single sign-on provider — passwordless email magic-link auth, Postgres-backed cookie sessions, PASETO v4.public token issuance, `/.well-known/paseto-keys` for offline verification by peers. The first real loco.rs crate and the reference for converting the others. |
| [organization-service](../../organization/organization-service-with-loco) | Organization | loco.rs registry for schema.org/Organization — CRUD + matching (embeds organization-matcher; API DTO is the matcher's Organization type) + Tantivy full-text search (fuzzy + phonetic; duplicate-check blocks on the index) + audit log + event streaming + OpenAPI/Swagger + record merge + offline PASETO v4.public verification + blanket ABAC guard (`ORGANIZATION_REQUIRE_AUTH`, default-off) + durable outbox events (default memory) + field masking, masked view, and the audited GDPR export wired to the ABAC `mask` obligation (no consent model — an organization is not a data subject). |
| [care-pathway-service](../../care-pathway/care-pathway-service-with-loco) | Care pathway | loco.rs registry for clinical care pathways — CRUD + Tantivy full-text search (fuzzy + phonetic; condition codes and interventions searchable; duplicate-check blocks on the index) + matching (embeds care-pathway-matcher; API DTO is the matcher's CarePathway type) + condition-code validation + OpenAPI/Swagger + audit log + durable outbox event streaming (default memory) + offline PASETO v4.public verification + record merge + blanket ABAC guard (`CARE_PATHWAY_REQUIRE_AUTH`, default-off). Deferred: privacy, front-end merge action. |
| [case-service](../../case/case-service-with-loco) | Case | loco.rs registry for governmental cases (case tracking — benefits, legal, social-services, complaints, appeals, investigations) — CRUD + Tantivy full-text/fuzzy/phonetic search (title, subjects, agency, identifiers) with search-blocked duplicate candidates + matching (embeds case-matcher; API DTO is the matcher's Case type) + validation + OpenAPI/Swagger + audit log + durable outbox event streaming (default memory) + offline PASETO v4.public verification + record merge + record-level ABAC + masking obligations + cross-service links (`subject_of`). Case data is personal data; deferred: per-field privacy masking + GDPR export. |
| [project-portfolio-management-service](../../project-portfolio-management/project-portfolio-management-service-with-loco) | Plan | loco.rs registry for **plans** in one recursive collection (any plan may contain any other via `parent_ref`; the former Portfolio / Project / Product / Program are now an optional descriptive `kind` label) — CRUD + matching (embeds project-portfolio-management-matcher; API DTO is the matcher's Plan type; matching is **not** gated by kind) + **operational sub-resources** under `/api/plans/{pid}/…` (goals / tasks / issues + derived timeline & burndown views) + Tantivy full-text/fuzzy/phonetic name search (`kind` is a search filter, never a matching gate) + containment cycle check + OpenAPI/Swagger + audit log + event streaming + offline PASETO v4 public verification + record merge + cross-service links + bulk import/export. Integrates the central auth (users/SSO), person/worker (people refs), organization (sponsor). |

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
| [project-portfolio-management-matcher](../../project-portfolio-management/project-portfolio-management-matcher-rust-crate) | Plan | Plan matching (one recursive type; `kind` is an optional label, **no kind gate** — any two plans may match) — name (Jaro-Winkler + Soundex), goal-title & keywords Jaccard, owner-scoped code, owner org, parent plan (`parent_ref`), timeframe proximity (Gaussian decay), relationships & tags Jaccard; deterministic short-circuits on Jira / Asana / Trello / MS-Project / GitHub / Linear ids / URI / UUID, same-owner code, sameAs URL |

### Library crates

Peer-side support libraries — not services, not matchers. Dependency-light
and published to crates.io for downstream consumers.

| Crate | Entity | Purpose |
|-------|--------|---------|
| [authentication-verifier](../../authentication/authentication-verifier-rust-crate) | User | Peer-side **offline PASETO v4.public (Ed25519) verification** for the [authentication-service](../../authentication/authentication-service-with-loco). Fetches/holds the service's published keys from `/.well-known/paseto-keys` (`Verifier::from_paseto_keys_value` / `from_paseto_keys_url` behind the `fetch` feature), mirrors the `Claims` shape, and verifies `kid` / `iss` / `aud` / `exp` with no shared secret and no introspection hop. Published to crates.io as `authentication-verifier` (0.2); embedded by the sibling services' `src/auth.rs`. |
| [integrity-mac](../../integrity/integrity-mac-rust-crate) | — | Keyed integrity MACs (HMAC-SHA256) with HKDF domain separation and production-grade key handling — one audited implementation embedded by every service with a tamper-evidence tier (person, worker, care-pathway, case). The MAC is the only stored integrity value an adversary holding just the database cannot forge, since the SHA-256 and SHA-3 pre-image formats are published. |

### Cross-cutting services

Infrastructure services that span the entity trios rather than owning one
matcher-backed entity.

| Service | Purpose |
|---------|---------|
| [link-graph-service-with-loco](../../link/link-graph-service-with-loco) | Read-model **aggregator** for cross-service entity linking. Consumes every entity's event stream plus the new `linked`/`unlinked` events and serves the queryable cross-service graph (`neighbors` / `single-view` / freshness). The read side of the hybrid topology in [cross-service-linking.md](cross-service-linking.md); each entity service owns its link **writes** (`entity_links` + events). v1 edges: `same_identity` (person↔worker), `works_at`/`member_of` (person→org), `employed_by` (worker→org), `subject_of` (case→person). Cross-service links are deliberately **not** a matcher signal (separate from within-entity `relationships`). |

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
| [event-front-end-with-svelte](../../event/event-front-end-with-svelte) | event-service | Operator UI for Event CRUD / search / match / merge / audit (time window + Location union + Party / Offer) — calls under `/api/` |
| [course-front-end-with-svelte](../../course/course-front-end-with-svelte) | course-service | Operator UI for Course CRUD / search / match / merge / audit (schema.org/Course: course code, educational level, keywords, teaches, syllabus sections, instances sub-resource) |
| [authentication-front-end-with-svelte](../../authentication/authentication-front-end-with-svelte) | authentication-service | Operator UI for passwordless magic-link sign up / sign in / sign out (no data grid; deliberately dependency-light) |
| [organization-front-end-with-svelte](../../organization/organization-front-end-with-svelte) | organization-service | Operator UI for Organization CRUD + duplicate-check (schema.org/Organization: identifiers, address, jurisdiction; dependency-light, no data grid) |
| [care-pathway-front-end-with-svelte](../../care-pathway/care-pathway-front-end-with-svelte) | care-pathway-service | Operator UI for clinical care-pathway CRUD + duplicate-check (condition codes, care setting, interventions; dependency-light, no data grid) |
| [case-front-end-with-svelte](../../case/case-front-end-with-svelte) | case-service | Operator UI for governmental case CRUD + duplicate-check (title, agency, case number, type/status/priority, subjects, identifiers; dependency-light, no data grid; vitest + Playwright tests) |
| [project-portfolio-management-front-end-with-svelte](../../project-portfolio-management/project-portfolio-management-front-end-with-svelte) | project-portfolio-management-service | Operator UI for **plan** CRUD / search / match / merge / audit over one recursive collection (`kind` an optional label) **plus project-management views** — Kanban task board, issues, Gantt/timeline, burndown chart, goals (SVAR DataGrid + Lily; top-nav hamburger; 13-locale i18n) |

Per-project decision (2026-06-02): drift between front-ends is accepted; there is no shared `mxi-svelte-core` package. Copy-adapt from a sibling when scaffolding a new front-end.

## What every crate provides

> **Honest capability matrix** (task H-2). Grounded by grepping the tree:
> an ✅ means the capability has a live `src/` module in that crate **today**;
> `–` means it does not (some are planned — see each crate's `spec/§13`).
> This replaces an earlier "every crate provides everything" list that
> overclaimed (it advertised Tantivy, gRPC, privacy, and bulk for all crates
> when each is only a subset).

### The common baseline (all ten entity registries)

person, worker, place, thing, event, course, organization, care-pathway,
case, and portfolio each provide:

- **CRUD** on the domain entity with **soft-delete**
- **Data-quality validation** (required fields, format/range checks) → `422`
- **Matching** — probabilistic (weighted, configurable) **and** deterministic
  (short-circuit rules), embedding the sibling `*-matcher` crate — plus
  **duplicate detection** (real-time on create + batch scan) and **record
  merge** with transferred-data snapshots
- **Audit log** + an **in-memory event stream** of every CRUD/merge
- **REST API** (Axum via loco) with **OpenAPI / Swagger**
- **Offline PASETO v4.public verification** + the blanket **ABAC guard**
  (`<ENTITY>_REQUIRE_AUTH`, default-off), via the shared
  `authentication-verifier`
- **Observability** — structured `tracing` and a Prometheus
  `/metrics.prom` endpoint. **Not** OpenTelemetry export: person, worker
  and event carry an `src/observability/` module that builds an OTel
  `Resource` and then installs a plain JSON subscriber, with the exporter
  commented out (`// TODO: Initialize OTLP exporter`); no service exports
  a span or a metric over OTLP today (verified 2026-08-01)
- **PostgreSQL** persistence via SeaORM + migrations

### Capabilities that vary by crate

| Capability | person | worker | place | thing | event | course | org | care-pathway | case | portfolio |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| Full-text search via Tantivy¹ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Privacy masking module (`src/privacy`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | – | – |
| FHIR R5 surface | ✅ | ✅ | ✅ | ✅ | ✅ | – | ✅ | ✅ | ✅ | – |
| gRPC stub (Tonic) | ✅ | ✅ | – | – | ✅ | – | – | – | – | – |
| Durable outbox events (Phase 2)² | ✅ | ✅ | ✅ | ✅ | ✅ | – | ✅ | ✅ | ✅ | ✅ |
| Boundary normalization (phone/address) | ✅ | ✅ | ✅ | – | ✅ | – | – | – | – | – |
| Record-level ABAC + masking obligations | ✅ | ✅ | – | – | – | – | ✅ | ✅ | ✅ | – |
| Cross-service links (`entity_links` write-side) | ✅ | ✅ | – | – | – | – | – | – | ✅ | – |
| Bulk import/export | ✅ | – | – | – | – | – | – | – | – | – |

¹ Every entity registry now indexes via Tantivy (fuzzy + phonetic
retrieval, duplicate-check candidates blocked on the index rather than
scanned): organization on 2026-07-31, care-pathway on 2026-08-01, and
case + portfolio on 2026-08-02 (the last two). Portfolio additionally
indexes its optional `kind` label as a **search** filter (`?kind=`) —
deliberately never a duplicate-detection gate, since the embedded
matcher is kind-agnostic by design. Course additionally serves a
non-R5 FHIR surface (`/fhir/Basic` — no FHIR R5 resource models a
course), which the FHIR R5 row deliberately does not count. ² course emits **in-memory events only**
(no durable outbox yet); every durable-outbox service defaults to
`<ENTITY>_EVENT_TRANSPORT=memory`.

### The two cross-cutting services

These are **not** entity registries and share little of the matrix above:

- **authentication-service** — the central SSO provider: passwordless
  magic-link login, Postgres cookie sessions, PASETO v4.public **issuance**
  + `/.well-known/paseto-keys`, ABAC attribute sourcing, and an `auth_events`
  audit trail. No matching / FHIR / Tantivy.
- **link-graph-service** — the read-model **aggregator**, read-only to the
  world: it consumes every entity's event stream, serves the cross-service
  graph (`neighbors` / `single-view` / freshness), reconciles against each
  service's `entity_links`, audits, and sits behind the blanket guard. No
  CRUD writes / matching / FHIR.

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
