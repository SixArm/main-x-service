// REST API base URL. Configured via PUBLIC_API_BASE_URL. Falls back to
// the service crate's default (8080). We read via `import.meta.env`
// (Vite build-time injection) rather than SvelteKit's `$env/dynamic/public`
// so this module loads cleanly under vitest, which doesn't run the
// SvelteKit Vite plugin.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
