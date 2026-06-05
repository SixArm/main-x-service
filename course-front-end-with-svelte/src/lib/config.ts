// REST API base URL. Configured via PUBLIC_API_BASE_URL. Falls back
// to the Course Service's docker-compose host port (8084). 8080 is
// the person-service slot in the Main X Index family — defaulting
// to 8080 silently routed every request to the wrong service in
// developer setups. We read via `import.meta.env` (Vite build-time
// injection) rather than SvelteKit's `$env/dynamic/public` so this
// module loads cleanly under vitest, which doesn't run the
// SvelteKit Vite plugin.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };
export const API_BASE_URL: string = meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:8084";
