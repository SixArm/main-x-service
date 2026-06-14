// Unit tests for CaseRepository. These pin the exact HTTP verb + URL each
// method emits (the repository is the single source of endpoint paths), via
// a fake fetch that records calls and returns an empty JSON array.
import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import { CaseRepository } from "$lib/api/cases";
import type { Case } from "$lib/api/types";

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
  const repo = new CaseRepository(
    new ApiClient({ baseUrl: "http://svc.test", fetch: fetchFn }),
  );
  return { repo, calls };
}

const record: Case = {
  title: "Housing benefit appeal",
} as Case;

describe("CaseRepository", () => {
  // Pins: list -> GET /api/cases.
  it("list() GETs the collection", async () => {
    const { repo, calls } = spyClient();
    await repo.list();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases");
  });

  // Pins: get -> GET /api/cases/{pid}.
  it("get() GETs a single record by pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1");
  });

  // Pins: pids with `/` and spaces are percent-encoded into the path.
  it("get() URL-encodes the pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("a/b 1");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/a%2Fb%201");
  });

  // Pins: create -> POST /api/cases with the JSON-serialised record.
  it("create() POSTs the payload", async () => {
    const { repo, calls } = spyClient();
    await repo.create(record);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases");
    expect(calls[0]?.init.body).toBe(JSON.stringify(record));
  });

  // Pins: update -> PUT /api/cases/{pid} with the JSON-serialised record.
  it("update() PUTs to the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.update("p1", record);
    expect(calls[0]?.init.method).toBe("PUT");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1");
    expect(calls[0]?.init.body).toBe(JSON.stringify(record));
  });

  // Pins: remove -> DELETE /api/cases/{pid}.
  it("remove() DELETEs the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.remove("p1");
    expect(calls[0]?.init.method).toBe("DELETE");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1");
  });

  it("checkDuplicates() POSTs to the right endpoint (regression: not /duplicates)", async () => {
    const { repo, calls } = spyClient();
    await repo.checkDuplicates(record);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/check-duplicates");
  });
});
