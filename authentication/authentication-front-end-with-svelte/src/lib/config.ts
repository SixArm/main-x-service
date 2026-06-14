// Build/runtime configuration for the auth front-end. Both values are
// read from Vite's `import.meta.env` so the module imports cleanly under
// vitest (where SvelteKit's `$env/*` virtual modules are unavailable).
//
// Read via import.meta.env (not $env/dynamic/public) so this module
// also loads cleanly under vitest. 5150 is the auth service's loco
// default dev port.
// Narrow `import.meta` to expose the optional `env` bag without assuming
// any particular key exists (keys are absent when undefined at build).
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

/**
 * Base URL of the Authentication Service REST API.
 *
 * Sourced from the `PUBLIC_API_BASE_URL` build env var; falls back to the
 * loco.rs dev default (`http://localhost:5150`) when unset. Used by
 * {@link AuthRepository.withFetch} to construct the {@link ApiClient}.
 */
export const API_BASE_URL: string =
    meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:5150";

/**
 * Comma-separated allowlist of operator-app origins that the cross-origin
 * SSO handoff may redirect the access token to (see `$lib/auth/return-to`).
 *
 * This is the security-critical knob for the token handoff: only an origin
 * named here (exact `scheme://host[:port]`) is ever allowed to receive the
 * bearer token. Each entry is an exact origin; paths/queries are ignored.
 * Unset/empty means same-origin only — no external handoff occurs.
 *
 * Sourced from `VITE_RETURN_TO_ALLOWLIST` at build time. Parsed by
 * {@link parseAllowlist} and consumed by {@link isAllowedReturnTo} /
 * {@link nextDestination}.
 */
/// Comma-separated allowlist of operator-app origins that the
/// cross-origin SSO handoff may redirect the access token to (see
/// `$lib/auth/return-to`). Each entry is an exact `scheme://host[:port]`
/// origin. Unset/empty ⇒ same-origin only (no external handoff).
export const RETURN_TO_ALLOWLIST: string =
    meta.env?.VITE_RETURN_TO_ALLOWLIST ?? "";
