// See https://svelte.dev/docs/kit/types#app
declare global {
  namespace App {
    // Augments SvelteKit's error shape with optional app-specific fields
    // (a machine `code` and free-form `details`) surfaced on error pages.
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
    interface PageData {}
    interface PageState {}
    interface Platform {}
  }
}

export {};
