// Unit tests for the BFF session cookie helpers (PF-T18, PF-T23).
// `parseSessionId`/`sessionIdFromResponse` are pure string parsing with
// no SvelteKit runtime involved — the cheapest way to pin their
// behaviour directly, rather than only exercising them indirectly
// through `/verify`'s `load` function.
import { describe, expect, it } from "vitest";
import {
  SESSION_COOKIE,
  SESSION_COOKIE_OPTIONS,
  parseSessionId,
  sessionIdFromResponse,
} from "../../src/lib/server/session";

describe("parseSessionId", () => {
  it("extracts the session id from a single Set-Cookie line", () => {
    expect(parseSessionId(`${SESSION_COOKIE}=abc123; Path=/; HttpOnly`)).toBe(
      "abc123",
    );
  });

  it("returns null when the cookie is absent", () => {
    expect(parseSessionId("other-cookie=value; Path=/")).toBeNull();
  });

  it("returns null for an empty cookie value", () => {
    expect(parseSessionId(`${SESSION_COOKIE}=; Path=/`)).toBeNull();
  });

  it("finds the cookie regardless of attribute order/spacing", () => {
    expect(
      parseSessionId(`  Secure ;${SESSION_COOKIE}=xyz  ; SameSite=Lax`),
    ).toBe("xyz");
  });
});

describe("sessionIdFromResponse", () => {
  it("finds the session id via getSetCookie() when available", () => {
    const response = {
      headers: {
        getSetCookie: () => [
          "unrelated=1",
          `${SESSION_COOKIE}=found-it; Path=/; HttpOnly`,
        ],
        get: () => null,
      },
    } as unknown as Response;
    expect(sessionIdFromResponse(response)).toBe("found-it");
  });

  it("falls back to a single set-cookie header when getSetCookie is unavailable", () => {
    const response = {
      headers: {
        get: (name: string) =>
          name === "set-cookie" ? `${SESSION_COOKIE}=fallback; Path=/` : null,
      },
    } as unknown as Response;
    expect(sessionIdFromResponse(response)).toBe("fallback");
  });

  it("returns null when no Set-Cookie line carries the session cookie", () => {
    const response = {
      headers: {
        getSetCookie: () => ["other=1"],
        get: () => null,
      },
    } as unknown as Response;
    expect(sessionIdFromResponse(response)).toBeNull();
  });
});

describe("session cookie configuration", () => {
  it("uses the __Host-mxi_session cookie name", () => {
    expect(SESSION_COOKIE).toBe("__Host-mxi_session");
  });

  it("is httpOnly, Secure, SameSite=Lax and host-locked", () => {
    expect(SESSION_COOKIE_OPTIONS.httpOnly).toBe(true);
    expect(SESSION_COOKIE_OPTIONS.secure).toBe(true);
    expect(SESSION_COOKIE_OPTIONS.sameSite).toBe("lax");
    expect(SESSION_COOKIE_OPTIONS.path).toBe("/");
  });
});
