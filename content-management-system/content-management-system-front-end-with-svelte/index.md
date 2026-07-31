# Content Management System front-end — documentation index

The SvelteKit authoring and editorial client for the
[Loco JSON API sibling](../content-management-system-service-with-rust/).
The app owns no data: every page round-trips through the API.

## Start here

- **[README.md](README.md)** — what this is, status, quick start,
  and why preview is server-side.
- **[../spec/](../spec/index.md)** — the cross-cutting specification
  (**the single source of truth**).
- **[spec/](spec/index.md)** — this edition's stack-specific spec.
- **[AGENTS.md](AGENTS.md)** — working agreements for contributors.
- **[CHANGELOG.md](CHANGELOG.md)** — Keep a Changelog format.
- **The task queue** — [../spec/tasks.md](../spec/tasks.md)
  (CMS-T25/T26, both done 2026-07-31).

## Quick start

```bash
pnpm install
CMS_API_URL=http://localhost:5150 pnpm dev     # the Loco sibling
```

| Command | What it does |
|---|---|
| `pnpm dev` | dev server on `:5173` |
| `pnpm build` / `pnpm preview` | production build, then serve it |
| `pnpm check` | `svelte-check` (TypeScript strict) |
| `pnpm test` | vitest — the pure modules and the endpoint map |
| `pnpm exec playwright test` | e2e over a `page.route`-stubbed API; no service needed |
| `pnpm lint` / `pnpm format` | prettier |

Environment: `CMS_API_URL` (the CMS service) and `AUTH_API_URL` (the
authentication service). Both are read **server-side only** — the
browser talks to this app's own origin and nothing else.

## The routes

| Route | What it is for |
|---|---|
| `/` | dashboard: content health and the backlog for a chosen site |
| `/entries` | the entry list, filtered by content type or key |
| `/entries/{pid}` | the working surface: locale matrix, block editor, revision history, workflow, publish gate, preview |
| `/assets` | the asset library, the alt-text gate, orphans, storage |
| `/workflow` | what is waiting for a person, and what is waiting for a clock |
| `/translations` | locale coverage, open requests, staleness |
| `/insights` | health by rule, editorial throughput, time in state |
| `/settings` | locales and fallback chains, content types, templates, menus, redirects, webhooks (read-only) |
| `/signin` · `/verify` · `/signout` | magic-link session flow, entirely server-side |

Two routes are **not** pages:

- `/api/proxy/*` — the BFF reverse proxy. It injects the bearer
  server-side and **refuses** to forward the preview-token surface.
- `/preview/{pid}/{locale}?site=…` — mints a preview token, renders
  with it, revokes it, and returns only the render.

## How the code is arranged

```
src/
├── lib/
│   ├── api/client.ts      fetch wrapper; ApiError carries the status
│   ├── api/cms.ts         every endpoint the UI calls, in one file
│   ├── blocks.ts          the block model (pure)
│   ├── format.ts          the honesty rules (pure)
│   ├── i18n.svelte.ts     13 locales, one reactive holder
│   ├── proxy-paths.ts     what the BFF refuses to forward
│   ├── components/        BlockEditor · StateBadge · SitePicker
│   └── server/            config · session · auth  (never bundled)
└── routes/                the pages above
```

`blocks.ts` and `format.ts` are pure and exhaustively unit-tested, so
the views stay thin and the rules are checkable without a browser.

## Worked example: adding a view

Say you want a page listing everything scheduled to go live.

**1. Add the endpoint** to `src/lib/api/cms.ts` — never interpolate a
path at the call site, because this file is what a reader checks
against the service's OpenAPI document:

```ts
export const schedules = (
  sitePid: string,
  o?: Options,
): Promise<{ as_of: string; queued: ScheduledItem[] }> =>
  api(`/api/sites/${sitePid}/schedules`, o);
```

**2. Pin the path** in `tests/unit/cms-api.test.ts`. Then check it
against the running service, **with its method** — a path that exists
for `POST` and not `GET` looks fine in a path-only comparison and
404s at runtime. That mistake is why this instruction exists.

**3. Write the view**, formatting rather than computing:

```svelte
<script lang="ts">
  import * as cms from "$lib/api/cms";
  import { when } from "$lib/format";
  let queued = $state<ScheduledItem[]>([]);
  $effect(() => { cms.schedules(site).then((r) => (queued = r.queued)); });
</script>
{#each queued as item (item.entry_pid + item.locale)}
  <tr><td>{item.entry_key}</td><td>{when(item.publish_at)}</td></tr>
{/each}
```

Key an `{#each}` by position when the item has no id — two rows can
legitimately be identical, and a duplicate key crashes the whole view.

**4. Add the strings** to all 13 locales in `i18n.svelte.ts`. The
parity test fails otherwise, on purpose: a missing key falls back to
English inside an otherwise-translated page, which reads as a content
bug and gets reported by nobody.

**5. Stub it in Playwright.** Register the catch-all `404` route
**first** — Playwright matches in reverse registration order, so a
catch-all added last shadows every specific stub.

## The rules this client will not bend

1. **Blocks, never HTML.** The editor edits the block model and posts
   blocks. There is no `contenteditable` that serializes markup, no
   `{@html}` anywhere, and deliberately no `toHtml`/`fromHtml` helper
   — a helper like that gets used, and the lossless round trip ends
   the moment it is.
2. **No token in browser JS.** The session cookie is httpOnly; the
   SvelteKit server exchanges it for a short-lived PASETO. Preview
   tokens never leave the server at all.
3. **A `409` is a comparison, never a retry.** Retrying discards
   whoever saved first.
4. **The client formats; it does not compute.** A `null` ratio renders
   as no-data, never `0%`.
5. **Staleness has three answers** — up to date, N revisions behind,
   and *unknown*. Collapsing the third into the first tells a
   translator their page is fine when nobody knows.
6. **Alt text is prompted, and its absence is explained** in terms of
   the consequence: the page will not publish. A CMS that makes alt
   text easy to skip produces an inaccessible site.

## Known gotchas

- Lily renamed its helper packages `*-select` → `*-picker`
  (`LocalePicker` / `ThemePicker`), and the DOM changed with it: a
  button plus a `ul` listbox, not a `<select>`. All 16 front-ends were
  migrated on 2026-07-31, so copy-adapting a sibling is safe again —
  but style `.locale-picker-button`, not `select`, and drive the
  listbox by clicking rather than with `selectOption`.
- `browser` being true does not mean `localStorage` works (Safari
  private mode throws). The i18n holder guards both read and write,
  because it runs in a module-level constructor: an unguarded access
  takes the whole app down, not just the locale switcher.
- A SvelteKit endpoint may only export HTTP verbs and a fixed set of
  config names — hence `$lib/proxy-paths.ts` rather than exporting the
  predicate from `+server.ts`.
