// Render as a single-page app (SPA) — disable SSR globally.
// SVAR Svelte Grid uses browser-only APIs and can't render server-side,
// so SSR breaks any route that mounts <Grid>. Since this front-end is a
// thin client over the backend REST API (no SEO requirement), CSR-only
// is the right default. Server `load`/actions (`+layout.server.ts`,
// `+page.server.ts`, `signin`/`verify`) still run server-side — that is
// where the BFF holds the httpOnly session cookie.
export const ssr = false;
export const prerender = false;
