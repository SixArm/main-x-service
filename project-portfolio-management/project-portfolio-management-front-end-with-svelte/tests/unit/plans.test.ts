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
    await repo.merge({
      main_pid: "main-1",
      duplicate_pid: "dup-2",
      reason: "confirmed",
    });
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toContain("/api/plans/merge");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/merge");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ main_pid: "main-1", duplicate_pid: "dup-2", reason: "confirmed" }),
    );
  });

  it("merge() omits an absent reason", async () => {
    const { repo, calls } = spyClient();
    await repo.merge({ main_pid: "main-1", duplicate_pid: "dup-2" });
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ main_pid: "main-1", duplicate_pid: "dup-2" }),
    );
  });

  it("recentMerges() GETs the merge history", async () => {
    const { repo, calls } = spyClient();
    await repo.recentMerges();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/plans/merges/recent");
  });
});

// Pins the pagination contract (agents/share/restful.md) the `/plans`
// list route relies on: `listPage()` sends `limit`/`offset` and reads
// `X-Total-Count`/`X-Limit`/`X-Offset` off the response, falling back to
// the page length when a header is absent (a service that predates the
// headers still works).
describe("PlanRepository.listPage", () => {
  /** A PlanRepository whose fetch answers with `body` and `headers`. */
  function pagedRepo(body: unknown, headers: Record<string, string>) {
    const calls: string[] = [];
    const fetchFn = (async (input: RequestInfo | URL) => {
      calls.push(String(input));
      return new Response(JSON.stringify(body), { status: 200, headers });
    }) as unknown as typeof fetch;
    const repo = new PlanRepository(
      new ApiClient({ baseUrl: "http://svc.test", fetch: fetchFn }),
    );
    return { repo, calls };
  }

  it("GETs the collection and reads the pagination headers", async () => {
    const { repo, calls } = pagedRepo(
      [{ pid: "p1", name: "Apollo" }],
      { "x-total-count": "12", "x-limit": "1", "x-offset": "0" },
    );

    const page = await repo.listPage();

    expect(calls).toEqual(["http://svc.test/api/plans"]);
    expect(page.items).toEqual([{ pid: "p1", name: "Apollo" }]);
    expect(page.total).toBe(12);
    expect(page.limit).toBe(1);
    expect(page.offset).toBe(0);
  });

  it("sends limit/offset and carries the parent roll-up query through", async () => {
    const { repo, calls } = pagedRepo([], {
      "x-total-count": "0",
      "x-limit": "5",
      "x-offset": "10",
    });

    await repo.listPage({ limit: 5, offset: 10 }, "port-1");

    expect(calls).toEqual([
      "http://svc.test/api/plans?parent=port-1&limit=5&offset=10",
    ]);
  });

  it("falls back to the page length when headers are absent", async () => {
    const { repo } = pagedRepo([{ pid: "p1", name: "Apollo" }], {});

    const page = await repo.listPage();

    expect(page.total).toBe(1);
    expect(page.limit).toBe(1);
    expect(page.offset).toBe(0);
  });
});
