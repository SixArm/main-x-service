# Subprojects — the place trio

One entity, three subprojects, one directory. Dependency direction is
strictly one-way:

```
place-front-end-with-svelte --HTTP--> place-service-rust-crate --Cargo dep--> place-matcher-rust-crate
```

| Subproject | Kind | Responsibility |
|---|---|---|
| [place-service-rust-crate](../place-service-rust-crate/) | Rust service (loco.rs / Axum, SeaORM, Tantivy) | System of record: CRUD, search, geo-radius, duplicate detection, merge, review queue, validation, privacy, audit, events, REST API |
| [place-matcher-rust-crate](../place-matcher-rust-crate/) | Rust library (pure, no IO, no `unsafe`) | Canonical pairwise matching: deterministic + probabilistic scoring with per-field breakdown; normalisation primitives; usable standalone |
| [place-front-end-with-svelte](../place-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, SVAR DataGrid, Lily Headless) | Operator UI over the service's REST API: list/search, create with 409 surfacing, detail/edit, match, merge, audit |

## How they connect

- The **service embeds the matcher** (`place-matcher` in `Cargo.toml`,
  re-exported as `matcher_lib`) and projects its schema.org-shaped
  `Place` into the matcher's flat shape via
  [`src/matching/adapter.rs`](../place-service-rust-crate/src/matching/adapter.rs).
  Contract: entity [spec §5.3](../spec/05-domain-model.md); pinned by
  [`tests/duplicate_detection.rs`](../place-service-rust-crate/tests/duplicate_detection.rs).
- The **front-end mirrors the service wire format** in
  `src/lib/api/types.ts` (per-project copy; drift between front-ends
  is accepted, repo decision 2026-06-02) and reads the base URL from
  `PUBLIC_API_BASE_URL` (default `http://localhost:8080`).
- The matcher knows nothing about the other two.

## How to run each

```bash
# place-service-rust-crate — REST API on :8080, Swagger at /swagger-ui
cd place-service-rust-crate
cargo run --release          # needs PostgreSQL; see its README/CLAUDE.md
cargo test                   # unit + integration + bridge tests
cargo bench                  # Criterion benchmarks

# place-matcher-rust-crate — pure library
cd place-matcher-rust-crate
cargo test                   # unit + integration + property + doctests
cargo clippy --all-targets -- -D warnings
cargo run --example basic_usage

# place-front-end-with-svelte — dev server on :5173
cd place-front-end-with-svelte
cp .env.example .env && pnpm install
pnpm dev                     # expects a running service at PUBLIC_API_BASE_URL
pnpm test && pnpm test:e2e && pnpm check
```

## Where each subproject's docs live

| Subproject | Spec (SSOT for internals) | Agent guide | Reference docs |
|---|---|---|---|
| place-service | [spec/](../place-service-rust-crate/spec/index.md) (§1–§18) | [AGENTS.md](../place-service-rust-crate/AGENTS.md) | [AGENTS/](../place-service-rust-crate/AGENTS/index.md) |
| place-matcher | [spec/](../place-matcher-rust-crate/spec/index.md) (§1–§13, RFC 2119) | [AGENTS.md](../place-matcher-rust-crate/AGENTS.md) | [AGENTS/](../place-matcher-rust-crate/AGENTS/) |
| place-front-end | [spec/](../place-front-end-with-svelte/spec/index.md) (§1–§18) | [AGENTS.md](../place-front-end-with-svelte/AGENTS.md) | [AGENTS/](../place-front-end-with-svelte/AGENTS/index.md) |

Entity-level contract: [`../spec/index.md`](../spec/index.md). A
reference SQL schema sits at
[`../place-service-schema.sql`](../place-service-schema.sql).
