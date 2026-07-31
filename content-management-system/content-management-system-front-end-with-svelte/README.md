# Content Management System — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../content-management-system-service-with-rust/):
the authoring and editorial UI — entry list and structured block
editor, revision history with diff and restore, the review queue
and workflow actions, the schedule calendar, the asset library, the
translation dashboard with staleness, site settings (locales,
fallback chains, templates, menus, redirects), a delivery preview
panel, and the content-health and throughput insights.

> ⚠️ **Demo software.** Not a production CMS; synthetic content
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (CMS-T25/T26, 2026-07-31).** The app shell,
the BFF proxy and session flow, the typed API client, 13-locale
i18n, and all seven authoring views are live: entries, entry detail
(locale matrix, block editor, revision history, workflow, publish
gate, preview), assets, workflow, translations, insights, and site
settings. 28 vitest tests + 7 Playwright specs; every view verified
in a browser against the seeded service.

## Stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no
token in browser JS) · 13-locale i18n from the start · Lily Design
System (headless + `ThemePicker` + `LocalePicker` — note the
upstream rename from `*Select`) · SVAR Svelte DataGrid where it
fits · vitest + Playwright (`page.route`-stubbed).

## Quick start

```bash
pnpm install
CMS_API_URL=http://localhost:5150 pnpm dev   # the Loco sibling
pnpm test           # vitest
pnpm exec playwright test   # stubbed; no service required
pnpm check          # svelte-check
```

Environment: `CMS_API_URL` (the CMS service) and `AUTH_API_URL` (the
authentication service). The browser talks only to this app's own
origin; both variables are read server-side.

## Preview is server-side, on purpose

A preview token renders **unpublished** content, so it never reaches
the browser. The BFF proxy **refuses** `…/variants/{locale}/preview`
and `/api/preview-tokens/…` with a `403` that says where to go
instead; the app's own `/preview/{pid}/{locale}?site=…` route mints
the token, renders with it, revokes it, and returns only the result —
`no-store` and `noindex`. There is no shareable URL to leak.

## Views

| Area | Views |
|---|---|
| Authoring | entry list + filters, structured block editor, field forms per content type, revision history + diff + restore |
| Workflow | review queue, transition actions with reasons, schedule calendar, lock indicators |
| Assets | library grid, upload with alt-text prompt, usage ("where used"), rendition list |
| Localization | per-entry locale matrix, translation requests, staleness with revisions-behind |
| Site | locales + fallback chains, templates, menus, redirects, audience rules, webhooks |
| Delivery | preview panel (token-scoped), route inspector, sitemap view |
| Insights | content health by rule, editorial throughput, locale coverage, backlogs |

Site settings are **read-only** for now, and the page says so rather
than showing a form that silently fails.

## Design notes that shape the UI

- The editor is a **block editor**, not a rich-text blob — the
  service stores structured blocks, and the client must not invent
  HTML ([../spec/authoring.md](../spec/authoring.md)).
- **"Save" and "go live" are different verbs.** The UI shows which
  revision is published and warns plainly when the draft has moved
  past it ([../spec/workflow.md](../spec/workflow.md)).
- A stale save is a **`409` with the competing revision**, rendered
  as a real conflict UI, never a silent overwrite.
