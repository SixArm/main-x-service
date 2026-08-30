---
name: sixarm-services-maintainer-skill
description: Technical reference for maintainers making a code, spec, or infrastructure change in the SixArm main-x-service repo — the crate layout, the spec-driven-development workflow (spec + code + test in one change), how to run CI checks locally, the two service shapes, and the recipe for adding a new entity or rolling a family-wide change across crates. Use before or while editing code, specs, or tests in this repository, or when asked how to add an entity, run a check, or find where a behaviour is defined.
---

# SixArm Services — maintainer reference

This skill is the *how to work in this repo* layer. For a plain-language
explanation of what the system does, use `sixarm-services-skill`
instead. This skill assumes you are about to read, write, or review
code, a spec, or a test here.

## The one rule that overrides everything else

**Each subproject's `spec/` directory is the single source of truth.**
Code conforms to the spec, not the reverse. A behavioural change is
**one change carrying three parts**: a spec edit, a code edit, and a
test edit — never code without the other two. See
[`agents/share/index.md`](../agents/share/index.md) and each crate's
own `agents/spec-driven-development.md` (older crates) for the full
discipline and its anti-patterns.

Two spec shapes exist on purpose and are not being unified:

| Shape | Used by | Layout | Live task queue |
|---|---|---|---|
| Numbered | entity registries, matcher crates, front-ends | `spec/01-*.md` … (§1–§18, or §1–§25 for matchers) | §13 |
| Topic-named | the five consumer apps | `spec/<topic>.md` + `requirements.md`/`design.md`/`tasks.md` | `spec/tasks.md` |

There is no per-subproject `plan.md`: planning content lives in the
spec itself (§8–§12 numbered, `design.md` topic-named). The repo root
additionally keeps cross-cutting [`plan.md`](../plan.md) and
[`tasks.md`](../tasks.md) for work spanning multiple subprojects.

## Repo shape

No root `Cargo.toml` — **each crate is its own workspace**. This is
deliberate (see `agents/share/architecture.md`), so nothing can build
or check `--workspace`; every script iterates the ~55 crates
individually (`scripts/ci-crates.sh`).

Four crate families, each entity nested under its own top-level
directory (`person/`, `place/`, `organization/`, …):

- **Entity registry services** — CRUD + matching for one entity, e.g.
  `person/person-service-with-loco/`. Two internal layouts exist and
  are **not** being converted into each other: the older
  **person-style** shape (`src/api/rest/`, `src/db/`, a separate domain
  model — person, worker, place, thing, event, course) and the newer
  **loco-idiomatic** shape (`src/controllers/`, `src/models/` where the
  API DTO *is* the matcher crate's type stored as JSONB — organization,
  care-pathway, case, portfolio). Check which shape a crate uses before
  assuming where a router, a middleware layer, or a model lives — the
  two differ in surface count (one router-construction path vs. a
  second hand-rolled one for tests) and in whether there's a separate
  domain model to keep in sync with the matcher.
- **Matcher crates** — dependency-light comparison libraries, one per
  entity, embedded directly by the matching sibling service, e.g.
  `person/person-matcher-rust-crate/`. Own `spec/01-*.md` … `25-*.md`
  shape.
- **Front-ends** — one independent SvelteKit SPA per entity, e.g.
  `person/person-front-end-with-svelte/`. Deliberate drift between
  them (no shared package) — copy-adapt a sibling rather than factor
  one out.
- **Cross-cutting services** — `authentication/authentication-service-with-loco/`
  (the SSO provider) and `link/link-graph-service-with-loco/` (the
  read-model aggregator); neither is an entity registry.

Consumer apps (`case-folder/`, `patient-flow/`,
`workforce-planning-management/`, `contact-relationship-management/`,
`content-management-system/`) *consume* the registries rather than
being one; they use the topic-named spec shape and their own
service+front-end pair.

Full map: [`agents/share/overview.md`](../agents/share/overview.md).
Layered request flow and the two internal shapes in detail:
[`agents/share/architecture.md`](../agents/share/architecture.md).

## Adding a new entity

Matcher first, then service (API DTO = the matcher's type, stored as
JSONB — no separate model to drift), then front-end (copy-adapt a
sibling). Do not invent a fifth internal shape; pick loco-idiomatic
unless there's a specific reason to match the older layout. See a
recent trio (organization, care-pathway, case, portfolio) as the
reference recipe.

## Rolling a change across the family

Several changes exist as a **task tracked in the root `tasks.md`** that
lands **one crate ("slice") at a time** rather than in one PR (e.g.
OpenTelemetry OTLP rollout, `PRO-H12`; header-based API versioning,
`api-versioning.md`). Before starting a slice:

1. Read the task's own entry in `tasks.md` for what's landed and what
   assumption the next slice should **not** carry over unverified (a
   prior slice's shape is one data point, not a guarantee the next
   crate matches it — confirm router-surface count, declared
   dependencies, etc. against that crate's own `Cargo.toml`/`src/`
   rather than assuming).
