// BFF server-side session helpers (never imported by browser code).
//
// The durable session lives in the authentication service; the browser
// holds only an opaque session id in the httpOnly `__Host-mxi_session`
// cookie (see `agents/share/authentication-sessions.md` §6). The SvelteKit
// server reads/sets that cookie and exchanges it for short-lived PASETO
// tokens server-side — the browser never sees a token.

import { redirect } from "@sveltejs/kit";

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

/**
 * Page-visit guard (PRO-H10, `agents/share/authentication-sessions.md`):
 * redirect an unauthenticated visitor away from a page whose entire
 * purpose is submitting a mutation (create/edit/merge/review-decide) —
 * read/list/search/view pages stay public, mirroring the backend's own
 * default-allow-read / mutation-deny ABAC posture
 * (`agents/share/authorization-attributes.md` §5) rather than inventing a
 * separate front-end policy.
 *
 * `locals.sessionId` is presence-only (set from the httpOnly cookie in
 * `hooks.server.ts`, never validated against the auth service here) —
 * this is a UX convenience, not the real enforcement boundary. The
 * backend's own `COURSE_REQUIRE_AUTH` + ABAC guard is what actually
 * protects the API; this only stops a signed-out visitor from *seeing* a
 * form whose submit would otherwise fail (or silently succeed against a
 * deployment with enforcement off).
 *
 * Deliberately does not carry a `next` param back through `/signin`: the
 * magic-link round trip only preserves `return_url`'s origin today (see
 * `requestMagicLink`), and threading a return path through it would touch
 * the authentication-service contract, not just this app. A visitor who
 * signs in from a guarded page lands on `/` and navigates back manually —
 * a known, deliberate v1 limitation, not an oversight.
 */
export function requireSignedIn(locals: App.Locals): void {
  if (locals.sessionId === null) {
    redirect(303, "/signin");
  }
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
