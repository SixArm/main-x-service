# Subprojects — the Plan Trio

One entity, three subprojects, one capability: a plan registry with
explainable duplicate matching **and** a project-management tool with
operational sub-resources and derived views.

| Subproject | Kind | Responsibility |
|---|---|---|
| [plan-matcher-rust-crate](../plan-matcher-rust-crate/) | Rust library (edition 2024, dependency-light) | Pairwise `Plan` comparison: deterministic short-circuits + weighted probabilistic scoring with per-component breakdown. Owns the canonical `Plan` domain type. |
| [plan-service-with-loco](../plan-service-with-loco/) | loco.rs 0.16 service (Axum 0.8, SeaORM 1.1, PostgreSQL) | Registry + PM tool: plan CRUD with soft delete + `/match` + `/check-duplicates` + `/deduplicate` + `/merge`; sub-resource CRUD (goals, tasks, issues, posts, comments, members); timeline + burndown read views; cross-service links; bulk import/export. Stores the matcher type verbatim as JSONB. |
| [plan-front-end-with-svelte](../plan-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, TS strict) | Operator UI: list / create / detail / edit / delete / check-duplicates + sub-resource management + timeline / burndown views over the service REST API. |

## Dependency direction

```
front-end  ──HTTP──▶  service  ──Cargo path dep──▶  matcher
```

Strictly downward. The matcher depends on nothing in the workspace
(serde, serde_json, strsim, unicode-normalization, thiserror). The
service's API DTO **is** `plan_matcher::Plan` — no adapter, no second
model (mirrors care-pathway). The front-end hand-mirrors that shape
in `src/lib/api/types.ts`.

## Cross-service integrations

The plan service references other Main X Index services by opaque id;
it never embeds their data. Resolution is the consuming front-end's /
link aggregator's job.

| Integration | Direction | Purpose |
|---|---|---|
| [authentication-service](../../authentication/authentication-service-with-loco/) | plan ← PASETO | Offline PASETO v4.public verification (`src/auth.rs` via `authentication-verifier`, published Ed25519 key); `Member.user_ref` and audit `actor` are auth user ids. See [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md) (RS256/JWKS not used). |
| [person-service](../../person/person-service-with-loco/) | plan → person `pid` | `Member.person_ref`, `*_ref` people references |
| [worker-service](../../worker/worker-service-with-loco/) | plan → worker `pid` | `Member.worker_ref`, assignee references on tasks / issues |
| [organization-service](../../organization/organization-service-with-loco/) | plan → org `pid` | `owner_org_id` sponsor, `OwnerOrg` match component |
| cross-service links | plan ↔ any | `/api/v1/plans/{pid}/links` link aggregator — typed inbound / outbound links to other plans and entities |

## How to run each

```bash
# Matcher — pure library; tests + demo
cd plan-matcher-rust-crate
cargo test
cargo run                    # demo binary (not SemVer surface)

# Service — needs PostgreSQL; port 5150
cd plan-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/plan_service_development
cargo loco start             # migrations auto-run in development
cargo test --test matching   # DB-free tests

# Front-end — needs the service running
cd plan-front-end-with-svelte
cp .env.example .env         # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                     # http://localhost:5173
pnpm run check               # svelte-check strict (0/0 expected)
```

## Where each subproject's docs live

| Doc | Matcher | Service | Front-end |
|---|---|---|---|
| Living spec | [spec/index.md](../plan-matcher-rust-crate/spec/index.md) (§1–§25) | [spec/index.md](../plan-service-with-loco/spec/index.md) (§1–§18) | [spec/index.md](../plan-front-end-with-svelte/spec/index.md) (§1–§18) |
| Agent guide | [AGENTS.md](../plan-matcher-rust-crate/AGENTS.md) | [AGENTS.md](../plan-service-with-loco/AGENTS.md) | [AGENTS.md](../plan-front-end-with-svelte/AGENTS.md) |
| Detailed guides | [AGENTS/](../plan-matcher-rust-crate/AGENTS/index.md) (5 files) | — (thin; entity spec §13 T-1) | — (thin) |
| User intro | [README.md](../plan-matcher-rust-crate/README.md) | [README.md](../plan-service-with-loco/README.md) | [README.md](../plan-front-end-with-svelte/README.md) |
| Changelog | [CHANGELOG.md](../plan-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG.md](../plan-service-with-loco/CHANGELOG.md) | [CHANGELOG.md](../plan-front-end-with-svelte/CHANGELOG.md) |

The entity-level contract between the three lives in
[`../spec/`](../spec/index.md). Drift between this front-end and its
siblings is accepted (repo decision 2026-06-02) — copy-adapt, don't
factor out.
