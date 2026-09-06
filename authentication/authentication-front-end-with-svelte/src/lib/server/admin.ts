// BFF server-side calls to the authentication service's ADMIN attribute
// API (never imported by browser code). The SvelteKit server exchanges
// the session for a short-lived PASETO and calls
// `GET`/`PUT /api/auth/admin/users/{pid}/attributes` with it as a bearer.
// The admin API itself requires the caller to carry `access=admin`
// (403 otherwise) — this module just plumbs the request.

import { env } from "$env/dynamic/private";
import { exchangeToken } from "./auth";
import type { UserAttributes } from "$lib/api/types";

const AUTH_API_URL = env.AUTH_API_URL ?? "http://localhost:5150";

type FetchFn = typeof fetch;

/** Success carries the payload; failure carries the HTTP status and a
 *  human-readable message (from the service's `{error, description}`). */
export type AdminResult<T> =
  | { ok: true; data: T }
  | { ok: false; status: number; message: string };

/** Read the `{error, description}` body of a failed admin response into a
 *  message, falling back to the status text. */
async function errorMessage(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as {
      description?: string;
      error?: string;
    };
    return body.description ?? body.error ?? res.statusText;
  } catch {
    return res.statusText;
  }
}

/** Exchange the session for a bearer, mapping a failure to a `401`. */
async function bearer(
  fetchFn: FetchFn,
  sid: string,
  csrf: string | null,
): Promise<AdminResult<string>> {
  const token = await exchangeToken(fetchFn, sid, csrf);
  if (!token) {
    return {
      ok: false,
      status: 401,
      message: "session expired — sign in again",
    };
  }
  return { ok: true, data: token };
}

/** `GET` a user's ABAC attributes by pid (admin only). */
export async function getUserAttributes(
  fetchFn: FetchFn,
  sid: string,
  csrf: string | null,
  pid: string,
): Promise<AdminResult<UserAttributes>> {
  const auth = await bearer(fetchFn, sid, csrf);
  if (!auth.ok) return auth;
  const res = await fetchFn(
    `${AUTH_API_URL}/api/auth/admin/users/${encodeURIComponent(pid)}/attributes`,
    { headers: { authorization: `Bearer ${auth.data}` } },
  );
  if (!res.ok) {
    return { ok: false, status: res.status, message: await errorMessage(res) };
  }
  return { ok: true, data: (await res.json()) as UserAttributes };
}

/** `PUT` (replace) a user's ABAC attribute map by pid (admin only). */
export async function putUserAttributes(
  fetchFn: FetchFn,
  sid: string,
  csrf: string | null,
  pid: string,
  attributes: Record<string, string[]>,
): Promise<AdminResult<UserAttributes>> {
  const auth = await bearer(fetchFn, sid, csrf);
  if (!auth.ok) return auth;
  const res = await fetchFn(
    `${AUTH_API_URL}/api/auth/admin/users/${encodeURIComponent(pid)}/attributes`,
    {
      method: "PUT",
      headers: {
        authorization: `Bearer ${auth.data}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({ attributes }),
    },
  );
  if (!res.ok) {
    return { ok: false, status: res.status, message: await errorMessage(res) };
  }
  return { ok: true, data: (await res.json()) as UserAttributes };
}
