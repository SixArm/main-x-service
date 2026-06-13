import { describe, it, expect, beforeEach, vi } from "vitest";
import { token, setToken, clearToken } from "$lib/auth.svelte";

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
