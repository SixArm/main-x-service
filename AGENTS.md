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
| [Person Service](person/person-service-rust-crate/) | Person (general) | [spec](person/person-service-rust-crate/spec/index.md) | [index](person/person-service-rust-crate/index.md) |
| [Worker Service](worker/worker-service-rust-crate/) | Worker (workforce / professional) | [spec](worker/worker-service-rust-crate/spec/index.md) | [index](worker/worker-service-rust-crate/index.md) |
| [Place Service](place/place-service-rust-crate/) | Place (schema.org/Place) | [spec](place/place-service-rust-crate/spec/index.md) | [index](place/place-service-rust-crate/index.md) |
| [Thing Service](thing/thing-service-rust-crate/) | Thing (schema.org/Thing — generic) | [spec](thing/thing-service-rust-crate/spec/index.md) | [index](thing/thing-service-rust-crate/index.md) |
| [Event Service](event/event-service-rust-crate/) | Event (schema.org/Event — time-bounded) | [spec](event/event-service-rust-crate/spec/index.md) | [index](event/event-service-rust-crate/index.md) |
| [Course Service](course/course-service-rust-crate/) | Course (schema.org/Course) — template + `CourseInstance` sub-resource | [spec](course/course-service-rust-crate/spec/index.md) | [index](course/course-service-rust-crate/index.md) |
| [Authentication Service](authentication/authentication-service-rust-crate/) | User (central single sign-on provider) — passwordless magic-link, RS256 JWT + JWKS. **First real loco.rs crate**; reference for converting the others. | [spec](authentication/authentication-service-rust-crate/spec/index.md) | [index](authentication/authentication-service-rust-crate/index.md) |
| [Organization Service](organization/organization-service-rust-crate/) | Organization (schema.org/Organization) — loco.rs CRUD + matching (embeds organization-matcher) + name search + audit log + event streaming + OpenAPI/Swagger + record merge | [spec](organization/organization-service-rust-crate/spec/index.md) | [index](organization/organization-service-rust-crate/index.md) |
| [Care Pathway Service](care-pathway/care-pathway-service-rust-crate/) | Care pathway (clinical pathway) — loco.rs CRUD + name search + matching (embeds care-pathway-matcher) + condition-code validation + OpenAPI/Swagger + audit log + event streaming + JWT verification + record merge | [spec](care-pathway/care-pathway-service-rust-crate/spec/index.md) | [index](care-pathway/care-pathway-service-rust-crate/index.md) |

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

### Front-end projects

SvelteKit front-ends sit alongside their service crates. Each is an
independent SPA built on SvelteKit 2 + Svelte 5 runes + SVAR Svelte
DataGrid + Lily Design System Svelte Headless, calling the sibling
service's REST API. Their per-project `spec.md` follows the same
§1–§18 SDD shape as the service crates. Drift between front-ends is
accepted (see `feedback_front_end_drift` memory) — no shared package.

| Project | Consumes | Spec | Changelog |
|---|---|---|---|
| [person-front-end-with-svelte](person/person-front-end-with-svelte/) | [person-service](person/person-service-rust-crate/) | [spec](person/person-front-end-with-svelte/spec/index.md) | [CHANGELOG](person/person-front-end-with-svelte/CHANGELOG.md) |
| [worker-front-end-with-svelte](worker/worker-front-end-with-svelte/) | [worker-service](worker/worker-service-rust-crate/) | [spec](worker/worker-front-end-with-svelte/spec/index.md) | [CHANGELOG](worker/worker-front-end-with-svelte/CHANGELOG.md) |
| [place-front-end-with-svelte](place/place-front-end-with-svelte/) | [place-service](place/place-service-rust-crate/) | [spec](place/place-front-end-with-svelte/spec/index.md) | [CHANGELOG](place/place-front-end-with-svelte/CHANGELOG.md) |
| [thing-front-end-with-svelte](thing/thing-front-end-with-svelte/) | [thing-service](thing/thing-service-rust-crate/) | [spec](thing/thing-front-end-with-svelte/spec/index.md) | [CHANGELOG](thing/thing-front-end-with-svelte/CHANGELOG.md) |
| [event-front-end-with-svelte](event/event-front-end-with-svelte/) | [event-service](event/event-service-rust-crate/) | [spec](event/event-front-end-with-svelte/spec/index.md) | [CHANGELOG](event/event-front-end-with-svelte/CHANGELOG.md) |
| [course-front-end-with-svelte](course/course-front-end-with-svelte/) | [course-service](course/course-service-rust-crate/) | [spec](course/course-front-end-with-svelte/spec/index.md) | [CHANGELOG](course/course-front-end-with-svelte/CHANGELOG.md) |
| [authentication-front-end-with-svelte](authentication/authentication-front-end-with-svelte/) | [authentication-service](authentication/authentication-service-rust-crate/) | [spec](authentication/authentication-front-end-with-svelte/spec/index.md) | [CHANGELOG](authentication/authentication-front-end-with-svelte/CHANGELOG.md) |
| [organization-front-end-with-svelte](organization/organization-front-end-with-svelte/) | [organization-service](organization/organization-service-rust-crate/) | [spec](organization/organization-front-end-with-svelte/spec/index.md) | [CHANGELOG](organization/organization-front-end-with-svelte/CHANGELOG.md) |
| [care-pathway-front-end-with-svelte](care-pathway/care-pathway-front-end-with-svelte/) | [care-pathway-service](care-pathway/care-pathway-service-rust-crate/) | [spec](care-pathway/care-pathway-front-end-with-svelte/spec/index.md) | [CHANGELOG](care-pathway/care-pathway-front-end-with-svelte/CHANGELOG.md) |

## Shared reference docs

@agents/share/index.md

@agents/share/architecture.md
@agents/share/dataflow.md
@agents/share/match-search-merge.md
@agents/share/match.md
@agents/share/search.md
@agents/share/merge.md
@agents/share/privacy.md
@agents/share/auditability.md
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
