// Build/runtime configuration for the worker operator front-end.
//
// API calls go to the same-origin BFF proxy (`/api/proxy/...`), whose
// server handler injects the server-exchanged PASETO and forwards to the
// worker service. The browser never holds a token (see
// `agents/share/authentication-sessions.md`). Sign-in is the app's own
// `/signin` (per-app magic-link login), not a cross-origin token handoff.

/**
 * Base URL the API client posts to: the same-origin BFF proxy. Absolute
 * (rooted at `location.origin`) so the client's `new URL(path, base)`
 * resolves; falls back to the dev origin under SSR/tests.
 */
export const API_BASE_URL: string =
  (typeof location !== "undefined"
    ? location.origin
    : "http://localhost:5173") + "/api/proxy";