2. Copy the most recently landed, most-similar-shaped crate's port,
   not the original reference, so adaptations accumulate rather than
   getting re-derived each time.
3. Update: the crate's own `spec/` (§13 task + relevant behaviour
   section), its `AGENTS.md`, its `CHANGELOG.md`, the root `tasks.md`
   entry, and any `agents/share/*.md` doc whose "current state" claim
   the slice changed. A doc that says "N of M crates carry X" is wrong
   the moment M+1 lands and nobody edits it.

## Verifying a change (do this before calling it done)

```sh
scripts/ci-check.sh fmt      <crate>     # cargo fmt --check
scripts/ci-check.sh clippy   <crate>     # cargo clippy --all-targets -- -D warnings
scripts/ci-check.sh deny     <crate>     # cargo deny check (advisories + licenses)
scripts/ci-check.sh msrv     <crate>     # cargo +<msrv> check --all-targets
scripts/ci-check.sh test     <crate>     # cargo test (DB-free suites)
scripts/ci-check.sh test-db  <crate>     # cargo test -- --ignored, against Postgres
scripts/ci-check.sh bench    <crate>     # cargo bench --no-run (benches compile+link)
```

Every stage shells out through `scripts/ci-crates.sh` so Codeberg
(`.woodpecker.yml`) and GitHub (`.github/workflows/ci.yml`) run
byte-identical commands. Verify independently rather than trusting a
port compiled the first time — a clean `cargo build` does not imply
clippy-pedantic-clean, MSRV-clean, or that the new tests actually run
(see `feedback_verify_dont_infer` for why this matters here
specifically).

**MSRV** is current stable minus two, in [`ci/msrv.txt`](../ci/msrv.txt)
(today 1.96) — declared per crate (`rust-version` in every
`[package]`; there's no root manifest to inherit from) and checked
against that file, not hand-derived per crate.

**`test-db` is opt-in per crate**: a crate's `--ignored` suite only
runs in CI once it's listed in [`ci/db-suites.txt`](../ci/db-suites.txt)
— an allowlist, not a denylist, so a newly-added suite must be
*observed* green locally first (`DB_SUITES_FORCE=1` forces it to run
before being added).

### Local Postgres for `test-db`

Every service crate carries its own `compose.test.yaml` (Podman, not
Docker — `postgres:18-alpine`, superuser `loco`/`loco`, PGDATA on
tmpfs so every start is a clean `initdb`):

```sh
scripts/test-db.sh up   <crate>
scripts/ci-check.sh test-db <crate>
scripts/test-db.sh down <crate>
TEST_DB_PORT=5433 scripts/test-db.sh up <crate>   # a second one alongside
```

## Family-wide conventions that are easy to violate by accident

- **PostgreSQL, not SQLite.** Loco's `queue.kind` for background jobs
  is `Postgres`, never `SQLite`, even though loco itself supports both.
- **Podman, not Docker.** Every `compose*.yaml` and container doc
  assumes Podman.
- **PASETO v4.public, never JWT, for anything session- or
  auth-shaped.** See [`agents/share/jwt.md`](../agents/share/jwt.md) —
  this is a hard rule with a stated rationale, not a style preference;
  a past incident (RUSTSEC-2023-0071) is why loco's own `auth`
  Cargo feature is dropped family-wide rather than merely unused.
- **`<ENTITY>_REQUIRE_AUTH` defaults off.** The blanket auth+ABAC guard
  exists in every registry but ships open; activating it is a tracked
  release gate, not implied by the code existing. See
  [`agents/share/security.md`](../agents/share/security.md) §4.
- **URLs carry no API version.** Version negotiation is the
  `Accepts-version` header, never a `/api/v1/` path segment. See
  [`agents/share/api-versioning.md`](../agents/share/api-versioning.md).
- **Cross-service links are never a matching signal.** `entity_links` /
  the link-graph aggregator answer "how are these related"; the
  matcher crates answer "are these the same" — keeping them separate
  is a named rule (`cross-service-linking.md` §7), not an oversight to
  "fix" by wiring one into the other.
- **`#![forbid(unsafe_code)]` on every crate root**, and matcher/
  validator/codec code over untrusted input must never panic (return
  `Result`) — both are pinned, not aspirational.

## Where behaviour actually lives (don't guess, read)

| Question | Look here first |
|---|---|
| What should this endpoint do? | The crate's own `spec/` (never `AGENTS.md`, `README.md`, or this skill) |
| What does the whole family guarantee about X? | `agents/share/<topic>.md` (auth, privacy, observability, bulk import/export, …) |
| What's still open / in progress? | `spec/13-*.md` or `spec/tasks.md` (per crate) and the root [`tasks.md`](../tasks.md) |
| Every `<ENTITY>_*` env var, its default, and what governs it | [`agents/share/configuration.md`](../agents/share/configuration.md) |
| Does crate X actually carry capability Y today? | `agents/share/overview.md`'s capability matrix — grounded by grepping the tree, corrected in place when a claim goes stale, not left to rot |

If a shared doc and a crate's own code disagree, that's a finding to
fix (update the doc or file a task), not a signal to trust the doc.
