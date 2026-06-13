import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
    testDir: "tests/e2e",
    timeout: 30_000,
    fullyParallel: true,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 1 : 0,
    reporter: "list",
    use: {
        baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:4173",
        trace: "retain-on-failure",
    },
    webServer: {
        command: "npm run build && npm run preview -- --port 4173",
        port: 4173,
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
