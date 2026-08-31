# main-x-service.github.io — spec

Single source of truth for this subproject. Code conforms to this spec,
not the reverse (family-wide discipline, see the root
[AGENTS.md](../../AGENTS.md)).

## 1. Purpose and vision

Give `main-x-service` — a ~55-crate Rust/SvelteKit monorepo with no
existing public front door — a small, honest, low-maintenance
documentation site: what it is, how it's put together, and where every
piece of source actually lives. Not a marketing site, not API docs (each
subproject documents its own API), not a mirror of the repo's internal
`agents/share/` reference docs — a front door that points into the real
repo rather than duplicating it at length.

## 2. Scope

**In scope:** an elevator-pitch home page, an architecture summary
(the layered request flow + the two internal service shapes), and a full
subproject index (every service/matcher/library/front-end/consumer-app/
cross-cutting-service, linking to its GitHub source).

**Out of scope:** per-subproject API reference (stays in each
subproject's own `spec/`), a component catalog (this isn't a design
system — that's `lilydesignsystem.github.io`'s job), authenticated
content, forms, or any BFF/session machinery — this site has none of the
family's operator-front-end concerns.

## 3. Requirements

- R1: Static, prerendered at build time (`@sveltejs/adapter-static`,
  `prerender = true`). No SSR runtime, no server.
- R2: Content on `/`, `/architecture/`, and `/subprojects/` stays
  consistent with the root `AGENTS.md` and `agents/share/architecture.md`
  — update both together when either changes, rather than letting this
  site's copy drift into its own story.
- R3: Uses the family's Lily Design System (`lily-design-system-svelte-
  theme-picker`) for visual consistency with the operator front-ends,
  scoped down to just theming (no locale/share/text-size pickers — there
  is no per-user state to persist on a static, unauthenticated site).
- R4: `.github/workflows/deploy.yml` lives inside this subproject
  (inert while nested in the monorepo; becomes the live Pages-deploy
  workflow once `git subtree split` promotes this directory to its own
  repo root).
- R5: Every page links back to the real GitHub/Codeberg source rather
  than re-hosting content that would drift.

## 4. Design

SvelteKit 2 + Svelte 5 runes + TypeScript strict, mirroring the family's
front-end conventions (`agents/share/architecture.md`'s SvelteKit stack)
but stripped to what a static site needs: no `src/lib/api/`, no BFF
`src/lib/server/`, no auth. Four routes (`+page.svelte` each), one shared
`+layout.svelte` for the nav chrome and theme picker. The subproject
index (`/subprojects/`) is a small typed data table in the page's own
`<script>` block, not fetched at runtime — this is a `git subtree`-
exported static site with no build-time access to the rest of the
monorepo's file tree once split out, so the data has to be inlined rather
than read from `AGENTS.md`/`llms.json` at build time.

## 5. Publishing

See the root [`spec/monorepo-github-pages/index.md`](../../spec/monorepo-github-pages/index.md)
for the full contract. In short: `git subtree split --prefix=main-x-service.github.io`
from the monorepo root produces the tree pushed to the sibling
`SixArm/main-x-service.github.io` repo, which is what GitHub Pages
actually serves. Never edit that sibling directly.

## 6. Tasks

- [x] Scaffold the SvelteKit project (adapter-static, Lily theme picker,
      four routes).
- [x] `.github/workflows/deploy.yml`, pinned to Node 26 per
      `spec/node-current-version/index.md`.
- [ ] Create the `SixArm/main-x-service.github.io` GitHub repository and
      perform the first `git subtree split` + push — deliberately not
      done as part of this subproject's initial scaffold, since creating
      a new public repository is an outward-facing action outside this
      site's own code; do this as an explicit, separate step.
- [ ] Confirm the actual GitHub Pages URL / custom domain (if any) once
      the sibling repo exists, and update `homepage` in `package.json`
      and the footer links accordingly.
- [ ] Consider whether `/subprojects/` should eventually be generated
      from `llms.json` at build time via a monorepo-root prebuild script,
      rather than hand-maintained — deferred until the hand-maintained
      version has actually drifted once, per the family's general
      "don't build the abstraction before the second real need" posture.

## 7. Open questions

- Custom domain (e.g. a `CNAME` under the SixArm org) vs. the default
  `sixarm.github.io/main-x-service.github.io/` URL — an org decision, not
  a code one.
