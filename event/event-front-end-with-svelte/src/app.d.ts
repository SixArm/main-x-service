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
        interface Locals {}
        interface PageData {}
        interface PageState {}
        interface Platform {}
    }
}

export {};
