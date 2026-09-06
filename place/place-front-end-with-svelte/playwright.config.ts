import { defineConfig, devices } from "@playwright/test";
import { AUTH_STUB_PORT } from "./tests/e2e/auth-stub-port";

// The smoke suite's "signed-in" state (T-25). PRO-H10 put every mutation
// page (`/places/new`, `/places/[id]/edit`, `/places/merge`, `/review`)
// behind `requireSignedIn`, which 303s an anonymous visitor to `/signin` —
// so a smoke test that asserts a form heading needs a session cookie to be
// present. The guard is presence-only (`hooks.server.ts` copies the
// cookie into `locals.sessionId`; nothing validates it against the auth
// service — `AGENTS.md`, "Page-visit guard"), so a stub value is enough:
// this exercises the real guard code path with a real cookie rather than
// weakening the guard or bypassing it with an env flag. The value is never
// sent anywhere — the smoke suite has no service behind it.
// `tests/e2e/places.spec.ts` pins the anonymous redirect separately with
// an empty storageState, so the guard itself stays tested.
export const SMOKE_STORAGE_STATE = {
  cookies: [
    {
      name: "__Host-mxi_session",
      value: "smoke-stub-session",
      domain: "localhost",
      path: "/",
      expires: -1,
      httpOnly: true,
      secure: true,
      sameSite: "Lax" as const,
    },
  ],
  origins: [],
};

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
      use: { ...devices["Desktop Chrome"], storageState: SMOKE_STORAGE_STATE },
    },
  ],
});
