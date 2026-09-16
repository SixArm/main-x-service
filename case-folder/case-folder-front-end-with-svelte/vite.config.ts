import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Proxy the API to the Loco app so the browser sees a single origin and
// the HttpOnly session cookie is first-party (see spec/auth.md). Override
// the target with LOCO_API_PROXY.
const apiTarget = process.env.LOCO_API_PROXY || 'http://localhost:5150';

export default defineConfig({
    plugins: [sveltekit()],
    server: {
        proxy: {
            '/api': { target: apiTarget, changeOrigin: true },
            '/healthz': { target: apiTarget, changeOrigin: true }
        }
    }
});
