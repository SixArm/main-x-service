# Subprojects — Organization Entity

The organization entity is a **trio** in this directory. Dependency
direction is strictly one-way:

```
organization-front-end-with-svelte
        │  HTTP (raw JSON, snake_case, no envelope)
        ▼
organization-service-with-loco          loco.rs app, PostgreSQL
        │  Cargo path dependency; DTO re-used directly
        ▼
organization-matcher-rust-crate          pure library, no IO
```

## The trio

| Subproject | Kind | Responsibility |
|---|---|---|
| [organization-service-with-loco](../organization-service-with-loco/) | loco.rs 0.16 service | Registry: CRUD (soft delete), name search (`ILIKE`), `/match` + `/check-duplicates`, audit log, in-memory event stream, OpenAPI/Swagger. Stores the DTO verbatim as JSONB |
| [organization-matcher-rust-crate](../organization-matcher-rust-crate/) | Rust library | Canonical pairwise matching: deterministic short-circuits (LEI/DUNS/ISO 6523/GLN/Wikidata/ROR/ISNI/VAT, same-jurisdiction tax ID, `same_as`) + probabilistic components. Owns the `Organization` type the whole entity uses |
| [organization-front-end-with-svelte](../organization-front-end-with-svelte/) | SvelteKit 2 SPA | Operator UI: list / create / detail+delete+check-duplicates / edit. Svelte 5 runes, TS strict, dependency-light (no data grid / design system — accepted drift) |

Key design decision: **the matcher's `Organization` type is the API
DTO and the persisted payload.** There is no service-side model fork
and no adapter (unlike the person entity) — one type, zero mapping
drift. See entity [spec §5](../spec/05-domain-model.md).

## How to run each

```bash
# matcher — library + demo
cd organization-matcher-rust-crate
cargo test && cargo run            # demo binary (not SemVer surface)

# service — needs PostgreSQL; default port 5150
cd organization-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/organization_service_development
cargo loco start                   # migrations auto-run in development
cargo test --test matching         # DB-free tests

# front-end — needs the service running
cd organization-front-end-with-svelte
cp .env.example .env               # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install && pnpm dev           # http://localhost:5173
pnpm run check                     # svelte-check strict (0/0 expected)
```

Smoke test the stack:

```bash
curl -s localhost:5150/api/organizations -H 'content-type: application/json' \
  -d '{"name":"Acme, Inc.","jurisdiction":"US","url":"https://acme.com"}'
curl -s localhost:5150/api/organizations/search?q=acme
```

## Where each subproject's docs live

| | Service | Matcher | Front-end |
|---|---|---|---|
| Spec | [spec/index.md](../organization-service-with-loco/spec/index.md) — §1–§18 in one file | [spec/index.md](../organization-matcher-rust-crate/spec/index.md) — §1–§25 in one file | [spec/index.md](../organization-front-end-with-svelte/spec/index.md) — §1–§18 in one file |
| Agent guide | [AGENTS.md](../organization-service-with-loco/AGENTS.md) | [AGENTS.md](../organization-matcher-rust-crate/AGENTS.md) + [AGENTS/](../organization-matcher-rust-crate/AGENTS/index.md) (4 topic guides) | [AGENTS.md](../organization-front-end-with-svelte/AGENTS.md) |
| Intro / nav | [README](../organization-service-with-loco/README.md) · [index](../organization-service-with-loco/index.md) | [README](../organization-matcher-rust-crate/README.md) · [index](../organization-matcher-rust-crate/index.md) | [README](../organization-front-end-with-svelte/README.md) · [index](../organization-front-end-with-svelte/index.md) |
| History | [CHANGELOG](../organization-service-with-loco/CHANGELOG.md) | [CHANGELOG](../organization-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG](../organization-front-end-with-svelte/CHANGELOG.md) |

The service and front-end docs are **thinner** than the mature
entities' (single-file specs; service lacks an `AGENTS/` set) — gap
tracked in entity [spec §13 T-1](../spec/13-tasks.md). Where crate
docs are thin, ground yourself in source:
[`src/controllers/organizations.rs`](../organization-service-with-loco/src/controllers/organizations.rs)
(routes + handlers),
[`src/models/organizations.rs`](../organization-service-with-loco/src/models/organizations.rs)
(persistence helpers),
[`src/lib/api/`](../organization-front-end-with-svelte/src/lib/api/)
(front-end client + types).
