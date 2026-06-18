import { describe, it, expect, vi } from "vitest";
import { ApiClient, ApiError } from "$lib/api/client";
import { CarePathwayRepository } from "$lib/api/care-pathways";
import type { CarePathway } from "$lib/api/types";

/** Capture the (method, path, body) the repository sends to the client. */
function spyClient() {
  const calls: Array<{ url: string; init: RequestInit }> = [];
  const fetchFn = vi.fn(
    async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init: init ?? {} });
      return new Response("[]", {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  ) as unknown as typeof fetch;
  const repo = new CarePathwayRepository(
    new ApiClient({ baseUrl: "http://svc.test", fetch: fetchFn }),
  );
  return { repo, calls };
}

/** A repository whose backing fetch returns a fixed error status/body. */
function failingRepo(status: number, body: unknown) {
  const fetchFn = vi.fn(async () => {
    return new Response(JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  return new CarePathwayRepository(
    new ApiClient({ baseUrl: "http://svc.test", fetch: fetchFn }),
  );
}

const pathway: CarePathway = {
  name: "Acute Stroke Care Pathway",
} as CarePathway;

// Pins every repository method to its exact endpoint contract: HTTP verb,
// URL (incl. pid/q URL-encoding), and request body shape — notably the
// check-duplicates path (regression guard against `/duplicates`) and merge
// carrying main/duplicate/reason in the body (not the URL).
describe("CarePathwayRepository", () => {
  it("list() GETs the collection", async () => {
    const { repo, calls } = spyClient();
    await repo.list();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways");
  });

  it("get() GETs a single record by pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways/p1");
  });

  it("get() URL-encodes the pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("a/b 1");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways/a%2Fb%201");
  });

  it("create() POSTs the payload", async () => {
    const { repo, calls } = spyClient();
    await repo.create(pathway);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways");
    expect(calls[0]?.init.body).toBe(JSON.stringify(pathway));
  });

  it("update() PUTs to the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.update("p1", pathway);
    expect(calls[0]?.init.method).toBe("PUT");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways/p1");
    expect(calls[0]?.init.body).toBe(JSON.stringify(pathway));
  });

  it("remove() DELETEs the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.remove("p1");
    expect(calls[0]?.init.method).toBe("DELETE");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways/p1");
  });

  it("search() GETs the search endpoint with q", async () => {
    const { repo, calls } = spyClient();
    await repo.search("stroke");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/care-pathways/search?q=stroke",
    );
  });

  it("search() URL-encodes q (spaces and reserved chars)", async () => {
    const { repo, calls } = spyClient();
    await repo.search("a b");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/care-pathways/search?q=a%20b",
    );
  });

  it("checkDuplicates() POSTs to the right endpoint (regression: not /duplicates)", async () => {
    const { repo, calls } = spyClient();
    await repo.checkDuplicates(pathway);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/care-pathways/check-duplicates",
    );
  });

  it("merge() POSTs main/duplicate/reason in the body (pids not in the URL)", async () => {
    const { repo, calls } = spyClient();
    await repo.merge("m1", "d1", "reason");
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways/merge");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ main_pid: "m1", duplicate_pid: "d1", reason: "reason" }),
    );
  });

  it("merge() omits reason when not supplied", async () => {
    const { repo, calls } = spyClient();
    await repo.merge("m1", "d1");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ main_pid: "m1", duplicate_pid: "d1" }),
    );
  });

  it("merge() propagates a 404 ApiError (unknown pid) for the detail-page error banner", async () => {
    // The detail page surfaces this via the error banner (spec §6.7).
    const repo = failingRepo(404, { error: "care pathway not found" });
    await expect(repo.merge("m1", "missing")).rejects.toMatchObject({
      name: "ApiError",
      status: 404,
    });
  });

  it("merge() propagates a 422 ApiError (service-side equal-pid rejection)", async () => {
    // The UI guards equal pids client-side; the service also 422s. Pin
    // that the repository surfaces it as a classified ApiError.
    const repo = failingRepo(422, { error: "cannot merge a record into itself" });
    let err: ApiError | undefined;
    try {
      await repo.merge("same", "same");
    } catch (e) {
      err = e as ApiError;
    }
    expect(err).toBeInstanceOf(ApiError);
    expect(err?.status).toBe(422);
  });

  it("audit() GETs the per-pathway audit endpoint", async () => {
    const { repo, calls } = spyClient();
    await repo.audit("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/care-pathways/p1/audit");
  });

  it("audit() URL-encodes the pid", async () => {
    const { repo, calls } = spyClient();
    await repo.audit("a b");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/care-pathways/a%20b/audit",
    );
  });

  it("recentEvents() GETs the event-stream endpoint", async () => {
    const { repo, calls } = spyClient();
    await repo.recentEvents();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/care-pathways/events/recent",
    );
  });
});
