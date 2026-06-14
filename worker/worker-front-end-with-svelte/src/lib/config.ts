// REST API base URL. Configured via PUBLIC_API_BASE_URL. Falls back to
// the service crate's default (8080). We read via `import.meta.env`
// (Vite build-time injection) rather than SvelteKit's `$env/dynamic/public`
// so this module loads cleanly under vitest, which doesn't run the
// SvelteKit Vite plugin.
// Narrow `import.meta` to expose `.env` without depending on the
// SvelteKit/Vite ambient types, so vitest (which doesn't load the
// SvelteKit Vite plugin) still type-checks and runs this module.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };
/**
 * Base URL of the Worker Service REST API that every {@link ApiClient}
 * targets.
 *
 * Resolved at build time from the `PUBLIC_API_BASE_URL` Vite env var so it
 * can be overridden per deployment without code changes. Falls back to the
 * Worker Service crate's default dev port (`8080`) when the var is unset
 * (e.g. local dev and unit tests).
 */
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
