// Build/runtime configuration for the workforce-planning-management operator front-end.
//
// API calls go to the same-origin BFF proxy (`/api/proxy/...`); the
// browser never holds a token (see `../spec/auth.md`). The base is
// deliberately **relative**: browser fetch resolves it against the
// page origin, and server `load` functions pass SvelteKit's `fetch`,
// which resolves relative URLs internally — so the same client works
// under dev, preview, and production without knowing its own port.

/** Base path the API client calls: the same-origin BFF proxy. */
export const API_BASE_URL = "/api/proxy";

/** Whiteboard poll interval (ms). */
export const BOARD_POLL_MS = 15_000;
