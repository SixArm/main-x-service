// Server-side (BFF) configuration. Never imported by browser code.

import { env } from "$env/dynamic/private";

/** Base URL of the human-capital-management service the BFF proxies to. */
export const HCM_API_URL: string =
  env.HCM_API_URL ?? "http://localhost:5150";

/** Authentication service base URL — for the session→PASETO exchange
 *  and the magic-link login flow (PF-T18). */
export const AUTH_API_URL: string = env.AUTH_API_URL ?? "http://localhost:5150";
