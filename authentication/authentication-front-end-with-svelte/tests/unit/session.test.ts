// Pins the federation-token persistence contract: `start()` mirrors the
// access token to BOTH the legacy `mxi.auth.*` keys and the shared
// `mxi_access_token` federation key (so same-origin siblings can read it),
// and `clear()` removes all of them. The setup below forces `browser=true`
// and polyfills localStorage so the store actually persists under vitest.
import { describe, it, expect, beforeEach, vi } from "vitest";

// Under vitest there is no SvelteKit app context, so `$app/environment`'s
// `browser` is false and the session store would skip all persistence.
// Force `browser = true` and provide a localStorage polyfill so we can
// assert the store writes BOTH the legacy key and the shared federation
// key. (jsdom here exposes sessionStorage but not localStorage.)
vi.mock("$app/environment", () => ({
  browser: true,
  dev: false,
  building: false,
}));

class MemoryStorage {
  private store = new Map<string, string>();
  getItem(k: string): string | null {
    return this.store.has(k) ? (this.store.get(k) as string) : null;
  }
  setItem(k: string, v: string): void {
    this.store.set(k, String(v));
  }
  removeItem(k: string): void {
    this.store.delete(k);
  }
  clear(): void {
    this.store.clear();
  }
}

const mem = new MemoryStorage();
vi.stubGlobal("localStorage", mem);

// Import AFTER the mocks/stubs are in place.
const { session, FEDERATION_TOKEN_KEY } =
  await import("$lib/auth/session.svelte");
import type { LoginResponse } from "$lib/api/types";

const LOGIN: LoginResponse = {
  token: "jwt.abc.def",
  pid: "11111111-1111-4111-8111-111111111111",
  name: "Alice",
  email: "alice@example.com",
  is_verified: true,
};

describe("session federation key", () => {
  beforeEach(() => {
    session.clear();
    mem.clear();
  });

  it("exports the shared federation key name", () => {
    expect(FEDERATION_TOKEN_KEY).toBe("mxi_access_token");
  });

  it("start() writes BOTH the legacy token key and the federation key", () => {
    session.start(LOGIN);
    // Back-compat bookkeeping.
    expect(localStorage.getItem("mxi.auth.token")).toBe(LOGIN.token);
    expect(localStorage.getItem("mxi.auth.user")).not.toBeNull();
    // Shared federation key (the new behaviour).
    expect(localStorage.getItem(FEDERATION_TOKEN_KEY)).toBe(LOGIN.token);
    expect(session.token).toBe(LOGIN.token);
  });

  it("clear() removes the federation key (and the legacy keys)", () => {
    session.start(LOGIN);
    expect(localStorage.getItem(FEDERATION_TOKEN_KEY)).toBe(LOGIN.token);
    session.clear();
    expect(localStorage.getItem(FEDERATION_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem("mxi.auth.token")).toBeNull();
    expect(localStorage.getItem("mxi.auth.user")).toBeNull();
    expect(session.token).toBeNull();
  });
});
