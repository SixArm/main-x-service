import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
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

const pathway: CarePathway = {
  name: "Acute Stroke Care Pathway",
} as CarePathway;

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
});
