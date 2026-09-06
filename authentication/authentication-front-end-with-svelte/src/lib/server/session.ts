// BFF server-side session helpers (never imported by browser code).
//
// The durable session lives in the authentication service; the browser
// holds only an opaque session id in the httpOnly `__Host-mxi_session`
// cookie (see `agents/share/authentication-sessions.md` §6). The SvelteKit
// server reads/sets that cookie here and exchanges it for short-lived
// PASETO tokens server-side — the browser never sees a token.

import { redirect } from "@sveltejs/kit";

/**
 * Page-visit guard (PRO-H10 / AFE-1): redirect an unauthenticated visitor
 * to `/signin` rather than render a page whose entire purpose is a
 * mutation. `locals.sessionId` is presence-only (set from the httpOnly
 * cookie, never re-validated here) — a UX convenience in front of the
 * auth service's real enforcement, not a substitute for it. Mirrors the
 * person/worker/thing/event/course reference (`$lib/server/session.ts`,
 * repo `tasks.md` PRO-H10).
 *
 * Declared as a TypeScript assertion function (rather than plain `void`,
 * as the reference crates have it): unlike those crates' guarded pages,
 * this route's `load` still needs `locals.sessionId` narrowed to `string`
 * afterward to pass it to `getUserAttributes`, so the narrowing is real
 * type safety, not decoration.
 */
export function requireSignedIn(
  locals: App.Locals,
): asserts locals is App.Locals & { sessionId: string } {
  if (locals.sessionId === null) {
    redirect(303, "/signin");
  }
}

/** The httpOnly session cookie name (host-locked via the `__Host-` prefix). */
export const SESSION_COOKIE = "__Host-mxi_session";

/** Cookie attributes for `cookies.set` — httpOnly, Secure, host-locked. */
export const SESSION_COOKIE_OPTIONS = {
  path: "/",
  httpOnly: true,
  secure: true,
  sameSite: "lax",
} as const;

/**
 * The CSRF synchroniser-token cookie. The auth service sets it
 * (`__Host-mxi_csrf`, readable) alongside the session at establishment;
 * the BFF re-hosts it on THIS origin as **httpOnly** (the browser never
 * needs it — browser↔BFF CSRF is SvelteKit's native form-action origin
 * check), and echoes it in the `X-CSRF-Token` header when it calls the
 * auth service's cookie-authed `POST /api/auth/token`.
 */
export const CSRF_COOKIE = "__Host-mxi_csrf";

/** Cookie attributes for the BFF-hosted CSRF token (httpOnly here). */
export const CSRF_COOKIE_OPTIONS = SESSION_COOKIE_OPTIONS;

/** Value of the named cookie in one raw `Set-Cookie` line, or `null`. */
function parseCookie(setCookie: string, name: string): string | null {
  const prefix = `${name}=`;
  const segment = setCookie
    .split(";")
    .map((s) => s.trim())
    .find((s) => s.startsWith(prefix));
  if (!segment) return null;
  const value = segment.slice(prefix.length);
  return value.length > 0 ? value : null;
}

/** All `Set-Cookie` lines of an upstream response (undici-aware). */
function setCookieLines(response: Response): string[] {
  const headers = response.headers as Headers & {
    getSetCookie?: () => string[];
  };
  return headers.getSetCookie?.() ?? [response.headers.get("set-cookie") ?? ""];
}

/** The CSRF token set by the auth service on an upstream response (from
 *  its `__Host-mxi_csrf` `Set-Cookie`), or `null` when absent. */
export function csrfFromResponse(response: Response): string | null {
  for (const line of setCookieLines(response)) {
    const token = parseCookie(line, CSRF_COOKIE);
    if (token) return token;
  }
  return null;
}

/**
 * Extract the `__Host-mxi_session` value from a raw `Set-Cookie` header
 * line emitted by the authentication service (so the BFF can re-host the
 * session id on its own origin). Returns `null` when absent/empty.
 */
export function parseSessionId(setCookie: string): string | null {
  const prefix = `${SESSION_COOKIE}=`;
  const segment = setCookie
    .split(";")
    .map((s) => s.trim())
    .find((s) => s.startsWith(prefix));
  if (!segment) return null;
  const value = segment.slice(prefix.length);
  return value.length > 0 ? value : null;
}

/**
 * Find the session id across all `Set-Cookie` lines of an upstream
 * response. Uses `getSetCookie()` where available (Node/undici), falling
 * back to the single `set-cookie` header.
 */
export function sessionIdFromResponse(response: Response): string | null {
  const headers = response.headers as Headers & {
    getSetCookie?: () => string[];
  };
  const lines = headers.getSetCookie?.() ?? [
    response.headers.get("set-cookie") ?? "",
  ];
  for (const line of lines) {
    const sid = parseSessionId(line);
    if (sid) return sid;
  }
  return null;
}
