import { describe, it, expect, beforeEach, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import {
  token,
  setToken,
  clearToken,
  captureTokenFromHash,
} from "$lib/auth.svelte";

/** A fake `fetch` that records the last call and returns an empty 200. */
function fakeFetch() {
  const calls: Array<{ url: string; init: RequestInit }> = [];
  const fn = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), init: init ?? {} });
    return new Response("{}", {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  });
  return { fn: fn as unknown as typeof fetch, calls };
}

function client(fetchImpl: { fn: typeof fetch }) {
  return new ApiClient({ baseUrl: "http://svc.test", fetch: fetchImpl.fn });
}

describe("auth token store", () => {
  beforeEach(() => clearToken());

  it("starts empty after clear", () => {
    expect(token()).toBeNull();
  });

  it("setToken / clearToken round-trip through the store", () => {
    setToken("tok-abc");
    expect(token()).toBe("tok-abc");

    clearToken();
    expect(token()).toBeNull();
  });
});

describe("ApiClient reads the auth store by default", () => {
  beforeEach(() => clearToken());

  it("attaches Authorization: Bearer when the store holds a token", async () => {
    setToken("store-tok");
    const f = fakeFetch();
    await client(f).get("/api/care-pathways");
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer store-tok");
  });

  it("omits the Authorization header when the store is empty", async () => {
    const f = fakeFetch();
    await client(f).get("/api/care-pathways");
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
  });

  it("a per-call token overrides the store", async () => {
    setToken("store-tok");
    const f = fakeFetch();
    await client(f).get("/api/care-pathways", { token: "call-tok" });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer call-tok");
  });

  it("a per-call null token suppresses the header even when the store is set", async () => {
    setToken("store-tok");
    const f = fakeFetch();
    await client(f).get("/api/care-pathways", { token: null });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
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
