# Subprojects — the Case Trio

One entity, three subprojects, one capability: a governmental
case-management registry with explainable duplicate matching.

| Subproject | Kind | Responsibility |
|---|---|---|
| [case-matcher-rust-crate](../case-matcher-rust-crate/) | Rust library (edition 2024, dependency-light) | Pairwise `Case` comparison: deterministic short-circuits + weighted probabilistic scoring with per-component breakdown. Owns the canonical `Case` domain type. |
| [case-service-rust-crate](../case-service-rust-crate/) | loco.rs 0.16 service (Axum 0.8, SeaORM 1.1, PostgreSQL) | Registry: CRUD with soft delete + `/search` + `/match` + `/check-duplicates` + `/merge`, audit log, event stream, RS256 JWT verification, OpenAPI. Stores the matcher type verbatim as JSONB. |
| [case-front-end-with-svelte](../case-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, TS strict, dependency-light — no data grid) | Operator UI: list / create / detail / edit / delete / check-duplicates over the service REST API. |

## Dependency direction

```
front-end  ──HTTP──▶  service  ──Cargo path dep──▶  matcher
```

Strictly downward. The matcher depends on nothing in the workspace
(serde, serde_json, strsim, unicode-normalization, thiserror). The
service's API DTO **is** `case_matcher::Case` — no adapter, no second
model. The front-end hand-mirrors that shape in `src/lib/api/types.ts`.

## How to run each

```bash
# Matcher — pure library; tests + demo
cd case-matcher-rust-crate
cargo test
cargo run                    # demo binary (not SemVer surface)

# Service — needs PostgreSQL; port 5150
cd case-service-rust-crate
export DATABASE_URL=postgres://loco:loco@localhost:5432/case_service_development
# optional SSO env (verifier is env-injected):
#   CASE_JWKS=… CASE_JWT_ISSUER=… CASE_JWT_AUDIENCE=…
cargo loco start             # migrations auto-run in development
cargo test --test matching   # DB-free tests
cargo test -- --ignored      # request-level tests (needs DATABASE_URL)

# Front-end — needs the service running
cd case-front-end-with-svelte
cp .env.example .env         # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                     # http://localhost:5173
pnpm run check               # svelte-check strict (0/0 expected)
pnpm test                    # vitest
pnpm test:e2e                # Playwright smoke
```

## Where each subproject's docs live

| Doc | Matcher | Service | Front-end |
|---|---|---|---|
| Living spec | [spec/index.md](../case-matcher-rust-crate/spec/index.md) (§1–§25) | [spec/index.md](../case-service-rust-crate/spec/index.md) (§1–§18) | [spec/index.md](../case-front-end-with-svelte/spec/index.md) (§1–§18) |
| Agent guide | [AGENTS.md](../case-matcher-rust-crate/AGENTS.md) | [AGENTS.md](../case-service-rust-crate/AGENTS.md) | [AGENTS.md](../case-front-end-with-svelte/AGENTS.md) |
| Detailed guides | [AGENTS/](../case-matcher-rust-crate/AGENTS/index.md) | — (thin; entity spec §13 T-13) | — (thin) |
| User intro | [README.md](../case-matcher-rust-crate/README.md) | [README.md](../case-service-rust-crate/README.md) | [README.md](../case-front-end-with-svelte/README.md) |
| Changelog | [CHANGELOG.md](../case-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG.md](../case-service-rust-crate/CHANGELOG.md) | [CHANGELOG.md](../case-front-end-with-svelte/CHANGELOG.md) |

The entity-level contract between the three lives in
[`../spec/`](../spec/index.md). Drift between this front-end and its
siblings is accepted (repo decision 2026-06-02) — copy-adapt, don't
factor out.
