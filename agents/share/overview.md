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
| [event-matcher](../../event/event-matcher-rust-crate) | Event | Time-bounded event matching — `start_date`/`end_date` scored independently by Gaussian decay over endpoint distance; scoring the `[start, end]` window-overlap fraction instead is an open question (event-matcher `spec/10-open-questions.md` OQ-C), not implemented |
| [course-matcher](../../course/course-matcher-rust-crate) | Course | Course matching — name (Jaro-Winkler), provider-scoped course code, educational level, keywords / teaches Jaccard, deterministic short-circuits on DOI / Wikidata / OER / LOM / URI / UUID |
| [organization-matcher](../../organization/organization-matcher-rust-crate) | Organization | Organization matching — legal-suffix-aware name, postal address, url/domain, jurisdiction, founding date, keywords; deterministic short-circuits on LEI / DUNS / ISO 6523 / GLN / Wikidata / ROR / ISNI / VAT, same-jurisdiction tax id, sameAs URL |
| [care-pathway-matcher](../../care-pathway/care-pathway-matcher-rust-crate) | Care pathway | Clinical care-pathway matching — name (Jaro-Winkler), target condition codes (ICD/SNOMED Jaccard), provider-scoped pathway code, care setting, interventions / keywords Jaccard; deterministic short-circuits on DOI / Wikidata / guideline-id / URI / UUID, same-provider pathway code, sameAs URL |
| [case-matcher](../../case/case-matcher-rust-crate) | Case | Governmental case matching — title (Jaro-Winkler + Soundex), subjects / keywords Jaccard, agency-scoped case number, case type / status; deterministic short-circuits on Docket / external-case-id / URI / UUID, same-agency case number, sameAs URL |
| [project-portfolio-management-matcher](../../project-portfolio-management/project-portfolio-management-matcher-rust-crate) | Plan | Plan matching (one recursive type; `kind` is an optional label, **no kind gate** — any two plans may match) — name (Jaro-Winkler + Soundex), goal-title & keywords Jaccard, owner-scoped code, owner org, parent plan (`parent_ref`), timeframe proximity (Gaussian decay), relationships & tags Jaccard; deterministic short-circuits on Jira / Asana / Trello / MS-Project / GitHub / Linear ids / URI / UUID, same-owner code, sameAs URL |

### Library crates

Peer-side support libraries — not services, not matchers. Dependency-light,
and published to crates.io for downstream consumers where noted below —
two of the three are; `integrity-mac` is not yet (see its row).

| Crate | Entity | Purpose |
|-------|--------|---------|
| [authentication-verifier](../../authentication/authentication-verifier-rust-crate) | User | Peer-side **offline PASETO v4.public (Ed25519) verification** for the [authentication-service](../../authentication/authentication-service-with-loco). Fetches/holds the service's published keys from `/.well-known/paseto-keys` (`Verifier::from_paseto_keys_value` / `from_paseto_keys_url` behind the `fetch` feature), mirrors the `Claims` shape, and verifies `kid` / `iss` / `aud` / `exp` with no shared secret and no introspection hop. Published to crates.io as `authentication-verifier` (0.9); embedded by the sibling services' `src/auth.rs`. |
| [integrity-mac](../../integrity/integrity-mac-rust-crate) | — | Keyed integrity MACs (HMAC-SHA256) with HKDF domain separation and production-grade key handling — one audited implementation, embedded family-wide (all ten entity registries, authentication-service, and link-graph-service): person/worker/care-pathway/case first, then organization/place/thing/portfolio/event/course/authentication/link-graph (landed through 2026-07-28). The MAC is the only stored integrity value an adversary holding just the database cannot forge, since the SHA-256 and SHA-3 pre-image formats are published. Only person, worker, care-pathway, and case additionally chain their audit rows (`prev_hash`/`hash`) with external-witness checkpoints — the rest verify row content, not deletion/reordering. **Not published to crates.io** — every consumer is in-tree via a Cargo `path` dependency; unlike `entity-ref` below, no release has been cut. |
| [entity-ref](../../link/entity-ref-rust-crate) | — | The [cross-service-linking](cross-service-linking.md) contract: `EntityRef` (the `entity_type:uuid` URN) + the closed v1 `EdgeKind` registry. The design doc (§2/§3) framed it as copy-per-project; in practice it is a real Cargo `path` dependency of eight crates as of 2026-08-04 — the `link-graph-service-with-loco` aggregator, the three edge-originating services (person, worker, case), and four consumer apps (contact-relationship-management, content-management-system, patient-flow, workforce-planning-management) that validate/dereference refs without originating edges. **Published to crates.io as `entity-ref` (0.2.0, 2026-08-05)** — correcting an earlier "not yet published" claim here; every in-tree consumer above still takes the Cargo `path` dependency rather than the crates.io release (cross-service-linking.md §12), so the publish changed nothing about how the family actually consumes it. |

