// BFF server-side calls to the authentication service (never bundled into
// the browser). The SvelteKit server holds the session cookie and does all
// token handling here; the browser never sees an access token.

import { AUTH_API_URL } from "./config";
import { SESSION_COOKIE } from "./session";

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

/** Request a magic link for an existing account, telling the auth service
 *  to return to THIS app's `/verify` (per-app login). */
export async function requestMagicLink(
  fetchFn: FetchFn,
  email: string,
  returnUrl: string,
  locale?: string,
): Promise<boolean> {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/magic-link`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ email, locale, return_url: returnUrl }),
  });
  return res.ok;
}

/** Exchange the opaque session id for a short-lived PASETO (server-to-
 *  server; sends the session as a `Cookie` header). `null` if invalid. */
export async function exchangeToken(
  fetchFn: FetchFn,
  sid: string,
): Promise<string | null> {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/token`, {
    method: "POST",
    headers: { cookie: `${SESSION_COOKIE}=${sid}` },
  });
  if (!res.ok) return null;
  const body = (await res.json()) as { token?: string };
  return body.token ?? null;
}

/** Revoke the session server-side (best-effort) before the cookie clear. */
export async function signout(fetchFn: FetchFn, sid: string): Promise<void> {
  const token = await exchangeToken(fetchFn, sid);
  if (!token) return;
  await fetchFn(`${AUTH_API_URL}/api/auth/signout`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}` },
  });
}
