import { describe, it, expect, beforeEach } from "vitest";
import {
  isAllowedReturnTo,
  parseAllowlist,
  nextDestination,
  persistReturnTo,
  readReturnTo,
  clearReturnTo,
  RETURN_TO_STORAGE_KEY,
} from "$lib/auth/return-to";

const SELF = "https://auth.example.com";
const APP = "https://app.example.com";

describe("parseAllowlist", () => {
  it("returns an empty list for undefined or empty input", () => {
    expect(parseAllowlist(undefined)).toEqual([]);
    expect(parseAllowlist("")).toEqual([]);
    expect(parseAllowlist("   ")).toEqual([]);
    expect(parseAllowlist(",, ,")).toEqual([]);
  });

  it("splits on commas and trims each entry", () => {
    expect(parseAllowlist("https://a.test, https://b.test")).toEqual([
      "https://a.test",
      "https://b.test",
    ]);
  });

  it("drops blank entries between commas", () => {
    expect(parseAllowlist("https://a.test,,  ,https://b.test,")).toEqual([
      "https://a.test",
      "https://b.test",
    ]);
  });
});

describe("isAllowedReturnTo", () => {
  it("allows an origin that is exactly on the allowlist", () => {
    expect(isAllowedReturnTo(`${APP}/dashboard`, [APP], SELF)).toBe(true);
  });

  it("allows our own origin (self) even when the allowlist is empty", () => {
    expect(isAllowedReturnTo(`${SELF}/account`, [], SELF)).toBe(true);
  });

  it("with an empty allowlist allows only self", () => {
    expect(isAllowedReturnTo(`${SELF}/x`, [], SELF)).toBe(true);
    expect(isAllowedReturnTo(`${APP}/x`, [], SELF)).toBe(false);
  });

  it("rejects a different host", () => {
    expect(isAllowedReturnTo("https://evil.example.com/x", [APP], SELF)).toBe(
      false,
    );
  });

  it("rejects a different port (origin includes port)", () => {
    expect(
      isAllowedReturnTo("https://app.example.com:8443/x", [APP], SELF),
    ).toBe(false);
  });

  it("rejects a different scheme (http vs https origin)", () => {
    expect(isAllowedReturnTo("http://app.example.com/x", [APP], SELF)).toBe(
      false,
    );
  });

  it("rejects non-http(s) schemes: javascript:", () => {
    expect(
      isAllowedReturnTo("javascript:alert(document.cookie)", [APP], SELF),
    ).toBe(false);
  });

  it("rejects non-http(s) schemes: data:", () => {
    expect(isAllowedReturnTo("data:text/html,<h1>x</h1>", [APP], SELF)).toBe(
      false,
    );
  });

  it("rejects a relative URL (not absolute)", () => {
    expect(isAllowedReturnTo("/dashboard", [APP], SELF)).toBe(false);
    expect(isAllowedReturnTo("//app.example.com/x", [APP], SELF)).toBe(false);
  });

  it("rejects garbage / unparseable input", () => {
    expect(isAllowedReturnTo("not a url", [APP], SELF)).toBe(false);
    expect(isAllowedReturnTo("", [APP], SELF)).toBe(false);
    expect(isAllowedReturnTo("http://", [APP], SELF)).toBe(false);
  });

  it("matches by origin regardless of path / query / fragment", () => {
    expect(isAllowedReturnTo(`${APP}/deep/path?q=1#frag`, [APP], SELF)).toBe(
      true,
    );
  });
});

describe("nextDestination", () => {
  const TOKEN = "jwt.abc.def";

  it("returns an external redirect with the token in the fragment when allowed", () => {
    const dest = nextDestination(`${APP}/dash`, TOKEN, [APP], SELF);
    expect(dest).toEqual({
      kind: "external",
      url: `${APP}/dash#access_token=${encodeURIComponent(TOKEN)}`,
    });
  });

  it("url-encodes the token in the fragment", () => {
    const dest = nextDestination(`${APP}/`, "a/b c+d", [APP], SELF);
    expect(dest).toEqual({
      kind: "external",
      url: `${APP}/#access_token=${encodeURIComponent("a/b c+d")}`,
    });
  });

  it("goes home (no token appended) for a non-allowlisted return_to", () => {
    expect(
      nextDestination("https://evil.example.com/x", TOKEN, [APP], SELF),
    ).toEqual({ kind: "home" });
  });

  it("goes home when return_to is null / undefined / empty", () => {
    expect(nextDestination(null, TOKEN, [APP], SELF)).toEqual({ kind: "home" });
    expect(nextDestination(undefined, TOKEN, [APP], SELF)).toEqual({
      kind: "home",
    });
    expect(nextDestination("", TOKEN, [APP], SELF)).toEqual({ kind: "home" });
  });

  it("never appends the token to a javascript: URL", () => {
    const dest = nextDestination("javascript:alert(1)", TOKEN, [APP], SELF);
    expect(dest).toEqual({ kind: "home" });
  });

  it("allows self-origin redirect", () => {
    const dest = nextDestination(`${SELF}/landing`, TOKEN, [], SELF);
    expect(dest.kind).toBe("external");
  });
});

describe("persist / read / clear return_to (sessionStorage)", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it("persists an allowlisted return_to from the URL", () => {
    const url = new URL(
      `${SELF}/signin?return_to=${encodeURIComponent(`${APP}/x`)}`,
    );
    persistReturnTo(url, [APP], SELF);
    expect(readReturnTo()).toBe(`${APP}/x`);
    expect(sessionStorage.getItem(RETURN_TO_STORAGE_KEY)).toBe(`${APP}/x`);
  });

  it("ignores a present-but-not-allowlisted return_to (never stored)", () => {
    const url = new URL(
      `${SELF}/signin?return_to=${encodeURIComponent("https://evil.test/x")}`,
    );
    persistReturnTo(url, [APP], SELF);
    expect(readReturnTo()).toBeNull();
  });

  it("is a no-op when there is no return_to param", () => {
    persistReturnTo(new URL(`${SELF}/signin`), [APP], SELF);
    expect(readReturnTo()).toBeNull();
  });

  it("clearReturnTo removes the parked value", () => {
    sessionStorage.setItem(RETURN_TO_STORAGE_KEY, `${APP}/x`);
    clearReturnTo();
    expect(readReturnTo()).toBeNull();
  });
});
