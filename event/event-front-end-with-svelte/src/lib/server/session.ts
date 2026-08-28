// BFF server-side session helpers (never imported by browser code).
//
// The durable session lives in the authentication service; the browser
// holds only an opaque session id in the httpOnly `__Host-mxi_session`
// cookie (see `agents/share/authentication-sessions.md` §6). The SvelteKit
// server reads/sets that cookie and exchanges it for short-lived PASETO
// tokens server-side — the browser never sees a token.

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
 * The CSRF double-submit cookie (`agents/share/authentication-sessions.md`
 * §4). Set alongside {@link SESSION_COOKIE} at session establishment
 * (`verify/+page.server.ts`). Unlike the session cookie this one is
 * **deliberately NOT httpOnly** — client-side JS (`$lib/api/client.ts`)
 * must read it to echo its value in the `X-CSRF-Token` header on every
 * mutating browser→BFF request; the proxy
 * (`routes/api/proxy/[...path]/+server.ts`) then checks the header
 * matches the cookie before forwarding upstream. This is a *separate*
 * cookie from the same-named one in `authentication-front-end-with-svelte`,
 * which is httpOnly there because it protects a different hop (BFF→auth
 * service, not browser→BFF).
 */
export const CSRF_COOKIE = "__Host-mxi_csrf";

/** Cookie attributes for the browser-readable CSRF token — NOT httpOnly. */
export const CSRF_COOKIE_OPTIONS = {
  path: "/",
  httpOnly: false,
  secure: true,
  sameSite: "lax",
} as const;

/** Mint a fresh, high-entropy CSRF token for a new session. */
export function generateCsrfToken(): string {
  return crypto.randomUUID();
}

/**
 * Double-submit check: the `X-CSRF-Token` request header must be present
 * and equal to the `__Host-mxi_csrf` cookie value. Both values originate
 * from the same trusted party (this BFF, at session establishment), so a
 * plain equality check is sufficient — there is no secret being compared
 * against an attacker-guessable value the way a MAC would need.
 */
export function verifyCsrf(
  cookieValue: string | undefined | null,
  headerValue: string | undefined | null,
): boolean {
  return Boolean(cookieValue) && cookieValue === headerValue;
}

/** Extract `__Host-mxi_session` from a single `Set-Cookie` header line. */
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

/** Find the session id across all `Set-Cookie` lines of an upstream
 *  response (uses `getSetCookie()` where available). */
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
