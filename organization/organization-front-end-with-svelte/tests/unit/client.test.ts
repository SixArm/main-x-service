import { describe, it, expect, vi } from "vitest";
import { ApiClient, ApiError, nextLink } from "$lib/api/client";

// Pins the ApiClient transport: JSON parsing, URL joining, body
// serialization, the optional explicit bearer token, and ApiError
// construction across the error paths. (Auth is normally handled by the
// BFF proxy, which injects the bearer server-side; the client attaches one
// only when explicitly passed.)

/** A fake `fetch` that records the last call and returns a canned response. */
function fakeFetch(
  status: number,
  body: unknown,
  contentType = "application/json",
) {
  const calls: Array<{ url: string; init: RequestInit }> = [];
  const text = typeof body === "string" ? body : JSON.stringify(body);
  const fn = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), init: init ?? {} });
    return new Response(text, {
      status,
      headers: { "content-type": contentType },
    });
  });
  return { fn: fn as unknown as typeof fetch, calls };
}

function client(fetchImpl: { fn: typeof fetch }) {
  return new ApiClient({ baseUrl: "http://svc.test", fetch: fetchImpl.fn });
}

/** Await a promise expected to reject and return the typed `ApiError`. */
async function caughtError(p: Promise<unknown>): Promise<ApiError> {
  try {
    await p;
    throw new Error("expected the request to reject");
  } catch (e) {
    return e as ApiError;
  }
}

describe("ApiClient", () => {
  it("GET parses a JSON body and resolves it", async () => {
    const f = fakeFetch(200, [{ pid: "p1", name: "Acme" }]);
    const data =
      await client(f).get<Array<{ pid: string }>>("/api/organizations");
    expect(data).toEqual([{ pid: "p1", name: "Acme" }]);
    expect(f.calls[0]?.init.method).toBe("GET");
  });

  it("joins the base URL with leading and non-leading paths", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/organizations");
    expect(f.calls[0]?.url).toBe("http://svc.test/api/organizations");
  });

  // Regression (2026-08-03): a base URL that itself has a path segment —
  // the BFF proxy, `<origin>/api/proxy` — must keep that segment. An
  // earlier version resolved an absolute-path `path` (one starting with
  // `/`) as a host-relative reference, which per the URL spec replaces
  // the base URL's entire path rather than appending to it, silently
  // discarding `/api/proxy` from every request in every BFF-proxied
  // front-end in the family.
  it("keeps the base URL's own path segment (a BFF proxy prefix)", async () => {
    const f = fakeFetch(200, {});
    const proxied = new ApiClient({
      baseUrl: "http://svc.test/api/proxy",
      fetch: f.fn,
    });
    await proxied.get("/api/organizations");
    expect(f.calls[0]?.url).toBe("http://svc.test/api/proxy/api/organizations");
  });

  it("POST serialises the body and sets JSON content-type", async () => {
    const f = fakeFetch(200, { pid: "p1", name: "Acme" });
    await client(f).post("/api/organizations", { body: { name: "Acme" } });
    const init = f.calls[0]?.init as RequestInit;
    expect(init.method).toBe("POST");
    expect(init.body).toBe(JSON.stringify({ name: "Acme" }));
    expect((init.headers as Record<string, string>)["content-type"]).toBe(
      "application/json",
    );
  });

  it("attaches a bearer token when supplied", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/organizations/whoami", { token: "tok-123" });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok-123");
  });

  it("omits the authorization header when no token is given", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/organizations");
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
  });

  it("an explicit null token sends no authorization header", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/x", { token: null });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
  });

  it("throws ApiError with the server message on non-2xx", async () => {
    const f = fakeFetch(422, { error: "name is required" });
    await expect(
      client(f).post("/api/organizations", { body: {} }),
    ).rejects.toMatchObject({
      name: "ApiError",
      status: 422,
      message: "name is required",
    });
  });

  // Pins: an RFC 9457 problem (axa 0.6) yields its detail, else its title.
  it("reads a problem's detail, then its title", async () => {
    const withDetail = fakeFetch(400, {
      type: "about:blank",
      title: "Bad Request",
      status: 400,
      detail: "name must not be blank",
      code: "validation",
    });
    expect(
      (
        await caughtError(
          client(withDetail).post("/api/organizations", { body: {} }),
        )
      ).message,
    ).toBe("name must not be blank");
    const titleOnly = fakeFetch(500, {
      type: "about:blank",
      title: "Internal Server Error",
      status: 500,
      code: "internal_server_error",
    });
    expect(
      (
        await caughtError(
          client(titleOnly).post("/api/organizations", { body: {} }),
        )
      ).message,
    ).toBe("Internal Server Error");
  });

  it("classifies 401 and 400 via the error getters", async () => {
    const unauth = await caughtError(
      client(fakeFetch(401, { error: "nope" })).get("/x"),
    );
    expect(unauth.isUnauthorized).toBe(true);
    expect(unauth.isBadRequest).toBe(false);

    const bad = await caughtError(
      client(fakeFetch(400, { error: "bad" })).get("/x"),
    );
    expect(bad.isBadRequest).toBe(true);
  });

  it("resolves an empty response body as undefined", async () => {
    // The service's soft-delete returns 200 with an empty body.
    const f = fakeFetch(200, "");
    await expect(
      client(f).delete("/api/organizations/p1"),
    ).resolves.toBeUndefined();
  });

  it("falls back to an HTTP status message when the error body is not JSON", async () => {
    const f = fakeFetch(500, "<html>boom</html>", "text/html");
    const err = await caughtError(client(f).get("/x"));
    expect(err.status).toBe(500);
    expect(err.message).toContain("boom");
  });
});

