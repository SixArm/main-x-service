## 10. Persistence

The front-end is stateless. No local DB, no client-side cache layer beyond Svelte component state. Page reloads re-fetch from the service.

(Roadmap: introduce SvelteKit `+page.ts` load functions with `event.fetch` for SSR hydration — T-13.)

