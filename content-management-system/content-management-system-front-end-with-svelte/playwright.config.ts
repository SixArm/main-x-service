import { defineConfig, devices } from "@playwright/test";

// Smoke tests run against the Vite dev server. The backend is stubbed
// per-test via `page.route`, so no running Rust service is required.
export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  use: {
    baseURL: "http://localhost:4183",
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  // Test the production build via `vite preview`: it serves static,
  // correctly-typed ES modules, avoiding the `vite dev` cold-start
  // dependency-optimisation race that flakes module loading.
  webServer: {
    command: "pnpm build && pnpm preview --port 4183 --strictPort",
    url: "http://localhost:4183",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
