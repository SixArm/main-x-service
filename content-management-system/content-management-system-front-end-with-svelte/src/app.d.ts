// See https://svelte.dev/docs/kit/types#app
declare global {
  namespace App {
    // Augments SvelteKit's error shape with optional app-specific
    // fields (a machine `code` and free-form `details`) surfaced on
    // error pages.
    interface Error {
      code?: string;
      details?: unknown;
    }
    // BFF: the server holds the opaque session id from the httpOnly
    // `__Host-mxi_session` cookie (set in `hooks.server.ts`); the
    // browser never reads it.
    interface Locals {
      sessionId: string | null;
    }
    // `page.data.title` convention (see `routes/+layout.svelte`): every
    // route's load function returns a `title` mirroring its own
    // <svelte:head><title>, which the layout reads for SharePicker.
    // Optional because a route may not set one (falls back to the brand
    // name) and because SvelteKit's own internal error/data states don't
    // carry it either.
    interface PageData {
      title?: string;
    }
    interface PageState {}
    interface Platform {}
  }
}

export {};
