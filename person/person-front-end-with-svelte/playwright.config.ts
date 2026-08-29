import { defineConfig, devices } from "@playwright/test";

// Two test surfaces:
//
//   `smoke`        — page-shell rendering. No service required; uses
//                    the existing preview server. Run with
//                    `pnpm test:e2e`.
//
//   `integration`  — golden-path flows that exercise the live
//                    Rust Person Service (`person-service-with-loco`)
//                    over real HTTP, PLUS a real signed-in session
//                    (PRO-P32) established once by the `setup` project
//                    below against a live authentication-service.
//                    Requires:
//                      - person-service at PUBLIC_API_BASE_URL
//                        (default http://localhost:8080)
//                      - authentication-service, in DEVELOPMENT mode,
//                        at AUTH_API_URL (default http://localhost:5150
//                        — examples/compose/authentication-dev.yml;
//                        NOT full-family.yml, which pins production
//                        mode and never logs the magic link — see
//                        tests/integration/auth.setup.ts)
//                    Run with `pnpm test:integration` or `bin/e2e`
//                    (which also health-checks both services first).
//
// The integration project rebuilds the preview with PUBLIC_API_BASE_URL
// baked in (it's read via `import.meta.env` at build time), so changing
// the URL means a re-build of preview. PERSON_API_URL and AUTH_API_URL
// are the BFF's own *server-side* config (`src/lib/server/config.ts`,
// read at runtime, not baked into the client bundle) — the webServer
// command below sets PERSON_API_URL to the SAME base as
// PUBLIC_API_BASE_URL so mutating flows submitted through the UI (which
// go through the server-side proxy, not the direct API) land on the
// live service this suite actually started, rather than silently
// falling back to this crate's own .env / .env.example default
// (http://localhost:5150 — the native `cargo run` dev port, not the
// podman-compose container's 8080). Before this fix, only the
// direct-REST-fixture helpers in golden-paths.spec.ts (which read
// PUBLIC_API_BASE_URL themselves) were guaranteed to hit the right
// instance; every UI-submitted mutation was one `.env` away from a
// silent misroute — plausibly why this was never caught before PRO-H10/
// PRO-H5 made every mutating flow require sign-in anyway.

const PREVIEW_PORT = Number(process.env.PLAYWRIGHT_PREVIEW_PORT ?? 4173);
const API_BASE_URL = process.env.PUBLIC_API_BASE_URL ?? "http://localhost:8080";
const AUTH_API_URL = process.env.AUTH_API_URL ?? "http://localhost:5150";

export default defineConfig({
  timeout: 30_000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: "list",
  use: {
    baseURL:
      process.env.PLAYWRIGHT_BASE_URL ?? `http://localhost:${PREVIEW_PORT}`,
    trace: "retain-on-failure",
  },
  webServer: {
    command:
      `PUBLIC_API_BASE_URL=${API_BASE_URL} npm run build && ` +
      `PERSON_API_URL=${API_BASE_URL} AUTH_API_URL=${AUTH_API_URL} ` +
      `npm run preview -- --port ${PREVIEW_PORT}`,
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
      // Real magic-link sign-in (PRO-P32), run once; `integration`
      // depends on it and reuses the resulting cookies. Playwright's
      // own recommended pattern for "authenticate once, reuse
      // everywhere" — see tests/integration/auth.setup.ts for why
      // this is a project dependency rather than a top-level
      // globalSetup (which would also gate the deliberately
      // service-free `smoke` project).
      name: "setup",
      testDir: "tests/integration",
      testMatch: /auth\.setup\.ts/,
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "integration",
      testDir: "tests/integration",
      testIgnore: /auth\.setup\.ts/,
      dependencies: ["setup"],
      use: {
        ...devices["Desktop Chrome"],
        // The real session + CSRF cookies `setup` established.
        storageState: "tests/integration/.auth-storage-state.json",
        // Surface the API URL to specs that hit the service
        // directly for setup / cleanup via Playwright's
        // request fixture.
        extraHTTPHeaders: { "x-mxi-test-source": "playwright-integration" },
      },
    },
  ],
});
