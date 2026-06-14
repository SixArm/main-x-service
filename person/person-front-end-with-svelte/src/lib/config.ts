// Person Service REST API base URL. Configured via PUBLIC_API_BASE_URL.
// Falls back to the service crate's default (8080). We read via
// `import.meta.env` (Vite build-time injection) rather than SvelteKit's
// `$env/dynamic/public` so this module loads cleanly under vitest, which
// doesn't run the SvelteKit Vite plugin.
// Narrow `import.meta` to expose the optional `env` bag. We avoid the
// non-null assertion so the module also loads when `env` is absent
// (e.g. under vitest, where the SvelteKit Vite plugin is not active).
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };
/**
 * Base URL of the Person Service REST API that every {@link ApiClient}
 * targets.
 *
 * Resolved at build time from the `PUBLIC_API_BASE_URL` Vite env var so a
 * single build can be pointed at dev/staging/prod. Falls back to the
 * service crate's default local port (8080) when the var is unset — which
 * is also the value seen under vitest.
 */
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
