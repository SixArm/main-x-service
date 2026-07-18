# Subprojects — the Portfolio Trio

One entity, three subprojects, one capability: a work-item registry
across four matchable kinds (Portfolio / Project / Product / Program)
with explainable duplicate matching **and** a project-management tool
with operational sub-resources and derived views.

| Subproject | Kind | Responsibility |
|---|---|---|
| [project-portfolio-management-matcher-rust-crate](../project-portfolio-management-matcher-rust-crate/) | Rust library (edition 2024, dependency-light) | Pairwise `WorkItem` comparison: a hard `kind` gate + deterministic short-circuits + weighted probabilistic scoring with per-component breakdown. Owns the canonical `WorkItem` domain type. |
| [project-portfolio-management-service-with-loco](../project-portfolio-management-service-with-loco/) | loco.rs 0.16 service (Axum 0.8, SeaORM 1.1, PostgreSQL) | Registry + PM tool over **four** matchable collections (`portfolios`, `projects`, `products`, `programs`), each with the identical controller shape: CRUD with soft delete + `/match` + `/check-duplicates` + `/deduplicate` + `/merge`; sub-resource CRUD (goals, tasks, issues); timeline + burndown read views; cross-service links; bulk import/export. Stores the matcher type verbatim as JSONB. |
| [project-portfolio-management-front-end-with-svelte](../project-portfolio-management-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, TS strict) | Operator UI: per-collection list / create / detail / edit / delete / check-duplicates + sub-resource management + timeline / burndown views over the service REST API; a portfolio detail rolls up its child projects / products / programs. |

## Dependency direction

```
front-end  ──HTTP──▶  service  ──Cargo path dep──▶  matcher
```

Strictly downward. The matcher depends on nothing in the workspace
(serde, serde_json, strsim, unicode-normalization, thiserror). The
service's API DTO **is** `project_portfolio_management_matcher::WorkItem` — no adapter, no
second model (mirrors care-pathway). The front-end hand-mirrors that
shape in `src/lib/api/types.ts`.

## The four matchable kinds

`WorkItem` carries a required `kind: WorkItemKind` discriminator —
`Portfolio` (the umbrella container), `Project`, `Product`, `Program`.
Each kind is a **distinct table and a distinct REST collection**; a
Portfolio is the umbrella under which Project / Product / Program records
sit (they carry a `portfolio_ref` to their parent). Matching runs
**within a collection only** — the matcher's R-GATE rule returns `0.0`
for any cross-kind pair, so a project is never matched against a product.
See [models.md](models.md) and [matching.md](matching.md).

## Cross-service integrations

The portfolio service references other Main X Index services by opaque
id; it never embeds their data. Resolution is the consuming front-end's /
link aggregator's job.

| Integration | Direction | Purpose |
|---|---|---|
| [authentication-service](../../authentication/authentication-service-with-loco/) | portfolio ← PASETO | Offline PASETO v4.public verification (`src/auth.rs` via `authentication-verifier`, published Ed25519 key); audit `actor` is an auth user id. See [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md) (RS256/JWKS not used). |
| [person-service](../../person/person-service-with-loco/) | portfolio → person `pid` | `lead_ref`, task / issue `assignee_ref` people references |
| [worker-service](../../worker/worker-service-with-loco/) | portfolio → worker `pid` | `lead_ref`, assignee references on tasks / issues |
| [organization-service](../../organization/organization-service-with-loco/) | portfolio → org `pid` | `owner_org_id` sponsor, `OwnerOrg` match component |
| cross-service links | portfolio ↔ any | `/api/{collection}/{pid}/links` link aggregator — typed inbound / outbound links to other work items and entities |

## How to run each

```bash
# Matcher — pure library; tests + demo
cd project-portfolio-management-matcher-rust-crate
cargo test
cargo run                    # demo binary (not SemVer surface)

# Service — needs PostgreSQL; port 5150
cd project-portfolio-management-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/project_portfolio_management_service_development
cargo loco start             # migrations auto-run in development
cargo test --test matching   # DB-free tests

# Front-end — needs the service running
cd project-portfolio-management-front-end-with-svelte
cp .env.example .env         # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                     # http://localhost:5173
pnpm run check               # svelte-check strict (0/0 expected)
```

## Where each subproject's docs live

| Doc | Matcher | Service | Front-end |
|---|---|---|---|
| Living spec | [spec/index.md](../project-portfolio-management-matcher-rust-crate/spec/index.md) (§1–§25) | [spec/index.md](../project-portfolio-management-service-with-loco/spec/index.md) (§1–§18) | [spec/index.md](../project-portfolio-management-front-end-with-svelte/spec/index.md) (§1–§18) |
| Agent guide | [AGENTS.md](../project-portfolio-management-matcher-rust-crate/AGENTS.md) | [AGENTS.md](../project-portfolio-management-service-with-loco/AGENTS.md) | [AGENTS.md](../project-portfolio-management-front-end-with-svelte/AGENTS.md) |
| Detailed guides | [AGENTS/](../project-portfolio-management-matcher-rust-crate/AGENTS/index.md) (5 files) | — (thin; entity spec §13 T-1) | — (thin) |
| User intro | [README.md](../project-portfolio-management-matcher-rust-crate/README.md) | [README.md](../project-portfolio-management-service-with-loco/README.md) | [README.md](../project-portfolio-management-front-end-with-svelte/README.md) |
| Changelog | [CHANGELOG.md](../project-portfolio-management-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG.md](../project-portfolio-management-service-with-loco/CHANGELOG.md) | [CHANGELOG.md](../project-portfolio-management-front-end-with-svelte/CHANGELOG.md) |

The entity-level contract between the three lives in
[`../spec/`](../spec/index.md). Drift between this front-end and its
siblings is accepted (repo decision 2026-06-02) — copy-adapt, don't
factor out.
