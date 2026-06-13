import { base } from "$app/paths";

// Read via import.meta.env (not $env/dynamic/public) so this module
// also loads cleanly under vitest. 5150 is the care-pathway service's
// loco default dev port.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

export const API_BASE_URL: string =
    meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:5150";

// Base URL of the central authentication front-end (the SSO sign-in
// SPA). The operator clicks "Sign in" and is sent to
// `${AUTH_FRONTEND_URL}/signin?return_to=…`; after the passwordless
// magic-link the auth front-end hands the access token back in the URL
// fragment (see `agents/share/jwt-enforcement.md`). Default targets the
// authentication front-end's vite dev port (5173); override per
// deployment with `VITE_AUTH_FRONTEND_URL`.
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
 */
export function signInUrl(origin?: string, basePath?: string): string {
    const resolvedOrigin =
        origin ?? (typeof location !== "undefined" ? location.origin : "");
    const resolvedBase = basePath ?? base;
    const returnTo = encodeURIComponent(resolvedOrigin + resolvedBase);
    const root = AUTH_FRONTEND_URL.replace(/\/+$/, "");
    return `${root}/signin?return_to=${returnTo}`;
}
