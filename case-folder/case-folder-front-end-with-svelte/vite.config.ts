import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Proxy the API to the case-folder service so the browser sees a single origin and
// the HttpOnly session cookie is first-party (see spec/auth.md). Override
// the target with CASE_FOLDER_API_URL.
const apiTarget = process.env.CASE_FOLDER_API_URL || 'http://localhost:5150';

export default defineConfig({
    plugins: [
        sveltekit({ preprocess: vitePreprocess(), adapter: adapter() })
    ],
    server: {
        proxy: {
            '/api': { target: apiTarget, changeOrigin: true },
            '/healthz': { target: apiTarget, changeOrigin: true }
        }
    }
});
