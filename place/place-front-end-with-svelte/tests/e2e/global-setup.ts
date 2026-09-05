// Starts the auth stub server (T-26) before Playwright's own `webServer`
// boots, since the preview server reads `AUTH_API_URL` once at process
// startup (`$env/dynamic/private`) — the stub must already be listening
// on the fixed port `playwright.config.ts` points `AUTH_API_URL` at.

import { AUTH_STUB_PORT } from "./auth-stub-port";
import { startAuthStub } from "./auth-stub-server";

export default async function globalSetup(): Promise<void> {
  await startAuthStub(AUTH_STUB_PORT);
  // No explicit teardown: the stub is a plain `http.Server` inside this
  // same Playwright process, so it exits naturally with the process
  // rather than needing a `globalTeardown` to close it.
}
