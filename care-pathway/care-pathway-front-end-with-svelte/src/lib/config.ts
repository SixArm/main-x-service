import { base } from "$app/paths";

// Read via import.meta.env (not $env/dynamic/public) so this module
// also loads cleanly under vitest. 5150 is the care-pathway service's
// loco default dev port.
// Narrow `import.meta` to the optional Vite `env` bag. Typed locally
// (rather than relying on ambient Vite types) so the access is safe even
// when this module is imported outside the Vite/SvelteKit pipeline.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

/**
 * Base URL of the care-pathway service REST API that every request is
 * issued against. Sourced from the build-time `PUBLIC_API_BASE_URL`
 * environment variable; falls back to the loco.rs dev default port
 * (`:5150`) when unset (e.g. local dev or unit tests).
 */
export const API_BASE_URL: string =
    meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:5150";

// Base URL of the central authentication front-end (the SSO sign-in
// SPA). The operator clicks "Sign in" and is sent to
// `${AUTH_FRONTEND_URL}/signin?return_to=…`; after the passwordless
// magic-link the auth front-end hands the access token back in the URL
// fragment (see `agents/share/jwt-enforcement.md`). Default targets the
// authentication front-end's vite dev port (5173); override per
// deployment with `VITE_AUTH_FRONTEND_URL`.
/**
 * Base URL of the central authentication (SSO) front-end. The "Sign in"
 * affordance redirects here; the auth front-end performs the passwordless
 * magic-link flow and hands the access token back in the URL fragment.
 * Sourced from `VITE_AUTH_FRONTEND_URL`; falls back to the auth
 * front-end's vite dev default port (`:5173`).
 */
export const AUTH_FRONTEND_URL: string =
    meta.env?.VITE_AUTH_FRONTEND_URL ?? "http://localhost:5173";

/**
 * Build the cross-origin SSO sign-in URL for the authentication
 * front-end, carrying an absolute `return_to` pointing back at this
 * operator SPA (origin + SvelteKit base path). After verifying, the
 * auth front-end appends the token to `return_to#access_token=<jwt>`
 * (only when our origin is on its allowlist).
 *
 * `origin` / `basePath` are injectable so this is unit-testable without
 * a DOM; they default to the live `window.location.origin` and the
 * SvelteKit `base`. A trailing slash is trimmed off the configured
 * `AUTH_FRONTEND_URL` so the result never doubles up to `//signin`.
 *
 * @param origin - Absolute origin to return to (e.g. `https://ops.example.com`);
 *   defaults to `window.location.origin`, or `""` under SSR/tests.
 * @param basePath - SvelteKit base path appended to `origin` in `return_to`;
 *   defaults to the configured `base`.
 * @returns The fully-formed `${AUTH_FRONTEND_URL}/signin?return_to=<encoded>` URL.
 */
export function signInUrl(origin?: string, basePath?: string): string {
    // Fall back to the live browser origin; tolerate SSR/test (no `location`).
    const resolvedOrigin =
        origin ?? (typeof location !== "undefined" ? location.origin : "");
    const resolvedBase = basePath ?? base;
    // The whole `origin + base` rides inside a single query param, so it
    // must be percent-encoded (colon + slashes included).
    const returnTo = encodeURIComponent(resolvedOrigin + resolvedBase);
    // Trim any trailing slash(es) so we never produce `…//signin`.
    const root = AUTH_FRONTEND_URL.replace(/\/+$/, "");
    return `${root}/signin?return_to=${returnTo}`;
}
