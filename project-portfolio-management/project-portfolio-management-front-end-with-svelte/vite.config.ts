import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
    // `svelteTesting()` selects Svelte's browser/client resolve condition so
    // components mount client-side under jsdom (rather than SSR), and wires
    // testing-library auto-cleanup. It only affects the vitest run.
    plugins: [
        sveltekit({
            preprocess: vitePreprocess(),
            adapter: adapter(),
            alias: { $lib: "src/lib" }
        }),
        svelteTesting()
    ],
    server: { port: 5173, strictPort: false },
    test: {
        include: ["tests/unit/**/*.{test,spec}.{ts,js}"],
        environment: "jsdom",
        globals: true,
    },
});
