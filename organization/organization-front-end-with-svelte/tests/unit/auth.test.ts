import { describe, it, expect, beforeEach } from "vitest";
import { auth, captureTokenFromHash } from "$lib/auth.svelte";

describe("auth store", () => {
  beforeEach(() => auth.clearToken());

  it("starts signed out", () => {
    expect(auth.token).toBeNull();
  });

  it("setToken / clearToken round-trip through the store", () => {
    auth.setToken("tok-abc");
    expect(auth.token).toBe("tok-abc");

    auth.clearToken();
    expect(auth.token).toBeNull();
  });

  it("trims whitespace and treats a blank token as signed out", () => {
    auth.setToken("  tok-trim  ");
    expect(auth.token).toBe("tok-trim");

    auth.setToken("   ");
    expect(auth.token).toBeNull();
  });
});

describe("captureTokenFromHash", () => {
  it("extracts the token from a well-formed fragment", () => {
    expect(captureTokenFromHash("#access_token=abc.def.ghi")).toBe(
      "abc.def.ghi",
    );
  });

  it("extracts when access_token is one of several fragment params", () => {
    expect(
      captureTokenFromHash("#token_type=Bearer&access_token=jjj&state=x"),
    ).toBe("jjj");
  });

  it("accepts a fragment without a leading '#'", () => {
    expect(captureTokenFromHash("access_token=plain")).toBe("plain");
  });

  it("URL-decodes the token value", () => {
    // A JWT never contains '+' or '/', but the decode path must round-trip
    // a percent-encoded value handed back by the auth front-end.
    expect(captureTokenFromHash("#access_token=a%2Bb%2Fc%3D")).toBe("a+b/c=");
  });

  it("returns null for an empty hash", () => {
    expect(captureTokenFromHash("")).toBeNull();
    expect(captureTokenFromHash("#")).toBeNull();
  });

  it("returns null when there is no access_token", () => {
    expect(captureTokenFromHash("#state=x&token_type=Bearer")).toBeNull();
  });

  it("returns null for garbage and for a blank access_token", () => {
    expect(captureTokenFromHash("#not-a-query-string")).toBeNull();
    expect(captureTokenFromHash("#access_token=")).toBeNull();
    expect(captureTokenFromHash("#access_token=%20%20")).toBeNull();
  });
});
