// SvelteKit ambient type declarations for the `App` namespace.
// See https://svelte.dev/docs/kit/types#app
declare global {
    namespace App {
        // Shape of errors surfaced to error boundaries; extended with
        // the service's `code`/`details` so handle-error pages can read them.
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
