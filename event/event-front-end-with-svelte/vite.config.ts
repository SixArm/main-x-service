// Vite + Vitest config. Uses vitest/config so the SvelteKit plugin and the
// `test` block coexist in one file.
import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
    plugins: [sveltekit(), svelteTesting()],
    server: {
        // Dev server port; strictPort=false lets Vite pick the next free port.
        port: 5173,
        strictPort: false,
    },
    test: {
        // Only the unit suite runs under vitest; e2e specs are Playwright's.
        include: ["tests/unit/**/*.{test,spec}.{ts,js}"],
        // jsdom gives a DOM for component/browser-API code under test.
        environment: "jsdom",
        // https, not the jsdom default http://localhost:3000: the
        // `__Host-mxi_csrf` cookie carries the `__Host-` prefix, which a
        // browser (and jsdom, faithfully) only accepts from a secure
        // origin. Without this, `document.cookie = "__Host-…"` silently
        // no-ops in tests/unit/client.test.ts's CSRF-header suite.
        environmentOptions: {
            jsdom: {
                url: "https://localhost:5173",
            },
        },
        // Expose describe/it/expect globally (no per-file import needed).
        globals: true,
    },
});
