import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

// Unit + component tests. Pure-logic tests (nhs.ts, client mappers) and
// component tests (@testing-library/svelte) both run under jsdom; the svelte
// plugin compiles .svelte imports. We provide the `$lib` alias ourselves
// (the plain svelte plugin, unlike SvelteKit, doesn't), and the browser
// resolve condition is guarded by VITEST so it can't affect the app build.
export default defineConfig({
    plugins: [svelte({ hot: false })],
    test: {
        environment: 'jsdom',
        include: ['src/**/*.test.ts'],
        setupFiles: ['./vitest-setup.ts']
    },
    resolve: {
        alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) },
        ...(process.env.VITEST ? { conditions: ['browser'] } : {})
    }
});
