// Unit tests for ApiClient. These pin the wire behaviour: URL joining, JSON
// body + content-type, the optional explicit per-call bearer token, and
// non-2xx -> ApiError mapping (incl. empty and non-JSON bodies). Auth is
// normally handled by the BFF proxy, which injects the bearer server-side;
// the client attaches one only when explicitly passed.
import { describe, it, expect, vi } from "vitest";
import { ApiClient, ApiError } from "$lib/api/client";

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
  // Pins: a 2xx JSON body is parsed and resolved as-is.
  it("GET parses a JSON body and resolves it", async () => {
    const f = fakeFetch(200, [{ pid: "p1", title: "Housing benefit appeal" }]);
    const data = await client(f).get<Array<{ pid: string }>>("/api/cases");
    expect(data).toEqual([{ pid: "p1", title: "Housing benefit appeal" }]);
    expect(f.calls[0]?.init.method).toBe("GET");
  });

  // Pins: relative path is resolved against baseUrl into the full URL.
  it("joins the base URL with leading and non-leading paths", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/cases");
    expect(f.calls[0]?.url).toBe("http://svc.test/api/cases");
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
    await proxied.get("/api/cases");
    expect(f.calls[0]?.url).toBe("http://svc.test/api/proxy/api/cases");
  });

  // Pins: POST stringifies the body and sets the JSON content-type header.
  it("POST serialises the body and sets JSON content-type", async () => {
    const f = fakeFetch(200, { pid: "p1", title: "Housing benefit appeal" });
    await client(f).post("/api/cases", {
      body: { title: "Housing benefit appeal" },
    });
    const init = f.calls[0]?.init as RequestInit;
    expect(init.method).toBe("POST");
    expect(init.body).toBe(JSON.stringify({ title: "Housing benefit appeal" }));
    expect((init.headers as Record<string, string>)["content-type"]).toBe(
      "application/json",
    );
  });

  // Pins: a per-call string token becomes `Authorization: Bearer …`.
  it("attaches a bearer token when supplied per call", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/cases/whoami", { token: "tok-123" });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok-123");
  });

  // Pins: with no per-call token, no auth header is sent (the BFF injects it).
  it("omits the authorization header when no token is given", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/cases");
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
  });

  // Pins: an explicit per-call `null` sends no authorization header.
  it("an explicit null token sends no authorization header", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/cases", { token: null });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
  });

  // Pins: a non-2xx JSON body maps to ApiError carrying status + message.
  it("throws ApiError with the server message on non-2xx", async () => {
    const f = fakeFetch(422, { error: "title is required" });
    await expect(
      client(f).post("/api/cases", { body: {} }),
    ).rejects.toMatchObject({
      name: "ApiError",
      status: 422,
      message: "title is required",
    });
  });

  // Pins: the isUnauthorized / isBadRequest convenience getters.
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

  // Pins: an empty 2xx body (soft-delete) resolves to undefined.
  it("resolves an empty response body as undefined", async () => {
    // The service's soft-delete returns 200 with an empty body.
    const f = fakeFetch(200, "");
    await expect(client(f).delete("/api/cases/p1")).resolves.toBeUndefined();
  });

  // Pins: a non-JSON error body still yields an ApiError with status + text.
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
    return (async () =>
      new Response(JSON.stringify(body), {
        status: 200,
        headers,
      })) as unknown as typeof fetch;
  }

  it("reads the pagination headers the service sends", async () => {
    const client = new ApiClient({
      baseUrl: "http://x",
      fetch: stub([{ pid: "a" }], {
        "x-total-count": "431",
        "x-limit": "25",
        "x-offset": "50",
      }),
    });
    const page = await client.getPage<{ pid: string }>("/api/things", {
      limit: 25,
      offset: 50,
    });
    expect(page.items).toHaveLength(1);
    expect(page.total).toBe(431);
    expect(page.limit).toBe(25);
    expect(page.offset).toBe(50);
  });

  it("falls back to the page length when a service sends no headers", async () => {
    const client = new ApiClient({
      baseUrl: "http://x",
      fetch: stub([{ pid: "a" }, { pid: "b" }], {}),
    });
    const page = await client.getPage("/api/things");
    expect(page.total).toBe(2);
    expect(page.offset).toBe(0);
  });
});
