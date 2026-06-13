# Documentation index

Case Tracking — Svelte edition. A SvelteKit browser client for
the [Loco JSON API sibling](../case-folder-service-with-rust) that
tracks paper case-note folders across NHS hospital cabinets.

## Start here

- **[README.md](README.md)** — quick start, prerequisites, route
  cheat-sheet, project layout, what was removed.
- **[spec/](spec/index.md)** — full specification (now a directory of
  topic files): stack, domain, NHS Number rules, wiring pattern, route +
  API mapping, cache API, examples, testing, accessibility, roadmap,
  plus the SDD files (requirements / design / tasks).
- **[AGENTS.md](AGENTS.md)** — working agreements for collaborators
  (human and AI): repo orientation, working rules, how-to-add-an-endpoint
  recipe, CI gate.
- **[CHANGELOG.md](CHANGELOG.md)** — notable changes, Keep a Changelog
  format.

## By role

### "I want to run this locally"

1. Boot the API: from `../case-folder-service-with-rust`,
   `cargo run -- task seed && cargo run -- start`.
2. From this project: `npm install && npm run dev` — opens at
   <http://localhost:5173>.
3. Override the API URL with `VITE_API_BASE_URL` if needed.

### "I want to extend the UI"

1. Read [AGENTS.md](AGENTS.md) "Working rules" before opening a PR.
2. Read [spec/architecture.md](spec/architecture.md) for the load +
   cache pattern.
3. Follow the recipe in [AGENTS.md](AGENTS.md) §"Add a new page" or
   §"Add a new endpoint to the API client".

### "I want to understand the wire contract"

1. Read [`../case-folder-service-with-rust/spec/api-contract.md`](../case-folder-service-with-rust/spec/api-contract.md)
   ("JSON contract") for envelopes + error shapes.
2. Look at [`src/lib/api/client.ts`](src/lib/api/client.ts) for the
   TypeScript view of the same.
3. Look at [`src/lib/store/types.ts`](src/lib/store/types.ts) for the
   camelCase domain types.

### "I want to deploy this"

1. Read [spec/regulatory.md](spec/regulatory.md) (regulatory + security
   & privacy). Treat as a pre-merge checklist.
2. Plan same-origin deployment for the API + front-end (so SSR can be
   re-enabled and CORS isn't a vector).
3. Pick a SvelteKit adapter that matches your hosting target — see
   [spec/testing.md](spec/testing.md) ("Build target").

## Topical index

- **Accessibility**: [spec/accessibility.md](spec/accessibility.md)
- **API client**: [`src/lib/api/client.ts`](src/lib/api/client.ts);
  [spec/architecture.md](spec/architecture.md)
- **Architecture**: [spec/architecture.md](spec/architecture.md);
  [`src/routes/+layout.ts`](src/routes/+layout.ts) (CSR-only)
- **Cache API**: [spec/cache-api.md](spec/cache-api.md);
  [`src/lib/store/cache.svelte.ts`](src/lib/store/cache.svelte.ts)
- **Domain model / types**: [spec/domain-model.md](spec/domain-model.md);
  [`src/lib/store/types.ts`](src/lib/store/types.ts)
- **Error handling**: [spec/architecture.md](spec/architecture.md) ("Error policy");
  [`src/routes/+error.svelte`](src/routes/+error.svelte)
- **Examples**: [spec/examples.md](spec/examples.md)
- **File layout**: [spec/architecture.md](spec/architecture.md)
- **NHS Number**: [spec/nhs-number.md](spec/nhs-number.md);
  [`src/lib/store/nhs.ts`](src/lib/store/nhs.ts)
- **Regulatory**: [spec/regulatory.md](spec/regulatory.md)
- **Requirements / design / tasks (SDD)**: [spec/requirements.md](spec/requirements.md),
  [spec/design.md](spec/design.md), [spec/tasks.md](spec/tasks.md)
- **Routes & API mapping**: [spec/routes.md](spec/routes.md)
- **Testing**: [spec/testing.md](spec/testing.md)
- **Theming & locale**: [spec/ui-conventions.md](spec/ui-conventions.md);
  [`src/routes/+layout.svelte`](src/routes/+layout.svelte);
  [`static/themes/`](static/themes/)
- **UI conventions**: [spec/ui-conventions.md](spec/ui-conventions.md)
- **Use cases**: [spec/routes.md](spec/routes.md)

## Sibling projects

- **[`../case-folder-service-with-rust`](../case-folder-service-with-rust)**
  — the JSON API back-end this client talks to.
- **`~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/`**
  — source of the locale and theme pickers, resolved via SvelteKit
  `kit.alias`; required at dev + build time.
- Five upstream Main-X-Services under `~/git/sixarm/main-x-service/`
  (Patient, Place, Worker, Thing, Event) — only relevant when running
  the Loco app against real services rather than its in-process stubs.
