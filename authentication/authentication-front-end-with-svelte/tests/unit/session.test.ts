// Pins the BFF cookie-parsing helpers: extracting the opaque session id
// and the CSRF synchroniser token from the auth service's upstream
// `Set-Cookie` lines, so the BFF can re-host both on its own origin.
import { describe, it, expect } from "vitest";
import {
  csrfFromResponse,
  parseSessionId,
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
