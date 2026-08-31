# Agent guide — main-x-service.github.io

Per [`spec/monorepo-github-pages/index.md`](../spec/monorepo-github-pages/index.md):
this is the monorepo's GitHub Pages **subproject**. It lives here, inside
`main-x-service/`, and is periodically exported via `git subtree` to a
sibling top-level read-only repository `SixArm/main-x-service.github.io`,
which is what GitHub Pages actually serves.

## Single source of truth

- Treat [`spec/index.md`](spec/index.md) as the single source of truth for
  this site's scope and content model.
- The **content** on this site (the family overview, architecture summary,
  and the subproject table) is deliberately kept in sync with the root
  [`AGENTS.md`](../AGENTS.md) and [`agents/share/`](../agents/share/) —
  update both together rather than letting this site's copy drift into
  its own story.

## What this is

A SvelteKit project (`@sveltejs/adapter-static`) that prerenders a small,
public documentation front door for the `main-x-service` monorepo: what
the Main X Index is, its architecture, and the full subproject index with
links back to source. It does not implement or document any one
subproject's own API — that stays in that subproject's own `spec/`.

## Routes

- `/` — home (elevator pitch + at-a-glance facts)
- `/architecture/` — layered request flow, the two internal service
  shapes, cross-cutting subsystems
- `/subprojects/` — the full family index (services, matchers, libraries,
  front-ends, consumer apps, cross-cutting services), each linking to its
  GitHub source
- `/about/` — what this site is, spec-driven development, AI-assisted
  development, license

## Working rules

- **Never work in the exported sibling repo**
  (`~/git/sixarm/main-x-service.github.io`) — it is a read-only
  `git subtree split` output. All edits happen here, in the monorepo, and
  are re-exported.
- This is a **static, unauthenticated** site (`prerender = true` at the
  root layout) — no BFF, no sessions, no forms. It uses only
  `lily-design-system-svelte-theme-picker` from the family's Lily
  dependency set (no locale/share/text-size pickers — this isn't an
  operator front-end with per-user preferences to persist).
- `.github/workflows/deploy.yml` lives **inside this subproject** on
  purpose: it does nothing while nested in the monorepo (GitHub Actions
  only reads a repo's *root* `.github/workflows/`), and becomes the live
  Pages-deploy workflow the moment `git subtree split` makes this
  directory the root of its own repo.
- Keep `static/assets/themes` as a symlink to the shared
  `lilydesignsystem/lily-design-system/themes` checkout (same convention
  as every operator front-end) — do not vendor a copy.

## Publishing (see the spec for the full contract)

```sh
# from the main-x-service repo root
git subtree split --prefix=main-x-service.github.io -b export/main-x-service.github.io
# then push that branch's tree to the sibling repo's main, e.g.:
git push git@github.com:SixArm/main-x-service.github.io.git export/main-x-service.github.io:main
```
