import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
    // `svelteTesting()` selects Svelte's browser/client resolve condition so
    // components mount client-side under jsdom (rather than SSR), and wires
    // testing-library auto-cleanup. It only affects the vitest run.
    plugins: [sveltekit(), svelteTesting()],
    server: {
        port: 5173,
        strictPort: false,
    },
    test: {
        include: ["tests/unit/**/*.{test,spec}.{ts,js}"],
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
        globals: true,
    },
});
