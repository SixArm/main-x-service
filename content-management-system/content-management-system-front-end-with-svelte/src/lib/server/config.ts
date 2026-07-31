// Server-side (BFF) configuration. Never imported by browser code.

import { env } from "$env/dynamic/private";

/** Base URL of the content-management-system service the BFF proxies to. */
export const CMS_API_URL: string = env.CMS_API_URL ?? "http://localhost:5150";

/** Authentication service base URL — for the session→PASETO exchange
 *  and the magic-link login flow. */
export const AUTH_API_URL: string = env.AUTH_API_URL ?? "http://localhost:5150";
