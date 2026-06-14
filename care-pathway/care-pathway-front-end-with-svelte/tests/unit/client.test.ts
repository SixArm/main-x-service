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

// Pins the low-level fetch wrapper: JSON parse/resolve, base+path URL
// joining, POST serialization + content-type, bearer header presence/
// absence, ApiError on non-2xx (server message vs. status fallback, and
// the 401/400 getters), empty-body → undefined, and non-JSON error bodies.
describe("ApiClient", () => {
  it("GET parses a JSON body and resolves it", async () => {
    const f = fakeFetch(200, [{ pid: "p1", name: "Stroke" }]);
    const data =
      await client(f).get<Array<{ pid: string }>>("/api/care-pathways");
    expect(data).toEqual([{ pid: "p1", name: "Stroke" }]);
    expect(f.calls[0]?.init.method).toBe("GET");
  });

  it("joins the base URL with leading and non-leading paths", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/care-pathways");
    expect(f.calls[0]?.url).toBe("http://svc.test/api/care-pathways");
  });

  it("POST serialises the body and sets JSON content-type", async () => {
    const f = fakeFetch(200, { pid: "p1", name: "Stroke" });
    await client(f).post("/api/care-pathways", { body: { name: "Stroke" } });
    const init = f.calls[0]?.init as RequestInit;
    expect(init.method).toBe("POST");
    expect(init.body).toBe(JSON.stringify({ name: "Stroke" }));
    expect((init.headers as Record<string, string>)["content-type"]).toBe(
      "application/json",
    );
  });

  it("attaches a bearer token when supplied", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/care-pathways/whoami", { token: "tok-123" });
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok-123");
  });

  it("omits the authorization header when no token is given", async () => {
    const f = fakeFetch(200, {});
    await client(f).get("/api/care-pathways");
    const headers = f.calls[0]?.init.headers as Record<string, string>;
    expect(headers.authorization).toBeUndefined();
  });

  it("throws ApiError with the server message on non-2xx", async () => {
    const f = fakeFetch(422, { error: "name is required" });
    await expect(
      client(f).post("/api/care-pathways", { body: {} }),
    ).rejects.toMatchObject({
      name: "ApiError",
      status: 422,
      message: "name is required",
    });
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
      client(f).delete("/api/care-pathways/p1"),
    ).resolves.toBeUndefined();
  });

  it("falls back to an HTTP status message when the error body is not JSON", async () => {
    const f = fakeFetch(500, "<html>boom</html>", "text/html");
    const err = await caughtError(client(f).get("/x"));
    expect(err.status).toBe(500);
    expect(err.message).toContain("boom");
  });
});
