# Subprojects — the Care Pathway Trio

One entity, three subprojects, one capability: a clinical
care-pathway registry with explainable duplicate matching.

| Subproject | Kind | Responsibility |
|---|---|---|
| [care-pathway-matcher-rust-crate](../care-pathway-matcher-rust-crate/) | Rust library (edition 2024, dependency-light) | Pairwise `CarePathway` comparison: deterministic short-circuits + weighted probabilistic scoring with per-component breakdown. Owns the canonical `CarePathway` domain type. |
| [care-pathway-service-with-loco](../care-pathway-service-with-loco/) | loco.rs 0.16 service (Axum 0.8, SeaORM 1.1, PostgreSQL) | Registry: CRUD with soft delete + `/match` + `/check-duplicates`. Stores the matcher type verbatim as JSONB. |
| [care-pathway-front-end-with-svelte](../care-pathway-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, TS strict, dependency-light — no data grid) | Operator UI: list / create / detail / edit / delete / check-duplicates over the service REST API. |

## Dependency direction

```
front-end  ──HTTP──▶  service  ──Cargo path dep──▶  matcher
```

Strictly downward. The matcher depends on nothing in the workspace
(serde, serde_json, strsim, unicode-normalization, thiserror). The
service's API DTO **is** `care_pathway_matcher::CarePathway` — no
adapter, no second model. The front-end hand-mirrors that shape in
`src/lib/api/types.ts`.

## How to run each

```bash
# Matcher — pure library; tests + demo
cd care-pathway-matcher-rust-crate
cargo test
cargo run                    # demo binary (not SemVer surface)

# Service — needs PostgreSQL; port 5150
cd care-pathway-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/care_pathway_service_development
cargo loco start             # migrations auto-run in development
cargo test --test matching   # DB-free tests

# Front-end — needs the service running
cd care-pathway-front-end-with-svelte
cp .env.example .env         # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                     # http://localhost:5173
pnpm run check               # svelte-check strict (0/0 expected)
```

## Where each subproject's docs live

| Doc | Matcher | Service | Front-end |
|---|---|---|---|
| Living spec | [spec/index.md](../care-pathway-matcher-rust-crate/spec/index.md) (§1–§25) | [spec/index.md](../care-pathway-service-with-loco/spec/index.md) (§1–§18) | [spec/index.md](../care-pathway-front-end-with-svelte/spec/index.md) (§1–§18) |
| Agent guide | [AGENTS.md](../care-pathway-matcher-rust-crate/AGENTS.md) | [AGENTS.md](../care-pathway-service-with-loco/AGENTS.md) | [AGENTS.md](../care-pathway-front-end-with-svelte/AGENTS.md) |
| Detailed guides | [agents/](../care-pathway-matcher-rust-crate/agents/index.md) (5 files) | — (thin; entity spec §13 T-1) | — (thin) |
| User intro | [README.md](../care-pathway-matcher-rust-crate/README.md) | [README.md](../care-pathway-service-with-loco/README.md) | [README.md](../care-pathway-front-end-with-svelte/README.md) |
| Changelog | [CHANGELOG.md](../care-pathway-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG.md](../care-pathway-service-with-loco/CHANGELOG.md) | [CHANGELOG.md](../care-pathway-front-end-with-svelte/CHANGELOG.md) |

The entity-level contract between the three lives in
[`../spec/`](../spec/index.md). Drift between this front-end and its
siblings is accepted (repo decision 2026-06-02) — copy-adapt, don't
factor out.
