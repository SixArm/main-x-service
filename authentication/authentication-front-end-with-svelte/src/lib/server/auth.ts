// BFF server-side calls to the authentication service (never imported by
// browser code). The SvelteKit server holds the session cookie and does
// all token handling here; the browser never sees an access token.

import { env } from "$env/dynamic/private";
import { SESSION_COOKIE } from "./session";
import type { CurrentUser } from "$lib/api/types";

/** Authentication-service base URL (server-side; loco dev default 5150). */
const AUTH_API_URL = env.AUTH_API_URL ?? "http://localhost:5150";

type FetchFn = typeof fetch;

/** Consume a magic-link token; returns the raw upstream response so the
 *  caller can read the `Set-Cookie` that establishes the session. */
export function verifyMagicLink(
  fetchFn: FetchFn,
  token: string,
): Promise<Response> {
  return fetchFn(
    `${AUTH_API_URL}/api/auth/magic-link/${encodeURIComponent(token)}`,
  );
}

/** Request a magic link for an existing account (sign in). */
export async function requestMagicLink(
  fetchFn: FetchFn,
  email: string,
  locale?: string,
): Promise<boolean> {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/magic-link`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ email, locale }),
  });
  return res.ok;
}

/** Create a passwordless account and trigger a magic link (sign up). */
export async function signup(
  fetchFn: FetchFn,
  email: string,
  name?: string,
  locale?: string,
): Promise<boolean> {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/signup`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ email, name, locale }),
  });
  return res.ok;
}

/** Exchange the opaque session id for a short-lived PASETO (server-to-
 *  server; sends the session as a `Cookie` header). `POST /token` is
 *  cookie-authed and mutating, so it requires the session's CSRF
 *  synchroniser token echoed in `X-CSRF-Token` — the BFF holds it (from
 *  the `__Host-mxi_csrf` cookie captured at verify) and forwards it here.
 *  `null` if invalid. */
export async function exchangeToken(
  fetchFn: FetchFn,
  sid: string,
  csrf: string | null,
): Promise<string | null> {
  const headers: Record<string, string> = {
    cookie: `${SESSION_COOKIE}=${sid}`,
  };
  if (csrf) headers["x-csrf-token"] = csrf;
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/token`, {
    method: "POST",
    headers,
  });
  if (!res.ok) return null;
  const body = (await res.json()) as { token?: string };
  return body.token ?? null;
}

/** Resolve the current user for a session id, or `null` when the session
 *  is missing/expired/revoked. */
export async function currentUser(
  fetchFn: FetchFn,
  sid: string,
  csrf: string | null,
): Promise<CurrentUser | null> {
  const token = await exchangeToken(fetchFn, sid, csrf);
  if (!token) return null;
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/me`, {
    headers: { authorization: `Bearer ${token}` },
  });
  if (!res.ok) return null;
  return (await res.json()) as CurrentUser;
}

/** Revoke the session server-side (best-effort) before the cookie clear. */
export async function signout(
  fetchFn: FetchFn,
  sid: string,
  csrf: string | null,
): Promise<void> {
  const token = await exchangeToken(fetchFn, sid, csrf);
  if (!token) return;
  await fetchFn(`${AUTH_API_URL}/api/auth/signout`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}` },
  });
}
