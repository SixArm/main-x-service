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
