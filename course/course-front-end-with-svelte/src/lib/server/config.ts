// Server-side BFF endpoints (never bundled into the browser).

import { env } from "$env/dynamic/private";

/** Course service base URL — the proxy forwards entity-API calls here
 *  with a server-injected PASETO. course-service-with-loco is the one
 *  service in the family whose own dev config overrides the generic loco
 *  default (5150) to 8084, so the fallback here matches that rather than
 *  the family default (T-28: fixed 2026-08-29 — the old 5150 fallback
 *  silently routed an unconfigured dev environment to whatever else was
 *  listening on the shared port). */
export const COURSE_API_URL = env.COURSE_API_URL ?? "http://localhost:8084";

/** Authentication service base URL — for the session→PASETO exchange and
 *  the magic-link login flow. */
export const AUTH_API_URL = env.AUTH_API_URL ?? "http://localhost:5150";
