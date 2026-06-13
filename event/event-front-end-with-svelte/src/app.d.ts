// See https://svelte.dev/docs/kit/types#app
declare global {
    namespace App {
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
