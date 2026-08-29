// Unit tests for the BFF session/CSRF helpers: the double-submit
// verification logic and the token minter. No SvelteKit runtime involved.
import { describe, expect, it } from "vitest";
import { isRedirect } from "@sveltejs/kit";
import {
  CSRF_COOKIE,
  CSRF_COOKIE_OPTIONS,
  SESSION_COOKIE_OPTIONS,
  generateCsrfToken,
  requireSignedIn,
  verifyCsrf,
} from "../../src/lib/server/session";

describe("verifyCsrf", () => {
  // Pins: matching cookie + header passes.
  it("accepts a matching cookie and header token", () => {
    expect(verifyCsrf("abc123", "abc123")).toBe(true);
  });

  // Pins: a mismatched header fails.
  it("rejects a mismatched header token", () => {
    expect(verifyCsrf("abc123", "different")).toBe(false);
  });

  // Pins: a missing header fails.
  it("rejects a missing header token", () => {
    expect(verifyCsrf("abc123", null)).toBe(false);
    expect(verifyCsrf("abc123", undefined)).toBe(false);
  });

  // Pins: a missing cookie fails, even if a header happens to be present
  // (an attacker cannot supply the cookie, so this can't be satisfied
  // cross-site).
  it("rejects a missing cookie token", () => {
    expect(verifyCsrf(null, "abc123")).toBe(false);
    expect(verifyCsrf(undefined, "abc123")).toBe(false);
  });

  // Pins: both absent fails (not vacuously true).
  it("rejects when both are absent", () => {
    expect(verifyCsrf(null, null)).toBe(false);
    expect(verifyCsrf(undefined, undefined)).toBe(false);
  });

  // Pins: an empty-string cookie is treated as absent, not as a valid
  // token an empty header could match.
  it("rejects an empty-string cookie even against an empty-string header", () => {
    expect(verifyCsrf("", "")).toBe(false);
  });
});

describe("requireSignedIn", () => {
  // Pins: PRO-H10's page-visit guard — a signed-out visitor is redirected
  // to /signin, never allowed to fall through to the page's own load.
  it("redirects to /signin when no session is present", () => {
    try {
      requireSignedIn({ sessionId: null });
      throw new Error("expected requireSignedIn to throw a redirect");
    } catch (error) {
      expect(isRedirect(error)).toBe(true);
      if (isRedirect(error)) {
        expect(error.status).toBe(303);
        expect(error.location).toBe("/signin");
      }
    }
  });

  // Pins: a signed-in visitor (any non-null session id) passes through
  // silently — no throw, no return value to check.
  it("does not throw when a session is present", () => {
    expect(() => requireSignedIn({ sessionId: "some-session-id" })).not.toThrow();
  });
});

describe("generateCsrfToken", () => {
  // Pins: tokens are non-empty and not trivially predictable/repeated.
  it("mints distinct, non-empty tokens", () => {
    const a = generateCsrfToken();
    const b = generateCsrfToken();
    expect(a.length).toBeGreaterThan(0);
    expect(a).not.toBe(b);
  });
});

describe("CSRF cookie configuration", () => {
  // Pins: the cookie name matches the family convention and the
  // hardcoded duplicate in `$lib/api/client.ts`.
  it("uses the __Host-mxi_csrf cookie name", () => {
    expect(CSRF_COOKIE).toBe("__Host-mxi_csrf");
  });

  // Pins: the whole point of the double-submit pattern — browser JS must
  // be able to read this cookie, so it must NOT be httpOnly, unlike the
  // session cookie.
  it("is NOT httpOnly, unlike the session cookie", () => {
    expect(CSRF_COOKIE_OPTIONS.httpOnly).toBe(false);
    expect(SESSION_COOKIE_OPTIONS.httpOnly).toBe(true);
  });

  // Pins: still Secure + SameSite=Lax + host-locked path, matching the
  // session cookie's other attributes.
  it("is Secure and SameSite=Lax like the session cookie", () => {
    expect(CSRF_COOKIE_OPTIONS.secure).toBe(true);
    expect(CSRF_COOKIE_OPTIONS.sameSite).toBe("lax");
    expect(CSRF_COOKIE_OPTIONS.path).toBe("/");
  });
});
