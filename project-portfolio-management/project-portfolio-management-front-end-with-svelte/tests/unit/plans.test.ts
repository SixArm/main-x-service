// Unit tests for PlanRepository. These pin the exact HTTP verb + URL each
// method emits (the repository is the single source of endpoint paths),
// via a fake fetch that records calls. There is one recursive collection,
// so every path is under `/api/plans`.
import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import { PlanRepository } from "$lib/api/plans";
import type { Plan } from "$lib/api/types";

/** Capture the (method, url, body) the repository sends to the client. */
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
  const repo = new PlanRepository(
    new ApiClient({ baseUrl: "http://svc.test", fetch: fetchFn }),
  );
  return { repo, calls };
}

const record: Plan = {
  kind: "Project",
  name: "Apollo platform migration",
};

describe("PlanRepository", () => {
  it("list() GETs the collection", async () => {
    const { repo, calls } = spyClient();
    await repo.list();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans");
  });

  it("list(parentRef) adds the roll-up query", async () => {
    const { repo, calls } = spyClient();
    await repo.list("port-1");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans?parent=port-1");
  });

  it("search() GETs the search endpoint with an encoded q", async () => {
    const { repo, calls } = spyClient();
    await repo.search("a b");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/search?q=a%20b");
  });

  it("get() GETs a single record by pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/p1");
  });

  it("get() URL-encodes the pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("a/b 1");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/a%2Fb%201");
  });

  it("create() POSTs the payload", async () => {
    const { repo, calls } = spyClient();
    await repo.create(record);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans");
    expect(calls[0]?.init.body).toBe(JSON.stringify(record));
  });

  it("update() PUTs to the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.update("p1", record);
    expect(calls[0]?.init.method).toBe("PUT");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/p1");
    expect(calls[0]?.init.body).toBe(JSON.stringify(record));
  });

  it("remove() DELETEs the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.remove("p1");
    expect(calls[0]?.init.method).toBe("DELETE");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/p1");
  });

  it("checkDuplicates() POSTs to check-duplicates (not /duplicates)", async () => {
    const { repo, calls } = spyClient();
    await repo.checkDuplicates(record);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/check-duplicates");
  });

  it("merge() POSTs the merge body", async () => {
    const { repo, calls } = spyClient();
    await repo.merge("main-1", "dup-2", "confirmed");
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/merge");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ main_pid: "main-1", duplicate_pid: "dup-2", reason: "confirmed" }),
    );
  });
});
