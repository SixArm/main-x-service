// Playwright e2e config: builds + previews the app, then runs the smoke
// specs against it in headless Chromium. CI tightens behavior (no .only,
// one retry).
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
    testDir: "tests/e2e",
    timeout: 30_000,
    fullyParallel: true,
    // Fail the run if a test is left focused with .only (guards CI).
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 1 : 0,
    reporter: "list",
    use: {
        // Target the preview server (overridable for an external deployment).
        baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:4173",
        // Keep a trace only when a test fails, for debugging.
        trace: "retain-on-failure",
    },
    webServer: {
        // Build then serve the production preview the tests run against.
        command: "npm run build && npm run preview -- --port 4173",
        port: 4173,
        // Locally reuse a running server; always start fresh in CI.
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
    },
    projects: [
        {
            name: "chromium",
            use: { ...devices["Desktop Chrome"] },
        },
    ],
});
