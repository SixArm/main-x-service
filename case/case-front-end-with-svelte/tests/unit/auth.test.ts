import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  token,
  setToken,
  clearToken,
  captureTokenFromHash,
} from "$lib/auth.svelte";

describe("auth session store", () => {
  beforeEach(() => {
    clearToken();
  });

  it("starts with no token", () => {
    expect(token()).toBeNull();
  });

  it("setToken / clearToken round-trip through the store", () => {
    setToken("tok-abc");
    expect(token()).toBe("tok-abc");
    clearToken();
    expect(token()).toBeNull();
  });

  it("writes through to localStorage under the shared key when available", () => {
    // jsdom here does not expose `localStorage`, so the store runs
    // in-memory; install a minimal stub to assert the write-through path
    // (and that the shared key is used). The store guards for absence, so
    // this only exercises the present-storage branch.
    const store = new Map<string, string>();
    const stub = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
    };
    vi.stubGlobal("localStorage", stub);
    try {
      setToken("tok-xyz");
      expect(store.get("mxi_access_token")).toBe("tok-xyz");
      clearToken();
      expect(store.has("mxi_access_token")).toBe(false);
    } finally {
      vi.unstubAllGlobals();
    }
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
