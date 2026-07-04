// Server-side BFF endpoints (never bundled into the browser).

import { env } from "$env/dynamic/private";

/** Care Pathway service base URL — the proxy forwards entity-API calls
 *  here with a server-injected PASETO. (loco dev default 5150.) */
export const CARE_PATHWAY_API_URL =
  env.CARE_PATHWAY_API_URL ?? "http://localhost:5150";

/** Authentication service base URL — for the session→PASETO exchange and
 *  the magic-link login flow. */
export const AUTH_API_URL = env.AUTH_API_URL ?? "http://localhost:5150";
