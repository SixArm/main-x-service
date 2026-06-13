import { describe, it, expect, beforeEach } from "vitest";
import { auth } from "$lib/auth.svelte";

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
