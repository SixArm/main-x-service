import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
    preprocess: vitePreprocess(),
    kit: {
        adapter: adapter()
        // Lily helpers (theme-picker / locale-picker) are declared as `file:`
        // dependencies in package.json and imported by their package names, so
        // no kit.alias is needed (and `npm install` fails loudly if the sibling
        // design-system repo is missing, rather than a confusing build error).
    }
};

export default config;
