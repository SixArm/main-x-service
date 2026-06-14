// REST API base URL. Configured via PUBLIC_API_BASE_URL. Falls back to
// the service crate's default (8080). We read via `import.meta.env`
// (Vite build-time injection) rather than SvelteKit's `$env/dynamic/public`
// so this module loads cleanly under vitest, which doesn't run the
// SvelteKit Vite plugin.
// Narrowed view of `import.meta` exposing the optional Vite `env` bag.
// Typed defensively (env may be undefined) so the lookup below is safe
// under both the Vite build and the bare vitest environment.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

/**
 * Absolute base URL of the Event Service REST API.
 *
 * Sourced from the `PUBLIC_API_BASE_URL` Vite env var at build time; when
 * unset (e.g. local dev, tests) it falls back to the service crate's
 * default listen address on port 8080. All {@link ApiClient} requests are
 * resolved relative to this value.
 */
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
