// Render as a single-page app (SPA) — disable SSR globally.
// SVAR Svelte Grid uses browser-only APIs and can't render server-side,
// so SSR breaks any route that mounts <Grid>. Since this front-end is a
// thin client over the backend REST API (no SEO requirement), CSR-only
// is the right default.
/** Disable server-side rendering for the whole app (CSR/SPA only). */
export const ssr = false;
/** Disable prerendering; pages are rendered live in the browser. */
export const prerender = false;
