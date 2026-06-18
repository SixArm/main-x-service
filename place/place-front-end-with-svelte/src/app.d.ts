// See https://svelte.dev/docs/kit/types#app
// Ambient SvelteKit `App` namespace augmentation. The `Error` shape is
// extended so thrown app errors can carry the service's `code`/`details`
// alongside SvelteKit's default `message`.
declare global {
    namespace App {
        /** App-level error shape; mirrors the service's ApiErrorBody extras. */
        interface Error {
            /** Machine-readable error code from the service. */
            code?: string;
            /** Optional structured error details. */
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
