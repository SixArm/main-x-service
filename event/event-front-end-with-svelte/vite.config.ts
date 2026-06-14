// Vite + Vitest config. Uses vitest/config so the SvelteKit plugin and the
// `test` block coexist in one file.
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
    plugins: [sveltekit()],
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
        // Expose describe/it/expect globally (no per-file import needed).
        globals: true,
    },
});
