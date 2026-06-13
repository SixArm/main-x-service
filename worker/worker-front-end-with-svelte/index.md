# worker-front-end-with-svelte — index

Navigation aid + worked flow examples. The behavioural source of truth
is [`spec.md`](spec/index.md); deep references live in [`AGENTS/`](AGENTS/).

## Top-level documents

| Document | Purpose |
|---|---|
| [spec.md](spec/index.md) | Single source of truth (§1–§18; live work queue in §13) |
| [README.md](README.md) | User-facing intro, routes, env vars |
| [CLAUDE.md](CLAUDE.md) | `@AGENTS.md` re-export for Claude Code session bootstrap |
| [AGENTS.md](AGENTS.md) | Agent guide (ground rules, drift policy, tech-stack rules) |
| [CHANGELOG.md](CHANGELOG.md) | Keep-a-Changelog history |

## AGENTS/ (per-area detail)

| Document | Purpose |
|---|---|
| [AGENTS/index.md](AGENTS/index.md) | This directory's index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline (three-part PRs, front-end specifics) |
| [AGENTS/testing.md](AGENTS/testing.md) | Vitest unit + Playwright smoke |

## Sibling service

- [`../worker-service-rust-crate/`](../worker-service-rust-crate/) — the system of record this UI calls. Its [`spec.md`](../worker-service-rust-crate/spec/index.md) and [`AGENTS/restful.md`](../worker-service-rust-crate/AGENTS/restful.md) are the API contract.

## Route map

The SPA mounts at `/`. All operator workflows live under `/workers`.

| Path | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit feed |
| `/workers` | List + search (SVAR DataGrid; `q` + fuzzy + phonetic toggles) |
| `/workers/new` | Create form with real-time 409 duplicate detection inline |
| `/workers/[id]` | Detail view (identity, identifiers, addresses, telecom) |
| `/workers/[id]/edit` | Edit form |
| `/workers/[id]/audit` | Per-record audit log |
| `/workers/match` | Score-bearing match check against a candidate Worker |
| `/workers/merge` | Two-ID merge preview + confirm |

## Worked flows

### Search → create with duplicate inline

1. Operator hits `/workers`, searches `"Jane Smith"` in the SearchBox.
2. Grid renders SVAR DataGrid with matching rows.
3. Operator clicks **New worker** → `/workers/new`.
4. Form submits to `POST /api/workers`. Service returns 409 with a
   `MatchResult[]` under `error.details` when the matcher flags
   probable duplicates.
5. Inline `MatchResultsList` renders each candidate with name,
   identifier, per-component breakdown, deterministic flag.

### Match-check workflow

1. `/workers/match` posts a partial Worker body to
   `POST /api/workers/match`.
2. Service returns blocked candidates sorted by descending score.
3. The page renders quality + per-component breakdown.

### Merge workflow

1. Operator picks main + duplicate IDs in `/workers/merge`.
2. Page calls `POST /api/workers/merge` with the two IDs and a
   merge reason.
3. On success: detail page redirect to main; duplicate is
   soft-deleted server-side.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:8080` | Worker Service base URL |

## Tech stack reminder

- SvelteKit 2 + Svelte 5 **runes only** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`).
- SPA-only (`+layout.ts` exports `ssr = false; prerender = false;`).
- TypeScript strict + `noUncheckedIndexedAccess`.
- SVAR DataGrid for tabular data; Lily Headless for accessibility primitives; native HTML elsewhere.
- No global stores for HTTP state — construct a `WorkerRepository` per page/component.

See [`AGENTS.md`](AGENTS.md) for the full ground-rules.
