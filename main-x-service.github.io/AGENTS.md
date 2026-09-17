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
  `@lilydesignsystem/svelte-theme-picker` from the family's Lily
  dependency set (no locale/share/text-size pickers — this isn't an
  operator front-end with per-user preferences to persist), and — unlike
  those front-ends — no `@lilydesignsystem/svelte-picker-bar` either,
  precisely because it needs only the one picker.
- `.github/workflows/deploy.yml` lives **inside this subproject** on
  purpose: it does nothing while nested in the monorepo (GitHub Actions
  only reads a repo's *root* `.github/workflows/`), and becomes the live
  Pages-deploy workflow the moment `git subtree split` makes this
  directory the root of its own repo.
- `@lilydesignsystem/svelte-theme-picker` is a registry dependency
  (`^0.1.0`, under the `@lilydesignsystem` npm org scope — not the
  earlier unscoped `lily-design-system-svelte-theme-picker`), **not** a
  `file:` path onto the sibling `lilydesignsystem`
  checkout — a `file:` path here would be doubly wrong: it makes
  `pnpm install` impossible on any CI runner (the same reason every
  operator front-end switched, `agents/share/svelte-front-end-stack.md`),
  *and* its relative depth is only ever correct while this directory is
  nested inside the monorepo — `git subtree split` promotes it to a
  repo root one level shallower, breaking the path outright. Same
  reasoning applies to `static/assets/themes`: it is a **vendored copy**
  (45 real `.css` files, not a symlink) so the exported sibling repo —
  which `git subtree split --prefix=main-x-service.github.io` derives
  from *only this directory's* history — carries its own theme
  stylesheets rather than a dangling reference to something outside the
  subtree entirely. Re-copy from `vendor/lily-design-system-themes/` at
  the monorepo root (or straight from the Lily checkout) if themes
  change; don't hand-edit the vendored files.

## Publishing (see the spec for the full contract)

```sh
# from the main-x-service repo root
make github-pages
```

That target (repo-root `Makefile`, delegating to `bin/make-github-pages`)
runs `git subtree push --prefix=main-x-service.github.io github-pages
main` — the only route from a monorepo commit to the live site, since
GitHub Pages serves the sibling repo, not this one. Requires a
`github-pages` remote, added once per checkout (not something the repo
can commit):

```sh
git remote add github-pages git@github.com:SixArm/main-x-service.github.io.git
```
