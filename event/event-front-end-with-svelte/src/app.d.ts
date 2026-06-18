// See https://svelte.dev/docs/kit/types#app
// SvelteKit ambient app types. Augments the framework's `App` namespace;
// purely type-level (no runtime output).
declare global {
    namespace App {
        // Shape of errors surfaced through SvelteKit error pages; extended
        // with the service's structured `code`/`details` (mirrors ApiError).
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
