// Unit tests for the auth session store. Pins: the in-memory token
// round-trip, the localStorage write-through under the shared key, and the
// pure hash-parsing rules of `captureTokenFromHash` (decode + reject blanks).
import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  token,
  setToken,
  clearToken,
  captureTokenFromHash,
} from "$lib/auth.svelte";

describe("auth session store", () => {
  // Reset to signed-out before each case so tests don't leak token state.
  beforeEach(() => {
    clearToken();
  });

  // Pins: a fresh store reports unauthenticated.
  it("starts with no token", () => {
    expect(token()).toBeNull();
  });

  // Pins: set makes the token current; clear returns to null.
  it("setToken / clearToken round-trip through the store", () => {
    setToken("tok-abc");
    expect(token()).toBe("tok-abc");
    clearToken();
    expect(token()).toBeNull();
  });

  // Pins: when storage exists, set/clear persist/remove under the shared key.
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
  // Pins: the happy path — a lone `access_token` fragment yields the JWT.
  it("extracts the token from a well-formed fragment", () => {
    expect(captureTokenFromHash("#access_token=abc.def.ghi")).toBe(
      "abc.def.ghi",
    );
  });

  // Pins: parsing finds `access_token` among other fragment params.
  it("extracts when access_token is one of several fragment params", () => {
    expect(
      captureTokenFromHash("#token_type=Bearer&access_token=jjj&state=x"),
    ).toBe("jjj");
  });

  // Pins: a leading '#' is optional.
  it("accepts a fragment without a leading '#'", () => {
    expect(captureTokenFromHash("access_token=plain")).toBe("plain");
  });

  // Pins: percent-encoded values are URL-decoded.
  it("URL-decodes the token value", () => {
    // A JWT never contains '+' or '/', but the decode path must round-trip
    // a percent-encoded value handed back by the auth front-end.
    expect(captureTokenFromHash("#access_token=a%2Bb%2Fc%3D")).toBe("a+b/c=");
  });

  // Pins: empty / bare-'#' fragments yield null.
  it("returns null for an empty hash", () => {
    expect(captureTokenFromHash("")).toBeNull();
    expect(captureTokenFromHash("#")).toBeNull();
  });

  // Pins: a fragment lacking `access_token` yields null.
  it("returns null when there is no access_token", () => {
    expect(captureTokenFromHash("#state=x&token_type=Bearer")).toBeNull();
  });

  // Pins: non-query garbage and empty/whitespace-only tokens are rejected.
  it("returns null for garbage and for a blank access_token", () => {
    expect(captureTokenFromHash("#not-a-query-string")).toBeNull();
    expect(captureTokenFromHash("#access_token=")).toBeNull();
    expect(captureTokenFromHash("#access_token=%20%20")).toBeNull();
  });
});
