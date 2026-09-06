// Shared fixed port for the auth stub (T-23): both `global-setup.ts` (which
// starts the listener) and `playwright.config.ts` (which points the built
// preview server's `AUTH_API_URL` at it) need the identical value.
export const AUTH_STUB_PORT = 4174;