### Cross-cutting services

Infrastructure services that span the entity trios rather than owning one
matcher-backed entity.

| Service | Purpose |
|---------|---------|
| [link-graph-service-with-loco](../../link/link-graph-service-with-loco) | Read-model **aggregator** for cross-service entity linking. Consumes every entity's event stream plus the new `linked`/`unlinked` events and serves the queryable cross-service graph (`neighbors` / `single-view` / freshness). The read side of the hybrid topology in [cross-service-linking.md](cross-service-linking.md); each entity service owns its link **writes** (`entity_links` + events). v1 edges: `same_identity` (person↔worker), `works_at`/`member_of` (person→org), `employed_by` (worker→org), `subject_of` (case→person), `continues_as` (a care-pathway instance into the next episode — the journey edge time-based analysis follows across a service boundary). Cross-service links are deliberately **not** a matcher signal (separate from within-entity `relationships`). |

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
  `/metrics.prom` endpoint. OpenTelemetry OTLP export is **rolling out
  but not yet family-wide** (repo `tasks.md` PRO-H9, subsuming the
  earlier AU-3): as of 2026-08-28, **person** carries a real exporter
  (`src/observability.rs`, ported from link-graph-service — see below),
  while **worker and event** still carry the original `src/observability/`
  stub that builds an OTel `Resource` and then installs a plain JSON
  subscriber, with the exporter commented out
  (`// TODO: Initialize OTLP exporter`); the other seven registries have
  no such module at all. Copy person's `src/observability.rs` for worker
  and event, not their own stub — see that file's module docs and
  person's `AGENTS.md` "OpenTelemetry OTLP export" section for the two
  adaptations person's port needed beyond link-graph-service's shape (the
  tower middleware wired onto **two** router-construction surfaces
  instead of one, and a renamed `tonic` dev-dependency to avoid an
  extern-prelude collision with the crate's own gRPC-stub dependency —
  both apply to worker and event too, since both also carry a `tonic`
  gRPC stub). The cross-cutting **link-graph-service** carries the
  original reference (`src/observability.rs`: OTLP/gRPC traces + metrics
  bridged from `tracing` through loco's `Hooks::init_logger` seam, with a
  per-request span, an `http.server.request.duration` histogram, and a
  W3C `traceparent` response header, proved against a real in-process
  collector). Rolling this across the remaining nine registries (seven
  with no module yet, plus worker and event's stubs) is queued work.
- **PostgreSQL** persistence via SeaORM + migrations

### Capabilities that vary by crate

| Capability | person | worker | place | thing | event | course | org | care-pathway | case | portfolio |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| Full-text search via Tantivy¹ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Privacy masking module (`src/privacy`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | – | ✅ |
| FHIR R5 surface | ✅ | ✅ | ✅ | ✅ | ✅ | – | ✅ | ✅ | ✅ | – |
| gRPC stub (Tonic) | ✅ | ✅ | – | – | ✅ | – | – | – | – | – |
| Durable outbox events (Phase 2)² | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Real-broker relay sink (`FluvioSink`, Phase 3)⁴ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Boundary normalization (phone/address) | ✅ | ✅ | ✅ | – | ✅ | – | – | – | – | – |
| Record-level ABAC + masking obligations | ✅ | ✅ | – | – | – | – | ✅ | ✅ | ✅ | ✅ |
| Cross-service links (`entity_links` write-side) | ✅ | ✅ | – | – | – | – | – | ✅ | ✅ | – |
| Bulk import/export³ | ✅ | – | – | – | – | – | ✅ | – | ✅ | – |
| Time-based analysis⁵ | – | – | – | – | – | – | – | ✅ | – | ✅ |

> The consumer application **patient-flow** also participates, though
> it is not one of the ten registries and so has no column above: it
> serves the stitched-journey timeline contract from
> `GET /api/stays/{pid}/time-analysis`, deriving value-adding time from
> its `Red2Green` day classifications.

¹ Every entity registry now indexes via Tantivy (fuzzy + phonetic
retrieval, duplicate-check candidates blocked on the index rather than
scanned): organization on 2026-07-31, care-pathway on 2026-08-01, and
case + portfolio on 2026-08-02 (the last two). Portfolio additionally
indexes its optional `kind` label as a **search** filter (`?kind=`) —
deliberately never a duplicate-detection gate, since the embedded
matcher is kind-agnostic by design. Course additionally serves a
non-R5 FHIR surface (`/fhir/Basic` — no FHIR R5 resource models a
course), which the FHIR R5 row deliberately does not count. ² All ten
entity registries carry a durable transactional outbox (course was
verified 2026-08-03, correcting an earlier "in-memory events only"
claim here — it has its own `course_outbox` table, a working
`EventTransport::Outbox` switch, and a wired `src/relay.rs`); every
durable-outbox service defaults to `<ENTITY>_EVENT_TRANSPORT=memory`,
so none of this changes default behaviour. ³ Organization and case landed
2026-08-03 (BLK-5), scoped to **JSONL + CSV only** (no Parquet) and a
**local-filesystem-only** artifact store (no S3 backend yet — unlike
person's BLK-3/BLK-4). Organization's per-row upsert is not yet
SEC-B3 advisory-lock-protected (a documented, narrow TOCTOU gap — see
its own spec §10.7); case's bulk export reuses its existing inline
`mask_case` redaction rather than a dedicated privacy module, so the
case ✗ in the privacy-masking row above does not mean its bulk export
is unmasked. ⁴ `FluvioSink` — the durable
bus's real-broker relay sink,
alongside the always-available no-broker `LoggingSink` — is behind each
crate's own `fluvio` Cargo feature (off by default) and gated further
by `<ENTITY>_FLUVIO_ENDPOINT`; unset ⇒ unchanged `LoggingSink`
behaviour, and an endpoint configured without the feature refuses to
start the relay (logged `error`) rather than silently falling back.
Landed case (BUS-1, 2026-08-02) then the other nine (BUS-3, 2026-08-03),
each with an opt-in `compose.fluvio.yaml` local broker and a
feature-gated, `#[ignore]`d live-broker round-trip test verified only
by compiling under the feature — no automated run in this repo stands
up a broker. Only case's producer side is wired to a real deployment
target today; the link-graph aggregator (BUS-2) already consumes all
ten topics, so the other nine sinks are live but currently idle until
a deployment actually points `<ENTITY>_FLUVIO_ENDPOINT` at a broker.

⁵ **Time-based analysis** (`src/tba.rs` + `src/controllers/tba.rs`)
measures elapsed calendar time through a process — the value-adding
ratio, constraint ranking, and queueing-theory flow — per
[time-based-analysis.md](time-based-analysis.md). Care-pathway (landed
2026-08-23) records journey segments by hand and scores cohorts against
NHS access standards; portfolio (2026-08-23/24) derives its intervals
from a task-transition log written by the existing board-move endpoint,
and adds rework/first-pass yield, throughput-based Monte-Carlo
forecasting, and a cross-plan rollup. Both ship a default-off Prometheus
gauge family. The other eight registries carry none, and that is a
scope decision rather than a gap: TBA needs a unit that enters, waits,
is worked on and leaves, and a registry of *identities* has records
rather than journeys (see that doc's §12).


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
  CRUD writes / matching / FHIR. Carries the family's original working
  OpenTelemetry OTLP exporter (2026-08-05, `src/observability.rs`) — the
  reference person's own exporter (2026-08-28, PRO-H9) was ported from,
  and worker's / event's commented-out stubs should be replaced with next.

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
