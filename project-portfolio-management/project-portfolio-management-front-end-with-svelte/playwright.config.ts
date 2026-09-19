import { defineConfig, devices } from "@playwright/test";

// Smoke tests run against the Vite dev server. The backend is stubbed
// per-test via `page.route`, so no running Rust service is required.
export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  use: {
    baseURL: "http://localhost:4173",
    trace: "on-first-retry",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] }, testIgnore: /mobile\.spec\.ts/ },
    // T-28k (repo tasks.md EV-1): a phone-width audit across every
    // route — 390x844 per the task's own spec (roughly an iPhone 12
    // mini). A separate project rather than a per-test viewport
    // override, so `pnpm test:e2e --project=mobile` runs it in
    // isolation, matching the acceptance text's "runs the mobile
    // project in CI".
    {
      name: "mobile",
      testMatch: /mobile\.spec\.ts/,
      use: { ...devices["Desktop Chrome"], viewport: { width: 390, height: 844 } },
    },
  ],
  // Test the production build via `vite preview`: it serves static,
  // correctly-typed ES modules, avoiding the `vite dev` cold-start
  // dependency-optimisation race that flakes module loading.
  webServer: {
    command: "npm run build && npm run preview -- --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
