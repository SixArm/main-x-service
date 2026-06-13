import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const lilyHelpers = resolve(
    fileURLToPath(new URL('.', import.meta.url)),
    '../../../lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers'
);

// Proxy the API to the Loco app so the browser sees a single origin and
// the HttpOnly session cookie is first-party (see spec/auth.md). Override
// the target with LOCO_API_PROXY.
const apiTarget = process.env.LOCO_API_PROXY || 'http://localhost:5150';

export default defineConfig({
    plugins: [sveltekit()],
    server: {
        fs: {
            allow: [lilyHelpers]
        },
        proxy: {
            '/api': { target: apiTarget, changeOrigin: true },
            '/healthz': { target: apiTarget, changeOrigin: true }
        }
    }
});
