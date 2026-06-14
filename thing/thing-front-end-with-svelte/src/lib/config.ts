// REST API base URL. Configured via PUBLIC_API_BASE_URL. Falls back to
// the service crate's default (8080). We read via `import.meta.env`
// (Vite build-time injection) rather than SvelteKit's `$env/dynamic/public`
// so this module loads cleanly under vitest, which doesn't run the
// SvelteKit Vite plugin.
// Cast import.meta to expose `env` with an index signature; the standard
// ImportMeta type doesn't declare arbitrary PUBLIC_* keys.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

/**
 * Base URL of the Thing Service REST API.
 *
 * Read from the build-time `PUBLIC_API_BASE_URL` env var (injected by Vite),
 * falling back to the service crate's local default port (8080). See the
 * note above on why `import.meta.env` is used instead of SvelteKit's
 * `$env/dynamic/public` (vitest compatibility).
 */
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
