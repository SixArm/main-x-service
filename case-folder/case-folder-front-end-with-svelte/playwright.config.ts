import { defineConfig, devices } from '@playwright/test';

// Playwright e2e config.
//
// The tests assume:
//   1. The Loco JSON API is running on http://localhost:5150
//      (or whatever LOCO_API_URL points at).
//   2. The seed task has been run at least once so the database has
//      the known demo data (`cargo run -- task seed` from the
//      `../case-folder-service-with-rust/` directory).
//
// Playwright boots the Svelte dev server itself via `webServer`.
// `global-setup.ts` pings /healthz and verifies seed state before
// any test runs, so a misconfigured environment fails fast with a
// helpful message instead of a wall of stale assertions.

const PORT = Number(process.env.PORT ?? 5173);
const BASE_URL = process.env.BASE_URL ?? `http://localhost:${PORT}`;
const LOCO_API_URL = process.env.LOCO_API_URL ?? 'http://localhost:5150';

// Saved authenticated browser state. `global-setup.ts` signs in once and
// writes it here; every spec reuses it via `use.storageState`. The auth
// spec opts out with an empty storageState to test the sign-in flow.
const AUTH_FILE = 'tests/e2e/.auth/state.json';

export default defineConfig({
    testDir: 'tests/e2e',
    timeout: 30_000,
    expect: { timeout: 5_000 },
    fullyParallel: false, // tests share the upstream Loco state — serialise.
    workers: 1,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 1 : 0,
    reporter: process.env.CI ? [['github'], ['list']] : 'list',
    use: {
        baseURL: BASE_URL,
        storageState: AUTH_FILE,
        trace: 'on-first-retry',
        screenshot: 'only-on-failure',
        actionTimeout: 5_000
    },
    projects: [
        {
            name: 'chromium',
            use: { ...devices['Desktop Chrome'] }
        }
    ],
    webServer: {
        command: 'npm run dev',
        url: BASE_URL,
        reuseExistingServer: !process.env.CI,
        timeout: 60_000,
        // Same-origin: the client uses relative /api, the dev server
        // proxies it to the Loco API so the session cookie is first-party.
        env: { VITE_API_BASE_URL: '', LOCO_API_PROXY: LOCO_API_URL }
    },
    globalSetup: './tests/e2e/global-setup.ts'
});

export { AUTH_FILE, BASE_URL, LOCO_API_URL };
