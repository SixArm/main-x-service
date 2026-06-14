// SvelteKit config: TS/PostCSS preprocessing, an auto-detected deploy
// adapter, and the $lib import alias.
import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
    // Enable <script lang="ts"> and other vite preprocessing.
    preprocess: vitePreprocess(),
    kit: {
        // adapter-auto picks the right build target for the host platform.
        adapter: adapter(),
        alias: {
            // `$lib` → src/lib for clean absolute-ish imports.
            $lib: "src/lib",
        },
    },
};

export default config;
