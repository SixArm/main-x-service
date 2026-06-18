// SvelteKit ambient type augmentations for the `App` namespace.
// See https://svelte.dev/docs/kit/types#app
declare global {
    namespace App {
        // App-specific error shape: optional `code` + `details` carried on
        // thrown SvelteKit errors (defaults otherwise).
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
