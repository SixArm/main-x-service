// See https://svelte.dev/docs/kit/types#app
declare global {
    namespace App {
        // `page.data.title` convention (see `routes/+layout.svelte`): every
        // route's load function returns a `title` mirroring what that
        // route renders as its own heading, which the layout reads for
        // SharePicker. Optional because a route may not set one (falls
        // back to the brand name) and because SvelteKit's own internal
        // data states don't carry it either.
        interface PageData {
            title?: string;
        }
    }
}

export {};
