// REST API base URL. Configured via PUBLIC_API_BASE_URL. Falls back to
// the service crate's default (8080). We read via `import.meta.env`
// (Vite build-time injection) rather than SvelteKit's `$env/dynamic/public`
// so this module loads cleanly under vitest, which doesn't run the
// SvelteKit Vite plugin.
// Narrow `import.meta` to expose the optional Vite `env` bag without
// asserting it always exists (vitest runs this module with no Vite env).
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };
/**
 * Base URL of the Place Service REST API, resolved once at module load.
 *
 * Read from the Vite build-time variable `PUBLIC_API_BASE_URL` so the
 * value is baked into the bundle; falls back to the service crate's
 * local default (port 8080) when the variable is unset (e.g. in tests).
 */
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
