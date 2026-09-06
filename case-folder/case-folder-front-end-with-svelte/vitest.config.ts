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
        alias: {
            $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
            // The plain svelte plugin resolves neither SvelteKit's `$app/*`
            // virtual modules nor the Lily helper packages, so point them at
            // test stubs (used only when a test imports a route/layout).
            '$app/state': fileURLToPath(new URL('./src/lib/test-support/app-state.ts', import.meta.url)),
            '$app/navigation': fileURLToPath(new URL('./src/lib/test-support/app-navigation.ts', import.meta.url)),
            '$app/environment': fileURLToPath(new URL('./src/lib/test-support/app-environment.ts', import.meta.url)),
            'lily-design-system-svelte-theme-picker': fileURLToPath(new URL('./src/lib/test-support/StubComponent.svelte', import.meta.url)),
            // The real share-picker / text-size-picker packages export their
            // component as a *named* export (SharePicker / TextSizePicker),
            // unlike theme-picker's default export — so these two route
            // through a thin re-export shim rather than StubComponent.svelte
            // directly.
            'lily-design-system-svelte-share-picker': fileURLToPath(new URL('./src/lib/test-support/StubSharePicker.ts', import.meta.url)),
            'lily-design-system-svelte-text-size-picker': fileURLToPath(new URL('./src/lib/test-support/StubTextSizePicker.ts', import.meta.url))
        },
        ...(process.env.VITEST ? { conditions: ['browser'] } : {})
    }
});
