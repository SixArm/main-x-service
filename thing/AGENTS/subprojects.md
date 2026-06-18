# Subprojects — Thing Entity

The thing entity is a trio. Dependency direction is strictly one-way:

```
thing-front-end-with-svelte ──REST──> thing-service-with-loco ──Cargo dep──> thing-matcher-rust-crate
```

## The trio

| Subproject | Role | Stack | Spec | Docs |
|---|---|---|---|---|
| [thing-service-with-loco](../thing-service-with-loco/) | System of record: REST CRUD, search, dedup, merge, audit, privacy, events | loco.rs 0.16 / Axum 0.8, SeaORM + PostgreSQL, Tantivy 0.22, Tonic stub, utoipa | [spec/](../thing-service-with-loco/spec/index.md) (§1–§18) | [AGENTS/](../thing-service-with-loco/AGENTS/index.md), [README](../thing-service-with-loco/README.md) |
| [thing-matcher-rust-crate](../thing-matcher-rust-crate/) | Canonical pairwise matcher: deterministic + probabilistic scoring with per-field breakdown | Pure Rust library — no IO, no async runtime, no `unsafe` | [spec/](../thing-matcher-rust-crate/spec/index.md) (§1–§13) | [AGENTS.md](../thing-matcher-rust-crate/AGENTS.md), [AGENTS/](../thing-matcher-rust-crate/AGENTS/), [README](../thing-matcher-rust-crate/README.md) |
| [thing-front-end-with-svelte](../thing-front-end-with-svelte/) | Operator UI over the service REST API | SvelteKit 2, Svelte 5 runes, SVAR DataGrid, Lily Headless, TS strict | [spec/](../thing-front-end-with-svelte/spec/index.md) (§1–§18) | [AGENTS.md](../thing-front-end-with-svelte/AGENTS.md), [README](../thing-front-end-with-svelte/README.md) |

## Responsibilities at a glance

- **Service** owns persistence, the REST contract, validation,
  privacy, audit, search, duplicate workflow. It embeds the matcher
  through [`src/matching/adapter.rs`](../thing-service-with-loco/src/matching/adapter.rs)
  (`thing-matcher = "0.6.1"` in Cargo.toml).
- **Matcher** owns the comparison algorithm and its normalisation; it
  is also usable standalone by any Rust consumer.
- **Front-end** owns presentation only; it keeps its own copy of API
  types / client / form primitives (drift between front-ends is
  accepted, decision 2026-06-02).

## How to run each

```bash
# Service — REST API on :8080 (PostgreSQL optional for in-memory paths)
cd thing-service-with-loco
cargo run --release
cargo test --lib            # unit tests
cargo test --tests          # integration + bridge tests
cargo bench                 # Criterion benchmarks

# Matcher — library; demo + examples
cd thing-matcher-rust-crate
cargo test                  # unit + integration + property + doctests
cargo run --example basic_usage
cargo clippy --all-targets -- -D warnings   # required gate

# Front-end — dev server on :5173, expects service on :8080
cd thing-front-end-with-svelte
cp .env.example .env
pnpm install
pnpm dev
pnpm test                   # vitest unit + playwright e2e
```

## Where the contract lives

- Entity spec [`../spec/05-domain-model.md`](../spec/05-domain-model.md)
  §5.3 — DTO contract (service → matcher projection).
- Entity spec [`../spec/09-api-surface.md`](../spec/09-api-surface.md)
  — REST + route summary (front-end → service).
- [`tests/duplicate_detection.rs`](../thing-service-with-loco/tests/duplicate_detection.rs)
  — 15 bridge tests enforcing the DTO contract.
