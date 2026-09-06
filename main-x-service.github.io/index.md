# main-x-service.github.io

The public documentation site for the [Main X Index](../index.md)
monorepo — see [AGENTS.md](AGENTS.md) for how this subproject is built
and published, and [spec/index.md](spec/index.md) for its scope and
content model.

## Quick start

```sh
cd main-x-service.github.io
pnpm install
pnpm dev       # http://localhost:5173
pnpm build     # static output in build/
pnpm check     # svelte-check
```

## What it publishes

- `/` — what the Main X Index is, at a glance
- `/architecture/` — the layered request flow and the two internal
  service shapes
- `/subprojects/` — the full family index, linking to every crate's
  source
- `/about/` — spec-driven development, AI-assisted development, license

Published to GitHub Pages from a sibling read-only repository,
`SixArm/main-x-service.github.io`, produced from this directory via
`git subtree` — see [AGENTS.md](AGENTS.md#publishing-see-the-spec-for-the-full-contract).
