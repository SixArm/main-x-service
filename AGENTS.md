# Main X Index Rust crates

@agents/share/overview.md

## Subprojects

Each per-crate `spec.md` is the **single source of truth** for that
crate. All service crates follow the same SDD shape (numbered
sections 1–18, with §13 holding the live task queue). See any of the
per-crate `AGENTS/spec-driven-development.md` files for the discipline.

### Service crates

| Crate | Entity | Spec | Index |
|---|---|---|---|
| [Person Service](person/person-service-with-loco/) | Person (general) | [spec](person/person-service-with-loco/spec/index.md) | [index](person/person-service-with-loco/index.md) |
| [Worker Service](worker/worker-service-with-loco/) | Worker (workforce / professional) | [spec](worker/worker-service-with-loco/spec/index.md) | [index](worker/worker-service-with-loco/index.md) |
| [Place Service](place/place-service-with-loco/) | Place (schema.org/Place) | [spec](place/place-service-with-loco/spec/index.md) | [index](place/place-service-with-loco/index.md) |
| [Thing Service](thing/thing-service-with-loco/) | Thing (schema.org/Thing — generic) | [spec](thing/thing-service-with-loco/spec/index.md) | [index](thing/thing-service-with-loco/index.md) |
| [Event Service](event/event-service-with-loco/) | Event (schema.org/Event — time-bounded) | [spec](event/event-service-with-loco/spec/index.md) | [index](event/event-service-with-loco/index.md) |
| [Course Service](course/course-service-with-loco/) | Course (schema.org/Course) — template + `CourseInstance` sub-resource | [spec](course/course-service-with-loco/spec/index.md) | [index](course/course-service-with-loco/index.md) |
| [Authentication Service](authentication/authentication-service-with-loco/) | User (central single sign-on provider) — passwordless magic-link, RS256 JWT + JWKS. **First real loco.rs crate**; reference for converting the others. | [spec](authentication/authentication-service-with-loco/spec/index.md) | [index](authentication/authentication-service-with-loco/index.md) |
| [Organization Service](organization/organization-service-with-loco/) | Organization (schema.org/Organization) — loco.rs CRUD + matching (embeds organization-matcher) + name search + audit log + event streaming + OpenAPI/Swagger + record merge + JWT verification | [spec](organization/organization-service-with-loco/spec/index.md) | [index](organization/organization-service-with-loco/index.md) |
| [Care Pathway Service](care-pathway/care-pathway-service-with-loco/) | Care pathway (clinical pathway) — loco.rs CRUD + name search + matching (embeds care-pathway-matcher) + condition-code validation + OpenAPI/Swagger + audit log + event streaming + JWT verification + record merge | [spec](care-pathway/care-pathway-service-with-loco/spec/index.md) | [index](care-pathway/care-pathway-service-with-loco/index.md) |
| [Case Service](case/case-service-with-loco/) | Case (governmental case management / case tracking) — loco.rs CRUD + title search + matching (embeds case-matcher) + validation + OpenAPI/Swagger + audit log + event streaming + JWT verification + record merge | [spec](case/case-service-with-loco/spec/index.md) | [index](case/case-service-with-loco/index.md) |
| [Plan Service](plan/plan-service-with-loco/) | Plan (project / product / programme / initiative / portfolio / epic) — loco.rs CRUD + matching (embeds plan-matcher; DTO = matcher `Plan` type as JSONB) + **project-management sub-resources** (goals / tasks / issues / posts / comments / members + derived timeline & burndown) + name search + audit + event streaming + OpenAPI/Swagger + record merge + JWT verification + cross-service links + bulk import/export. Integrates auth (users/SSO), person/worker (people), organization (sponsor). | [spec](plan/plan-service-with-loco/spec/index.md) | [index](plan/plan-service-with-loco/index.md) |

### Matcher crates

