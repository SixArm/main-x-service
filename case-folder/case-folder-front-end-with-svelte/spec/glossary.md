# Glossary

> Part of the [Svelte edition specification](index.md). Shared domain
> vocabulary: [root glossary](../../spec/glossary.md). Svelte-specific terms:

| Term         | Meaning                                                                                              |
| ------------ | ---------------------------------------------------------------------------------------------------- |
| API client   | `src/lib/api/client.ts` — the only module that calls `fetch` for `/api/*`.                            |
| Cache        | `src/lib/store/cache.svelte.ts` — the rune-reactive container that pages read.                        |
| CSR          | Client-Side Rendering. `ssr = false` in `+layout.ts`.                                                 |
| Lily         | Lily Design System (headless Svelte) — the UI primitive library.                                      |
| Lily helper  | A sibling, opinionated helper component (locale picker, theme picker) that owns one lifecycle.        |
| LocalePicker | `lily-design-system-svelte-locale-picker` — sets `<html lang>`/`<html dir>` from a radiogroup, persists to `localStorage`. |
| ThemePicker  | `lily-design-system-svelte-theme-picker` — swaps `<link>` href + `<html data-theme>` to load one of the shared Lily themes from `static/assets/themes/` (~41-theme catalogue). |
| Rune         | Svelte 5 reactivity primitive (`$state`, `$derived`, `$effect`, `$props`).                            |
| SVAR Grid    | `wx-svelte-grid` — the data grid used on the dashboard.                                               |
| load + cache | The wiring pattern: `+page.ts` loads via `api.*` into the cache; `+page.svelte` reads reactively.     |
