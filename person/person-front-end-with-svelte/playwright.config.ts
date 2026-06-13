import { defineConfig, devices } from "@playwright/test";

// Two test surfaces:
//
//   `smoke`        — page-shell rendering. No service required; uses
//                    the existing preview server. Run with
//                    `pnpm test:e2e`.
//
//   `integration`  — golden-path flows that exercise the live
//                    Rust Person Service (`person-service-rust-crate`)
//                    over real HTTP. Requires the service to be
//                    running at PUBLIC_API_BASE_URL (default
//                    http://localhost:8080). Run with
//                    `pnpm test:integration` (which sets the env
//                    var) or `bin/e2e` (which also health-checks the
//                    service first).
//
// The integration project rebuilds the preview with
// PUBLIC_API_BASE_URL baked in (it's read via `import.meta.env` at
// build time), so changing the URL means a re-build of preview.

const PREVIEW_PORT = Number(process.env.PLAYWRIGHT_PREVIEW_PORT ?? 4173);
const API_BASE_URL = process.env.PUBLIC_API_BASE_URL ?? "http://localhost:8080";

export default defineConfig({
    timeout: 30_000,
    fullyParallel: false,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 1 : 0,
    reporter: "list",
    use: {
        baseURL: process.env.PLAYWRIGHT_BASE_URL ?? `http://localhost:${PREVIEW_PORT}`,
        trace: "retain-on-failure",
    },
    webServer: {
        command: `PUBLIC_API_BASE_URL=${API_BASE_URL} npm run build && npm run preview -- --port ${PREVIEW_PORT}`,
        port: PREVIEW_PORT,
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
    },
    projects: [
        {
            name: "smoke",
            testDir: "tests/e2e",
            use: { ...devices["Desktop Chrome"] },
        },
        {
            name: "integration",
            testDir: "tests/integration",
            use: {
                ...devices["Desktop Chrome"],
                // Surface the API URL to specs that hit the service
                // directly for setup / cleanup via Playwright's
                // request fixture.
                extraHTTPHeaders: { "x-mxi-test-source": "playwright-integration" },
            },
        },
    ],
});
