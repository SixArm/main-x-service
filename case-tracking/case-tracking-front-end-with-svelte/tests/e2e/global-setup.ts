// Pre-flight + sign-in for the e2e suite.
//
// 1. Verify the Loco API is reachable through the dev-server proxy.
// 2. Sign in once with the demo allowlist identity (magic link →
//    verify) so we hold a real session cookie.
// 3. Verify the API is seeded with demo data.
// 4. Save the authenticated browser state to AUTH_FILE; every spec
//    reuses it via `use.storageState`, so the suites run signed in
//    with no per-test login. `auth.spec.ts` opts out to test the flow.

import { request } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';

const PORT = Number(process.env.PORT ?? 5173);
const BASE_URL = process.env.BASE_URL ?? `http://localhost:${PORT}`;
const AUTH_FILE = 'tests/e2e/.auth/state.json';
const LOGIN_EMAIL = process.env.E2E_LOGIN_EMAIL ?? 'records@example.nhs.uk';

function fail(message: string): never {
    const banner = '═'.repeat(70);
    throw new Error(
        `\n${banner}\n` +
            `Playwright pre-flight failed.\n\n` +
            `${message}\n\n` +
            `Make sure the Loco JSON API is running in stub mode (it seeds\n` +
            `demo data and the auth allowlist incl. ${LOGIN_EMAIL}):\n` +
            `  cd ../case-tracker-service-with-rust\n` +
            `  USE_UPSTREAM_STUBS=1 cargo run -- start     # listens on :5150\n\n` +
            `Then re-run \`npm run test:e2e\`.\n` +
            `${banner}\n`
    );
}

export default async function globalSetup() {
    const ctx = await request.newContext({ baseURL: BASE_URL });

    // 1. API reachable (through the dev-server proxy at /healthz).
    try {
        const r = await ctx.get('/healthz');
        if (!r.ok()) fail(`/healthz → HTTP ${r.status()}`);
        const health = (await r.json()) as { status?: string };
        if (health.status !== 'ok') fail(`/healthz returned ${JSON.stringify(health)}`);
    } catch (e) {
        fail(`Cannot reach the API via ${BASE_URL}/healthz: ${(e as Error).message}`);
    }

    // 2. Sign in: request a magic link, then verify it for a session.
    const requested = await ctx.post('/api/auth/request', { data: { email: LOGIN_EMAIL } });
    if (!requested.ok()) fail(`/api/auth/request → HTTP ${requested.status()}`);
    const requestedBody = (await requested.json()) as { magic_link?: string | null };
    const link = requestedBody.magic_link ?? null;
    if (!link) {
        fail(
            `No magic link returned for ${LOGIN_EMAIL}. The email must be on the ` +
                `allowlist and auth.expose_magic_link must be on (dev/stub mode).`
        );
    }
    const token = new URL(link).searchParams.get('token');
    if (!token) fail(`Magic link had no token: ${link}`);
    const verified = await ctx.post('/api/auth/verify', { data: { token } });
    if (!verified.ok()) fail(`/api/auth/verify → HTTP ${verified.status()}`);

    // 3. Now authenticated — verify the demo data is present.
    const statsRes = await ctx.get('/api/stats');
    if (!statsRes.ok()) fail(`/api/stats → HTTP ${statsRes.status()} (after sign-in)`);
    const stats = (await statsRes.json()) as {
        patients: number;
        folders: { total: number };
        places: { cabinets: number };
    };
    if (stats.folders.total === 0 || stats.places.cabinets === 0 || stats.patients === 0) {
        fail(
            `The API is up but appears to have no demo data.\n` +
                `Found: ${stats.patients} patients, ${stats.folders.total} folders, ` +
                `${stats.places.cabinets} cabinets.`
        );
    }

    // 4. Persist the authenticated state for every spec.
    await mkdir(dirname(AUTH_FILE), { recursive: true });
    await ctx.storageState({ path: AUTH_FILE });
    await ctx.dispose();
}
