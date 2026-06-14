// Pins the AuthRepository → HTTP contract: that each method targets the
// correct endpoint path + method, places the token in the path/headers as
// expected, and serialises the right JSON body (including the
// undefined-name drop). The fetch layer is a spy, so these assert the
// request the repository *issues*, not the service behaviour.
import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import { AuthRepository } from "$lib/api/auth";

/** Capture the (method, url, body, headers) the repository sends. */
function spyClient(responseBody: unknown = {}) {
  const calls: Array<{ url: string; init: RequestInit }> = [];
  const fetchFn = vi.fn(
    async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init: init ?? {} });
      return new Response(JSON.stringify(responseBody), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  ) as unknown as typeof fetch;
  const repo = new AuthRepository(
    new ApiClient({ baseUrl: "http://auth.test", fetch: fetchFn }),
  );
  return { repo, calls };
}

describe("AuthRepository", () => {
  it("signup() POSTs email + name to /api/auth/signup", async () => {
    const { repo, calls } = spyClient();
    await repo.signup("a@example.com", "Alice");
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://auth.test/api/auth/signup");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ email: "a@example.com", name: "Alice" }),
    );
  });

  it("signup() sends name as undefined when omitted", async () => {
    const { repo, calls } = spyClient();
    await repo.signup("a@example.com");
    // `undefined` drops out of JSON.stringify, leaving just the email.
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ email: "a@example.com" }),
    );
  });

  it("requestMagicLink() POSTs the email to /api/auth/magic-link", async () => {
    const { repo, calls } = spyClient();
    await repo.requestMagicLink("a@example.com");
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://auth.test/api/auth/magic-link");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ email: "a@example.com" }),
    );
  });

  it("verify() GETs /api/auth/magic-link/{token} and returns the login", async () => {
    const login = {
      token: "jwt",
      pid: "p1",
      name: "Alice",
      email: "a@example.com",
      is_verified: true,
    };
    const { repo, calls } = spyClient(login);
    const result = await repo.verify("magic-123");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe(
      "http://auth.test/api/auth/magic-link/magic-123",
    );
    expect(result).toEqual(login);
  });

  it("verify() URL-encodes the token", async () => {
    const { repo, calls } = spyClient();
    await repo.verify("a/b 1");
    expect(calls[0]?.url).toBe(
      "http://auth.test/api/auth/magic-link/a%2Fb%201",
    );
  });

  it("me() GETs /api/auth/me with the bearer token", async () => {
    const { repo, calls } = spyClient({
      pid: "p1",
      name: "Alice",
      email: "a@example.com",
    });
    await repo.me("tok-abc");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://auth.test/api/auth/me");
    const headers = calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok-abc");
  });

  it("signout() POSTs /api/auth/signout with the bearer token", async () => {
    const { repo, calls } = spyClient();
    await repo.signout("tok-abc");
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://auth.test/api/auth/signout");
    const headers = calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok-abc");
  });
});
