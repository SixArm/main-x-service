// SPA mode: no SSR, no prerender. Pages render client-side and reach the
// services via the same-origin BFF proxy; the session is an httpOnly
// cookie resolved server-side (`+layout.server.ts`), never in the browser.
export const ssr = false;
export const prerender = false;
