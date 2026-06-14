// SPA mode: no SSR, no prerender. The session lives in the browser
// (localStorage), so every route renders client-side.

/** Disable server-side rendering — the session is browser-only. */
export const ssr = false;
/** Disable prerendering — every route is dynamic and client-rendered. */
export const prerender = false;
