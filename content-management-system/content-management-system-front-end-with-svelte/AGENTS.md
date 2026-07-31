# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **SvelteKit browser client** for the
[Loco JSON API sibling](../content-management-system-service-with-rust/):
the authoring and editorial UI — block editor, revision history and
diff, review queue and workflow actions, schedule calendar, asset
library, translation dashboard, site settings, delivery preview,
and content insights. The Svelte app owns no data; every page
round-trips through the API.

> ⚠️ Demo software, not a production CMS. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth —
   especially [authoring](../spec/authoring.md) (the block model),
   [workflow](../spec/workflow.md) (transitions and reasons),
   [auth](../spec/auth.md) (personas, preview tokens, masking), and
   [insights](../spec/insights.md) (honesty rules). Task queue:
   [`../spec/tasks.md`](../spec/tasks.md) (CMS-T25/T26).
2. **Family front-end conventions.** SvelteKit 2, **Svelte 5 runes
   only** (no legacy stores/`$:`), TypeScript strict, SPA mode.
   Drift between front-ends is accepted — copy-adapt from a sibling
   (the CRM and PPM front-ends are the closest sources for the BFF
   proxy, i18n, and board/grid patterns); no shared package.
3. **BFF auth.** The SvelteKit server holds the cookie session and
   exchanges it for short-lived PASETO tokens; **no token in browser
   JS, no localStorage credentials**. Mutations go through server
   routes with CSRF protection. **Preview tokens are never put in a
   shareable client URL** — preview is fetched server-side.
4. **Blocks, never HTML.** The editor edits the structured block
   model and posts blocks. Do not add a `contenteditable` that
   serializes markup, and never render server content with
   `{@html}` — the service's sanitizer is a boundary control, not
   permission to trust its output blindly.
5. **Publishing honesty.** Always show which revision is live and
   whether the draft has moved past it; render the `409` conflict
   as a real comparison, not a retry button; show reasons on every
   transition that requires one.
6. **Localization honesty.** Show `locale_served` vs
   `locale_requested` in preview, and surface translation staleness
   with the count of source revisions behind — never a bare badge.
7. **No client-side insight math.** Health findings, throughput,
   time-in-state, and staleness come from the API; the client
   formats, it does not compute. Render `as_of`; `null` ratios show
   a no-data state, never `0%`.
8. **Accessibility is the product here.** A CMS UI that makes alt
   text easy to skip produces an inaccessible site. Alt text is
   prompted at upload and blocks publish (the service enforces it);
   the UI must explain the refusal, not hide it.
9. **13-locale i18n from the start** (the PPM lesson) with the
   full-coverage parity test.
10. **Tests.** vitest for the API client path map, block-model
    transforms, the diff renderer, and staleness formatters;
    Playwright over a `page.route`-stubbed API mirroring the
    endpoint contract (unstubbed = 404-loud).

## Running (target)

```bash
pnpm install
pnpm dev           # expects the Loco sibling (stub mode) on its default port
pnpm test          # vitest
pnpm exec playwright test
```
