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
        // Locals/PageData/PageState/Platform are intentionally empty — this
        // SPA carries no server-side request locals or typed page data.
        interface Locals {}
        interface PageData {}
        interface PageState {}
        interface Platform {}
    }
}

export {};
