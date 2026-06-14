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
        // Empty by design — no server-side locals, page data, page state,
        // or platform bindings in this SPA-only front-end.
        interface Locals {}
        interface PageData {}
        interface PageState {}
        interface Platform {}
    }
}

export {};
