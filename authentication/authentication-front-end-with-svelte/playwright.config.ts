import { defineConfig, devices } from "@playwright/test";

// Smoke tests run against the production build served by `vite preview`
// (which, via SvelteKit's vite plugin, also runs the real SSR/server
// routes — +page.server.ts/+layout.server.ts load functions and actions
// execute for real, not just the client bundle).
//
// The auth-service calls this app makes all happen server-side, from the
// BFF's own `fetch` (src/lib/server/auth.ts / admin.ts) — never from the
// browser. `page.route()` only intercepts browser-issued requests, so it
// cannot stub those calls (spec §11/§13). Instead, `AUTH_API_URL` is
// pointed at a small Node-level mock auth server
// (tests/e2e/mock-auth-server.mjs), started as a second `webServer`
// alongside the app; no running Rust service is required.
const MOCK_AUTH_PORT = 5199;
const MOCK_AUTH_URL = `http://localhost:${MOCK_AUTH_PORT}`;

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  use: {
    baseURL: "http://localhost:4173",
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: [
    {
      // The stub the BFF's server-side fetch calls actually reach.
      command: "node tests/e2e/mock-auth-server.mjs",
      url: `${MOCK_AUTH_URL}/__mock/health`,
      reuseExistingServer: !process.env.CI,
      timeout: 30_000,
      env: { MOCK_AUTH_PORT: String(MOCK_AUTH_PORT) },
    },
    {
      // Test the production build via `vite preview`: it serves static,
      // correctly-typed ES modules, avoiding the `vite dev` cold-start
      // dependency-optimisation race that flakes module loading.
      command: "npm run build && npm run preview -- --port 4173 --strictPort",
      url: "http://localhost:4173",
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      env: { AUTH_API_URL: MOCK_AUTH_URL },
    },
  ],
});
