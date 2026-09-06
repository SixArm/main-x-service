// Pins the BFF cookie-parsing helpers: extracting the opaque session id
// and the CSRF synchroniser token from the auth service's upstream
// `Set-Cookie` lines, so the BFF can re-host both on its own origin.
import { describe, it, expect } from "vitest";
import { isRedirect } from "@sveltejs/kit";
import {
  csrfFromResponse,
  parseSessionId,
  requireSignedIn,
  sessionIdFromResponse,
} from "$lib/server/session";

/** A response whose `getSetCookie()` yields the given Set-Cookie lines. */
function resWithSetCookie(lines: string[]): Response {
  return {
    headers: {
      getSetCookie: () => lines,
      get: () => lines[0] ?? "",
    },
  } as unknown as Response;
}

describe("BFF cookie parsing", () => {
  const sessionLine =
    "__Host-mxi_session=sid-42; HttpOnly; Secure; SameSite=Lax; Path=/";
  const csrfLine = "__Host-mxi_csrf=csrf-abc; Secure; SameSite=Lax; Path=/";
  const lines = [sessionLine, csrfLine];

  it("extracts the session id from a Set-Cookie line", () => {
    expect(parseSessionId(sessionLine)).toBe("sid-42");
    expect(sessionIdFromResponse(resWithSetCookie(lines))).toBe("sid-42");
  });

  it("extracts the CSRF token across multiple Set-Cookie lines", () => {
    expect(csrfFromResponse(resWithSetCookie(lines))).toBe("csrf-abc");
  });

  it("returns null when the CSRF cookie is absent or empty", () => {
    expect(csrfFromResponse(resWithSetCookie(["foo=1"]))).toBeNull();
    expect(csrfFromResponse(resWithSetCookie(["__Host-mxi_csrf="]))).toBeNull();
  });
});

describe("requireSignedIn", () => {
  // Pins: AFE-1 / PRO-H10's page-visit guard — a signed-out visitor is
  // redirected to /signin, never allowed to fall through to the page's
  // own load (e.g. `/admin/attributes`, whose entire purpose is a PUT).
  it("redirects to /signin when no session is present", () => {
    try {
      requireSignedIn({ sessionId: null, csrfToken: null });
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
    expect(() =>
      requireSignedIn({ sessionId: "some-session-id", csrfToken: null }),
    ).not.toThrow();
  });
});