describe("getPage", () => {
  /** A fetch stub answering with `body` and the given headers. */
  function stub(body: unknown, headers: Record<string, string>) {
    return async (_url: string, _init?: RequestInit) =>
      new Response(JSON.stringify(body), { status: 200, headers });
  }

  it("reads the next page from the Link header", async () => {
    const client = new ApiClient({
      baseUrl: "http://x",
      fetch: stub([{ pid: "a", name: "A" }], {
        link: '</api/organizations?limit=1&cursor=AbC_d-1>; rel="next"',
      }) as unknown as typeof fetch,
    });
    const page = await client.getPage<{ pid: string }>("/api/organizations", {
      limit: 1,
    });
    expect(page.items).toHaveLength(1);
    expect(page.next).toBe("/api/organizations?limit=1&cursor=AbC_d-1");
  });

  it("says the last page is the last when there is no Link", async () => {
    const client = new ApiClient({
      baseUrl: "http://x",
      fetch: stub([{ pid: "a" }, { pid: "b" }], {}) as unknown as typeof fetch,
    });
    const page = await client.getPage("/api/organizations");
    expect(page.items).toHaveLength(2);
    expect(page.next).toBeNull();
  });

  it("sends a first page's limit, and a next URL as it is", async () => {
    const seen: string[] = [];
    const client = new ApiClient({
      baseUrl: "http://x",
      fetch: (async (url: string) => {
        seen.push(url);
        return new Response("[]", { status: 200 });
      }) as unknown as typeof fetch,
    });
    await client.getPage("/api/organizations", { limit: 10 });
    await client.getPage("/api/organizations?limit=10&cursor=xyz");
    expect(seen[0]).toContain("/api/organizations?limit=10");
    expect(seen[1]).toContain("/api/organizations?limit=10&cursor=xyz");
    expect(seen.join(" ")).not.toContain("offset");
  });
});

describe("nextLink", () => {
  it("takes only a same-origin path", () => {
    expect(nextLink('<https://evil.example/x>; rel="next"')).toBeNull();
    expect(nextLink('<//evil.example/x>; rel="next"')).toBeNull();
    expect(
      nextLink('</a?cursor=1>; rel="prev", </b?cursor=2>; rel="next"'),
    ).toBe("/b?cursor=2");
    expect(nextLink(null)).toBeNull();
  });
});
