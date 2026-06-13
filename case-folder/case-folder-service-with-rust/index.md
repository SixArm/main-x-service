# Documentation index

Case Tracking — Loco edition. JSON-only back-end API for tracking
paper case-note folders across NHS hospital cabinets.

## Start here

- **[README.md](README.md)** — quick start, prerequisites, route
  cheat-sheet, project layout.
- **[spec/](spec/index.md)** — full specification (now a directory of
  topic files): stack, domain, NHS Number rules, route table, JSON
  contract, response shapes, examples, testing, regulatory checklist,
  roadmap, plus the SDD files (requirements / design / tasks).
- **[AGENTS.md](AGENTS.md)** — working agreements for collaborators
  (human and AI): repo orientation, response style, CI gates, how to
  add an endpoint.
- **[CHANGELOG.md](CHANGELOG.md)** — notable changes, Keep a Changelog
  format.

## By role

### "I want to call this API from my front-end"

1. Read [README.md](README.md) §"Routes at a glance" for the route
   table.
2. Read [spec/api-contract.md](spec/api-contract.md) for envelopes,
   error shapes, and soft-fail behaviour.
3. Read [spec/examples.md](spec/examples.md) for `curl` recipes.
4. Wire shapes are defined in
   [`src/responses/mod.rs`](src/responses/mod.rs).

### "I want to extend this API"

1. Read [AGENTS.md](AGENTS.md) "Working rules" before opening a PR.
2. Read [spec/routes.md](spec/routes.md) to find the use case your
   endpoint serves.
3. Follow the recipe in [AGENTS.md](AGENTS.md) §"Add a new endpoint".

### "I want to run this in production"

1. Read [spec/regulatory.md](spec/regulatory.md) ("Regulatory
   considerations" + "Security & privacy gates"). Treat as a pre-merge
   checklist.
2. Read [README.md](README.md) §"Useful subcommands" + §"Quick start"
   for environment variables.
3. Wire the five upstream Main-X-Services or stand up shims for the
   ones not yet deployed.

## Domain quick reference

The tracker proxies **five external services**:

| Service                  | Owns                                                    |
| ------------------------ | ------------------------------------------------------- |
| **Main Patient Service** | Patient records keyed by NHS Number                     |
| **Main Place Service**   | Buildings → rooms → cabinets (parent chain)             |
| **Main Worker Service**  | Workforce — clinicians, nurses, administrators, …       |
| **Main Thing Service**   | Folders (`thing_type = "CaseFile"`)                     |
| **Main Event Service**   | Move audit log (`event_type = "FolderMove"`)            |

The tracker keeps **no local tables**; the only state it writes is
folders to the Thing Service and move events to the Event Service.
Snapshots of patient/cabinet/worker labels are denormalised into both
so the audit trail keeps working when upstreams are down.

## Topical index

- **API contract**: [spec/api-contract.md](spec/api-contract.md), [spec/examples.md](spec/examples.md); [`src/responses/mod.rs`](src/responses/mod.rs)
- **Architecture**: [spec/architecture.md](spec/architecture.md); [`src/app.rs`](src/app.rs)
- **Configuration**: [spec/database.md](spec/database.md); [`config/`](config/)
- **Domain model**: [spec/domain-model.md](spec/domain-model.md)
- **Examples (curl + Rust)**: [spec/examples.md](spec/examples.md)
- **File layout**: [spec/architecture.md](spec/architecture.md); [README.md](README.md) §"Layout"
- **NHS Number rules**: [spec/nhs-number.md](spec/nhs-number.md); [`src/nhs.rs`](src/nhs.rs)
- **Regulatory checklist**: [spec/regulatory.md](spec/regulatory.md)
- **Requirements / design / tasks (SDD)**: [spec/requirements.md](spec/requirements.md), [spec/design.md](spec/design.md), [spec/tasks.md](spec/tasks.md)
- **Roadmap**: [spec/roadmap.md](spec/roadmap.md)
- **Routes**: [spec/routes.md](spec/routes.md); [README.md](README.md) §"Routes at a glance"
- **Testing**: [spec/testing.md](spec/testing.md); [`tests/requests/`](tests/requests/)
- **Upstream services**: [spec/domain-model.md](spec/domain-model.md) ("Client interfaces"); [`src/main_*_service/`](src/)

## Sibling projects

- **[`../case-folder-front-end-with-svelte`](../case-folder-front-end-with-svelte)**
  — same domain, Svelte + TypeScript front-end. A **client of this
  API**: every page round-trips through `/api/*`, and its Playwright
  e2e suite runs against this service in stub mode.
- The five upstream Main-X-Services live under
  `~/git/sixarm/main-x-service/`.
