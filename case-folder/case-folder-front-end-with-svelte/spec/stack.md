# Stack & versions

> Part of the [Svelte edition specification](index.md).

| Layer                | Choice                                                          | Pin                       |
| -------------------- | --------------------------------------------------------------- | ------------------------- |
| App framework        | [SvelteKit 2](https://svelte.dev/docs/kit) (Svelte 5 runes)     | `@sveltejs/kit ^2.55.0`   |
| Language             | TypeScript (`strict`)                                           | `^5.8.0`                  |
| Build tool           | Vite                                                            | `^6.2.0`                  |
| Package manager      | npm — **canonical**; `package-lock.json` committed (no `pnpm-*`) |                          |
| Headless UI          | [Lily Design System](https://lilydesignsystem.io) Svelte headless |                        |
| Lily helpers         | `lily-design-system-svelte-locale-select`, `lily-design-system-svelte-theme-select` (`file:` deps, symlinked from the sibling git path) | source |
| Data grid            | SVAR Svelte (`wx-svelte-grid` + `Willow` theme)                 | `^2.1.0`                  |
| Styling              | Plain CSS + NHS UK tokens, runtime theme swap via `:root[data-theme]` | —                   |
| State                | Rune-based reactive cache, one module singleton                 |                           |
| Rendering            | **Client-side only** (`ssr = false` in `+layout.ts`)            |                           |
| Type check           | `svelte-check`                                                  |                           |

**Why CSR-only.** The Loco API lives on `http://localhost:5150` during
dev and the SvelteKit app on `:5173`. Server-side `load` calls work
during SSR but a production deployment behind a single reverse proxy is
a separate concern. CSR is the simplest correct default.
