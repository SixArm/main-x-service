# Main X Index Rust crates

@agents/share/overview.md

## Subprojects

- [Main Person Service Rust crate](main-person-service-rust-crate/)
- [Main Patient Index Rust crate](main-patient-index-rust-crate/)
- [Main Worker Service Rust crate](main-worker-service-rust-crate/)
- [Main Place Service Rust crate](main-place-service-rust-crate/)
- [Main Thing Service Rust crate](main-thing-service-rust-crate/)
- [Main Event Service Rust crate](main-event-service-rust-crate/)

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
@agents/share/web-stack.md
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

### Web UI

Every crate ships a server-rendered UI on top of its REST API. See [agents/share/web-stack.md](agents/share/web-stack.md).

- Loco.rs framework conventions
- Tera templates
- HTMX for server-driven AJAX
- Alpine.js for light client-side state
- Lily Design System (HTML Headless) for accessible component structure; NHS UK theme bundled as `lily.css` provides the visual layer

Run with: `cargo run --bin web` (binds `0.0.0.0:5150`).

## Per-crate docs

Each crate ships its own:

- `AGENTS.md` — directory of the crate's reference docs
- `AGENTS/index.md` — index of `AGENTS/*` files
- `AGENTS/models.md` — domain model reference
- `AGENTS/matching.md` — per-crate matching tuning
- `AGENTS/restful.md` — REST API surface
- `AGENTS/testing.md` — test layout
- `CLAUDE.md` — project overview (loaded by Claude Code at session start)
