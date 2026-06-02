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
| [Person Service](person-service-rust-crate/) | Person (general) | [spec](person-service-rust-crate/spec.md) | [index](person-service-rust-crate/index.md) |
| [Worker Service](worker-service-rust-crate/) | Worker (workforce / professional) | [spec](worker-service-rust-crate/spec.md) | [index](worker-service-rust-crate/index.md) |
| [Place Service](place-service-rust-crate/) | Place (schema.org/Place) | [spec](place-service-rust-crate/spec.md) | [index](place-service-rust-crate/index.md) |
| [Thing Service](thing-service-rust-crate/) | Thing (schema.org/Thing — generic) | [spec](thing-service-rust-crate/spec.md) | [index](thing-service-rust-crate/index.md) |
| [Event Service](event-service-rust-crate/) | Event (schema.org/Event — time-bounded) | [spec](event-service-rust-crate/spec.md) | [index](event-service-rust-crate/index.md) |

### Matcher crates

The matcher crates are reusable, dependency-light Rust libraries for
pairwise record comparison. They follow their own SDD shape (see each
crate's `spec.md` for its full §1–§25 structure), distinct from the
service-crate shape (§1–§18). Use them standalone, or embed them in
the corresponding service crate's matching layer.

| Crate | Entity | Spec | Index |
|---|---|---|---|
| [Person Matcher](person-matcher-rust-crate/) | Person | [spec](person-matcher-rust-crate/spec.md) | [index](person-matcher-rust-crate/index.md) |
| [Worker Matcher](worker-matcher-rust-crate/) | Worker | [spec](worker-matcher-rust-crate/spec.md) | [index](worker-matcher-rust-crate/index.md) |
| [Place Matcher](place-matcher-rust-crate/) | Place | [spec](place-matcher-rust-crate/spec.md) | [index](place-matcher-rust-crate/index.md) |
| [Thing Matcher](thing-matcher-rust-crate/) | Thing | [spec](thing-matcher-rust-crate/spec.md) | [index](thing-matcher-rust-crate/index.md) |
| [Event Matcher](event-matcher-rust-crate/) | Event | [spec](event-matcher-rust-crate/spec.md) | [index](event-matcher-rust-crate/index.md) |

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
@agents/share/technology.md
@agents/share/stack-for-rust-loco.md
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

There is intentionally no `plan.md` and no `tasks.md`: plan content
lives in `spec.md §8–§12`, task content in `spec.md §13`, status /
roadmap in `spec.md §14–§15`, open questions in `spec.md §16`.
