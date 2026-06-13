import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const LILY_HELPERS =
    '../../../../lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers';

/** @type {import('@sveltejs/kit').Config} */
const config = {
    preprocess: vitePreprocess(),
    kit: {
        adapter: adapter(),
        alias: {
            '@lily/locale-picker': `${LILY_HELPERS}/lily-design-system-svelte-locale-picker`,
            '@lily/theme-picker': `${LILY_HELPERS}/lily-design-system-svelte-theme-picker`
        }
    }
};

export default config;
