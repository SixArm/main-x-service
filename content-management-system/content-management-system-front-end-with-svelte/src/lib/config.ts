// Build/runtime configuration for the CMS authoring front-end.
//
// API calls go to the same-origin BFF proxy (`/api/proxy/...`); the
// browser never holds a token (see `../spec/auth.md`). The base is
// deliberately **relative**: browser fetch resolves it against the
// page origin, and server `load` functions pass SvelteKit's `fetch`,
// which resolves relative URLs internally — so the same client works
// under dev, preview, and production without knowing its own port.

/** Base path the API client calls: the same-origin BFF proxy. */
export const API_BASE_URL = "/api/proxy";

/** Where a preview render is fetched from. Deliberately its own path
 *  rather than the generic proxy: a preview token must stay on the
 *  server, so the browser asks for "the preview of this revision" and
 *  the BFF mints and spends the token (see `../spec/auth.md`). */
export const PREVIEW_BASE_URL = "/preview";
