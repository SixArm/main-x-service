import { defineConfig, devices } from "@playwright/test";
import { AUTH_STUB_PORT } from "./tests/e2e/auth-stub-port";

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: "list",
  // Starts the auth stub server (T-26) before the webServer below boots,
  // since AUTH_API_URL is read once at the preview server's startup.
  globalSetup: "./tests/e2e/global-setup.ts",
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:4173",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npm run build && npm run preview -- --port 4173",
    port: 4173,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: { AUTH_API_URL: `http://127.0.0.1:${AUTH_STUB_PORT}` },
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
