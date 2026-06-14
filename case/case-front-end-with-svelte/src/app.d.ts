// See https://svelte.dev/docs/kit/types#app
declare global {
    namespace App {
        // Augments SvelteKit's error shape with optional app-specific fields
        // (a machine `code` and free-form `details`) surfaced on error pages.
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
