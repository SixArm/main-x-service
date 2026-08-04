// Server-side (BFF) configuration. Never imported by browser code.

import { env } from "$env/dynamic/private";

/** Base URL of the workforce-planning-management service the BFF proxies to. */
export const WPM_API_URL: string = env.WPM_API_URL ?? "http://localhost:5150";

/** Authentication service base URL — for the session→PASETO exchange
 *  and the magic-link login flow (WPM-T18). */
export const AUTH_API_URL: string = env.AUTH_API_URL ?? "http://localhost:5150";
