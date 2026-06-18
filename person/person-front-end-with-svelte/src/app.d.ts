// See https://svelte.dev/docs/kit/types#app
// SvelteKit ambient type augmentations for this app.
declare global {
    namespace App {
        /**
         * Shape of errors thrown to SvelteKit's error boundary. Extended
         * with `code`/`details` so {@link ApiError} fields survive into
         * the framework's error page.
         */
        interface Error {
            /** Machine-readable error code (mirrors ApiError.code). */
            code?: string;
            /** Structured error details (mirrors ApiError.details). */
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
