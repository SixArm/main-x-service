// Render as a single-page app (SPA) — disable SSR globally.
// SVAR Svelte Grid uses browser-only APIs and can't render server-side,
// so SSR breaks any route that mounts <Grid>. Since this front-end is a
// thin client over the backend REST API (no SEO requirement), CSR-only
// is the right default.
/** Disable server-side rendering for all routes (CSR-only SPA). */
export const ssr = false;
/** Disable static prerendering — pages are data-driven from the live API. */
export const prerender = false;
