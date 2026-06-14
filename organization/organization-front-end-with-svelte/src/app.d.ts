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
        interface Locals {}
        interface PageData {}
        interface PageState {}
        interface Platform {}
    }
}

export {};
