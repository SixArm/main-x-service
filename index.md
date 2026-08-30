# Main X Service

@agents/share/overview.md

## Subprojects

Subprojects are grouped one directory per entity. Each entity
directory holds a front-end web app, a matcher (or verifier) library
crate, a service API crate, and entity-level `spec/` + `agents/`
umbrella docs. The full, current entity / matcher / library / front-end
/ cross-cutting-service / consumer-app tables — including the **honest
per-crate capability matrix** (which crate has Tantivy, FHIR, gRPC,
bulk import/export, …) — live in the `overview.md` this file `@`-includes
above; they are **not repeated here**, because a second hand-maintained
copy is exactly how two docs stop agreeing (this file's table drifted
badly enough by 2026-08-04 that it was still missing `case` and
`project-portfolio-management` entirely, months after both shipped —
fixed by removing the duplicate rather than re-syncing it once more).

Each crate is self-contained: it owns its REST API, its persistence
schema, and its matching algorithm. They share an architecture and a
documentation layout, not code (per-project drift is accepted — see
`overview.md`'s front-end section).

## Examples and tutorials

- [`tutorials/`](tutorials/) — six numbered walkthroughs, each
  live-verified end to end: [01 getting started](tutorials/01-getting-started.md),
  [02 identity lifecycle](tutorials/02-identity-lifecycle.md) (create →
  duplicate → match → merge → audit), [03 authentication & ABAC](tutorials/03-authentication-abac.md),
  [04 cross-service linking](tutorials/04-cross-service-linking.md),
  [05 bulk import/export](tutorials/05-bulk-import-export.md),
  [06 the event bus](tutorials/06-event-bus.md).
- [`examples/`](examples/) — runnable reference material the tutorials
  build on: [`compose/`](examples/compose/) (single-service, full-family,
  and ABAC-enforced Podman Compose stacks), [`data/`](examples/data/)
  (synthetic JSONL fixtures with deliberate duplicate pairs, no real
  PII), [`policies/`](examples/policies/) (an ABAC policy cookbook), and
  [`api/`](examples/api/) (per-service `.http` request collections,
  including the full auth handshake).

## Running

From any subproject root:

```bash
# REST + gRPC API
cargo run --release

# Tests
cargo test --lib

# Benchmarks (where available)
cargo bench
```

Or bring up a whole stack via Podman Compose — see
[`examples/compose/README.md`](examples/compose/README.md) and
[tutorial 01](tutorials/01-getting-started.md).

## Documentation

The full, current index of shared reference docs — architecture, auth
(sessions + ABAC), privacy, compliance, the event bus, cross-service
linking, bulk import/export, the complete environment-variable
reference, operational runbooks, and more — is
[`agents/share/index.md`](agents/share/index.md). It is **not**
duplicated here (same reasoning as the Subprojects section above): that
file's own table is the thing that stays current, and every doc it
links to is `@`-included by the crate that needs it, not copy-pasted
per crate.

Some subprojects also carry a `<crate>/agents/` directory of
crate-local reference docs (`index.md`, `models.md`, `matching.md`,
`restful.md`, `testing.md`, …) — the six original entity crates
(person, worker, place, thing, event, course) and the matcher crates,
specifically; newer subprojects keep the equivalent material in their
own `spec/` instead (see root [`AGENTS.md`](AGENTS.md)'s "The `agents/`
directory: older subprojects only" for why that split is deliberate,
not a gap).

## AI agent guidance

- [`llms.txt`](llms.txt) / [`llms.json`](llms.json) — a curated,
  size-bounded map of this repo's most important content (every
  entity registry, matcher, library, front-end, consumer app, and
  cross-cutting service, plus the shared reference docs and CI entry
  points), for an AI tool that wants a starting point rather than a
  full-tree crawl. See
  [`spec/llms-json-and-llms-txt/index.md`](spec/llms-json-and-llms-txt/index.md).
- [`sixarm-services-skill/`](sixarm-services-skill/) — a Claude Code
  skill explaining the system's concepts, terminology, and worked
  examples for someone who wants to understand what it does.
- [`sixarm-services-maintainer-skill/`](sixarm-services-maintainer-skill/) —
  a Claude Code skill for someone about to change code, specs, or
  infrastructure here: the SDD discipline, the crate layout, and the
  local CI-check commands.
- Both skills are defined by their own `spec/agent-skills/index.md`.

## Architecture snapshot

```
┌─────────────────────────────────────────────────────────────┐
│ Client (curl / SDK / gRPC client)                           │
└────────────────────────────┬────────────────────────────────┘
                             │
            ┌────────────────▼────────────────┐
            │ REST API (Axum) + gRPC (Tonic)  │
            │ + OpenAPI/Swagger UI            │
            │ /api/<plural>/…                 │
            └────────────────┬────────────────┘
                             │
            ┌────────────────▼────────────────┐
            │ Application logic               │
            │  • Validation & normalization   │
            │  • Matching (probabilistic +    │
            │     deterministic)              │
            │  • Privacy (masking, GDPR)      │
            │  • Audit log emission           │
            └────────────────┬────────────────┘
                             │
        ┌────────────────────┼─────────────────────┐
        │                    │                     │
┌───────▼──────┐    ┌────────▼────────┐   ┌────────▼────────┐
│ PostgreSQL   │    │ Tantivy index   │   │ Event stream    │
│ (SeaORM)     │    │ (full-text +    │   │ (Fluvio /       │
│              │    │  fuzzy/phonetic)│   │  in-memory)     │
└──────────────┘    └─────────────────┘   └─────────────────┘
```

## Status

Rust backend services (loco.rs) plus SvelteKit operator front-ends —
this repo is no longer backend-only, and has not been since the
front-end projects landed. See `overview.md`'s capability matrix for
what's actually implemented per crate today, and
[`tasks.md`](tasks.md) for the live work queue.

## Publishing

Published crates (currently `authentication-verifier` and the
matcher crates — see `AGENTS.md`'s Library/Matcher crate tables for
which ones) are released to crates.io with `cargo publish`, run
locally by the maintainer against a stored API token; no publish step
runs in CI today. We intend to move to **Trusted Publishing** (OIDC,
no long-lived tokens stored anywhere) once it is production-ready
across every code forge we use (GitHub.com, Codeberg.org) and every
target registry (crates.io, npm) — see
[`spec/trusted-publishing/index.md`](spec/trusted-publishing/index.md).

## License

See [`LICENSE.md`](LICENSE.md). Each crate declares its own SPDX
license expression in its manifest — `LICENSE.md` documents the two
expressions in use (Rust crates, front-end packages) and how they
relate.
