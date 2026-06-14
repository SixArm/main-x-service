import { base } from "$app/paths";

// Read via import.meta.env (not $env/dynamic/public) so this module
// also loads cleanly under vitest. 5150 is the case service's loco
// default dev port.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

/**
 * Origin of the Case Service REST API. Sourced from `PUBLIC_API_BASE_URL`,
 * defaulting to the loco dev port `5150`.
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
 * Origin of the central authentication front-end (SSO sign-in SPA).
 * Sourced from `VITE_AUTH_FRONTEND_URL`, defaulting to its vite dev port
 * `5173`. See {@link signInUrl}.
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
 * a DOM; they default to the live `location.origin` and the SvelteKit
 * `base`. A trailing slash on `AUTH_FRONTEND_URL` is trimmed to avoid a
 * `//signin` path.
 *
 * @param origin Absolute origin to return to; defaults to `location.origin` (or `""` under SSR).
 * @param basePath SvelteKit base path appended to the origin; defaults to `base`.
 * @returns The fully-qualified `${AUTH_FRONTEND_URL}/signin?return_to=…` URL.
 */
export function signInUrl(origin?: string, basePath?: string): string {
    // Fall back to the live origin in the browser; empty string under SSR.
    const resolvedOrigin =
        origin ?? (typeof location !== "undefined" ? location.origin : "");
    const resolvedBase = basePath ?? base;
    // Encode origin+base into a single query param so its `:` and `/` ride safely.
    const returnTo = encodeURIComponent(resolvedOrigin + resolvedBase);
    // Trim any trailing slash so we never emit `//signin`.
    const root = AUTH_FRONTEND_URL.replace(/\/+$/, "");
    return `${root}/signin?return_to=${returnTo}`;
}
