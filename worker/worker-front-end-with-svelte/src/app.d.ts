// See https://svelte.dev/docs/kit/types#app
// Ambient SvelteKit `App.*` interface augmentations for this project.
declare global {
  namespace App {
    /**
     * Shape of errors surfaced through SvelteKit's error handling.
     * Extended with `code`/`details` so {@link ApiError} fields can ride
     * along to error pages.
     */
    interface Error {
      /** Service error code, when the error originated from the API. */
      code?: string;
      /** Endpoint-specific extra detail. */
      details?: unknown;
    }
    /**
     * Per-request server locals. BFF: the server holds the opaque
     * session id from the httpOnly `__Host-mxi_session` cookie (set in
     * `hooks.server.ts`); the browser never reads it.
     */
    interface Locals {
      sessionId: string | null;
    }
    /**
     * Shared page load data. `title` follows the `page.data.title`
     * convention (see `routes/+layout.svelte`): every route's own load
     * function returns a plain `title` string mirroring what that
     * route's `<svelte:head><title>` renders, so the layout — which
     * does not know which leaf page is active — can read it for
     * SharePicker without scraping `document.title`. Optional because
     * a route may not set one (falls back to the brand name) and
     * because SvelteKit's own internal error/data states don't carry
     * it either.
     */
    interface PageData {
      title?: string;
    }
    /** Ephemeral page state for shallow routing (unused). */
    interface PageState {}
    /** Adapter platform context (unused). */
    interface Platform {}
  }
}

export {};