The matcher crates are reusable, dependency-light Rust libraries for
pairwise record comparison. They follow their own SDD shape (see each
crate's `spec.md` for its full §1–§25 structure), distinct from the
service-crate shape (§1–§18). Use them standalone, or embed them in
the corresponding service crate's matching layer.

| Crate | Entity | Spec | Index |
|---|---|---|---|
| [Person Matcher](person/person-matcher-rust-crate/) | Person | [spec](person/person-matcher-rust-crate/spec/index.md) | [index](person/person-matcher-rust-crate/index.md) |
| [Worker Matcher](worker/worker-matcher-rust-crate/) | Worker | [spec](worker/worker-matcher-rust-crate/spec/index.md) | [index](worker/worker-matcher-rust-crate/index.md) |
| [Place Matcher](place/place-matcher-rust-crate/) | Place | [spec](place/place-matcher-rust-crate/spec/index.md) | [index](place/place-matcher-rust-crate/index.md) |
| [Thing Matcher](thing/thing-matcher-rust-crate/) | Thing | [spec](thing/thing-matcher-rust-crate/spec/index.md) | [index](thing/thing-matcher-rust-crate/index.md) |
| [Event Matcher](event/event-matcher-rust-crate/) | Event | [spec](event/event-matcher-rust-crate/spec/index.md) | [index](event/event-matcher-rust-crate/index.md) |
| [Course Matcher](course/course-matcher-rust-crate/) | Course | [spec](course/course-matcher-rust-crate/spec/index.md) | [index](course/course-matcher-rust-crate/index.md) |
| [Organization Matcher](organization/organization-matcher-rust-crate/) | Organization (schema.org/Organization) | [spec](organization/organization-matcher-rust-crate/spec/index.md) | [index](organization/organization-matcher-rust-crate/index.md) |
| [Care Pathway Matcher](care-pathway/care-pathway-matcher-rust-crate/) | Care pathway (clinical pathway) | [spec](care-pathway/care-pathway-matcher-rust-crate/spec/index.md) | [index](care-pathway/care-pathway-matcher-rust-crate/index.md) |
| [Case Matcher](case/case-matcher-rust-crate/) | Case (governmental case) — title (Jaro-Winkler), subjects/keywords Jaccard, agency-scoped case number, type/status; deterministic short-circuits on Docket / external case id / URI / UUID, same-agency case number, sameAs URL | [spec](case/case-matcher-rust-crate/spec/index.md) | [index](case/case-matcher-rust-crate/index.md) |
| [Plan Matcher](plan/plan-matcher-rust-crate/) | Plan (project / programme / initiative) — name (Jaro-Winkler), goal-title & keywords Jaccard, owner-scoped plan code, owner org, plan type, timeframe proximity, relationships & tags Jaccard; deterministic short-circuits on Jira/Asana/Trello/MS-Project/GitHub/Linear ids / URI / UUID, same-owner plan code, sameAs URL | [spec](plan/plan-matcher-rust-crate/spec/index.md) | [index](plan/plan-matcher-rust-crate/index.md) |

### Library crates

Peer-side support libraries (not services, not matchers). Dependency-light
and published to crates.io for downstream consumers.

| Crate | Entity | Spec | Index |
|---|---|---|---|
| [Authentication Verifier](authentication/authentication-verifier-rust-crate/) | User — peer-side **offline RS256 JWT verification** for the [Authentication Service](authentication/authentication-service-with-loco/); fetches/holds the service's JWKS, mirrors the `Claims` shape, verifies `kid`/`iss`/`aud`/`exp`. Published to crates.io as `authentication-verifier` (0.1). | [spec](authentication/authentication-verifier-rust-crate/spec/index.md) | [index](authentication/authentication-verifier-rust-crate/index.md) |

### Front-end projects

SvelteKit front-ends sit alongside their service crates. Each is an
independent SPA built on SvelteKit 2 + Svelte 5 runes + SVAR Svelte
DataGrid + Lily Design System Svelte Headless, calling the sibling
service's REST API. Their per-project `spec.md` follows the same
§1–§18 SDD shape as the service crates. Drift between front-ends is
accepted (see `feedback_front_end_drift` memory) — no shared package.

| Project | Consumes | Spec | Changelog |
|---|---|---|---|
| [person-front-end-with-svelte](person/person-front-end-with-svelte/) | [person-service](person/person-service-with-loco/) | [spec](person/person-front-end-with-svelte/spec/index.md) | [CHANGELOG](person/person-front-end-with-svelte/CHANGELOG.md) |
| [worker-front-end-with-svelte](worker/worker-front-end-with-svelte/) | [worker-service](worker/worker-service-with-loco/) | [spec](worker/worker-front-end-with-svelte/spec/index.md) | [CHANGELOG](worker/worker-front-end-with-svelte/CHANGELOG.md) |
| [place-front-end-with-svelte](place/place-front-end-with-svelte/) | [place-service](place/place-service-with-loco/) | [spec](place/place-front-end-with-svelte/spec/index.md) | [CHANGELOG](place/place-front-end-with-svelte/CHANGELOG.md) |
| [thing-front-end-with-svelte](thing/thing-front-end-with-svelte/) | [thing-service](thing/thing-service-with-loco/) | [spec](thing/thing-front-end-with-svelte/spec/index.md) | [CHANGELOG](thing/thing-front-end-with-svelte/CHANGELOG.md) |
| [event-front-end-with-svelte](event/event-front-end-with-svelte/) | [event-service](event/event-service-with-loco/) | [spec](event/event-front-end-with-svelte/spec/index.md) | [CHANGELOG](event/event-front-end-with-svelte/CHANGELOG.md) |
| [course-front-end-with-svelte](course/course-front-end-with-svelte/) | [course-service](course/course-service-with-loco/) | [spec](course/course-front-end-with-svelte/spec/index.md) | [CHANGELOG](course/course-front-end-with-svelte/CHANGELOG.md) |
| [authentication-front-end-with-svelte](authentication/authentication-front-end-with-svelte/) | [authentication-service](authentication/authentication-service-with-loco/) | [spec](authentication/authentication-front-end-with-svelte/spec/index.md) | [CHANGELOG](authentication/authentication-front-end-with-svelte/CHANGELOG.md) |
| [organization-front-end-with-svelte](organization/organization-front-end-with-svelte/) | [organization-service](organization/organization-service-with-loco/) | [spec](organization/organization-front-end-with-svelte/spec/index.md) | [CHANGELOG](organization/organization-front-end-with-svelte/CHANGELOG.md) |
| [care-pathway-front-end-with-svelte](care-pathway/care-pathway-front-end-with-svelte/) | [care-pathway-service](care-pathway/care-pathway-service-with-loco/) | [spec](care-pathway/care-pathway-front-end-with-svelte/spec/index.md) | [CHANGELOG](care-pathway/care-pathway-front-end-with-svelte/CHANGELOG.md) |
| [case-front-end-with-svelte](case/case-front-end-with-svelte/) | [case-service](case/case-service-with-loco/) | [spec](case/case-front-end-with-svelte/spec/index.md) | [CHANGELOG](case/case-front-end-with-svelte/CHANGELOG.md) |
| [plan-front-end-with-svelte](plan/plan-front-end-with-svelte/) | [plan-service](plan/plan-service-with-loco/) | [spec](plan/plan-front-end-with-svelte/spec/index.md) | [CHANGELOG](plan/plan-front-end-with-svelte/CHANGELOG.md) |

### Consumer applications

Application subprojects that **consume** the index services rather than
being one of the matcher-backed index entities. They follow a different
internal shape (a cross-cutting `spec/` plus a per-edition service +
front-end) and are not part of the entity trio tables above.

> Note: distinct from the **`case`** index entity above. `case` is a
> matcher-backed registry of case *identities* (deduplicated, matchable);
> **`case-folder`** is an operational app that tracks the physical
> *location* of NHS paper case-note folders, consuming the person /
> place / worker services.

| Project | Purpose | Editions |
|---|---|---|
| [case-folder](case-folder/spec/index.md) | NHS paper case-note folder location tracking ("where is the folder for NHS Number X right now?") — barcode/QR/RFID move audit trail | [service-with-rust](case-folder/case-folder-service-with-rust/spec/index.md) (Loco JSON API) · [front-end-with-svelte](case-folder/case-folder-front-end-with-svelte/spec/index.md) (SvelteKit + SVAR + Lily) |

### Cross-cutting services

Infrastructure services that span the entity trios rather than owning one
entity. They are not matcher-backed and have no front-end of their own.

| Service | Purpose | Spec |
|---|---|---|
| [link-graph-service-with-loco](link/link-graph-service-with-loco/spec/index.md) | **Read-model aggregator** for cross-service entity linking — consumes every entity's event stream (+ the new `linked`/`unlinked` events) and serves the queryable cross-service graph (`neighbors` / `single-view` / freshness). The read side of the hybrid topology in [cross-service-linking.md](agents/share/cross-service-linking.md); each entity service owns its own link **writes** (`entity_links` + events). v1 edges: `same_identity` (person↔worker), `works_at`/`member_of` (person→org), `employed_by` (worker→org), `subject_of` (case→person). | [spec](link/link-graph-service-with-loco/spec/index.md) |

## Shared reference docs

@agents/share/index.md

@agents/share/architecture.md
@agents/share/dataflow.md
@agents/share/match-search-merge.md
@agents/share/match.md
@agents/share/search.md
@agents/share/merge.md
@agents/share/privacy.md
@agents/share/jwt.md
@agents/share/authentication-sessions.md
@agents/share/auditability.md
@agents/share/cross-service-linking.md
@agents/share/bulk-import-export.md
@agents/share/availability.md
@agents/share/observability.md
@agents/share/restful.md
@agents/share/loco.md
@agents/share/rust-loco-stack.md
@agents/share/postgresql.md
@agents/share/locales.md
@agents/share/compliance-for-healthcare.md
@agents/share/compliance-for-technology.md

## Common features

### Data management

- Create, read, update, and delete (CRUD) records
- Soft delete with complete audit trails
- Multiple identifiers per record (type + system + value)
- Identity documents (passport, driver's license, etc., where the entity supports them)
- Multiple contacts per record (phone / email / address)
- Automatic event stream publishing for all CRUD operations

### Matching

- **Probabilistic matching** — weighted fuzzy scoring
- **Deterministic matching** — rule-based with short-circuit (tax-ID, document, GLN, …)
- **Configurable scoring** — thresholds and weights are tunable
- **Components** — string similarity (Jaro-Winkler, Levenshtein, Soundex), date proximity, geo (Haversine), identifier exact-match
- **Score breakdown** — full per-component scores in API responses

### Data quality & validation

- Required-field enforcement
- Date / range validation (no future birth dates, lat/lon bounds, GLN check digit, …)
- Email and phone format checks
- Address validation (requires locality, postal code, or country)
- Document validation (number required, expiry check, issue-before-expiry)
- Phone normalization (E.164-like)
- Address standardization (title-case locality, uppercase region/country, abbreviation expansion)
- Validation integrated into create/update handlers (returns `422`)

## Per-crate docs

Each service crate ships an identical doc set:

- `spec.md` — **single source of truth** (numbered §1–§18, live tasks in §13)
- `index.md` — navigation aid with worked examples
- `README.md` / `CLAUDE.md` — user-facing intro (must stay consistent with the spec); `CLAUDE.md` is loaded by Claude Code at session start
- `AGENTS.md` — directory of the crate's reference docs
- `AGENTS/index.md` — index of `AGENTS/*` files
- `AGENTS/spec-driven-development.md` — SDD discipline (three-part PRs, section mapping, anti-patterns)
- `AGENTS/models.md` — domain model reference
- `AGENTS/matching.md` — per-crate matching tuning
- `AGENTS/restful.md` — REST API surface
- `AGENTS/testing.md` — test layout

Each front-end project ships a thinner doc set:

- `spec.md` — §1–§18 SDD shape (same as service crates)
- `README.md` — user-facing intro (routes, quick start, env vars)
- `CLAUDE.md` — one-line `@AGENTS.md` include
- `AGENTS.md` — agent guide (ground rules: Svelte 5 runes only, SPA mode, drift accepted)
- `CHANGELOG.md` — Keep a Changelog format; v0.1.0 inaugural entry

There is intentionally no `plan.md` and no `tasks.md`: plan content
lives in `spec.md §8–§12`, task content in `spec.md §13`, status /
roadmap in `spec.md §14–§15`, open questions in `spec.md §16`.
