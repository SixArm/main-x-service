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

  // Pins: merge -> POST /api/cases/merge with the service's own body shape
  // (`main_pid` / `duplicate_pid` / `reason` — not person's field names).
  it("merge() POSTs the merge request to /api/cases/merge", async () => {
    const { repo, calls } = spyClient();
    await repo.merge({
      main_pid: "main-1",
      duplicate_pid: "dup-1",
      reason: "confirmed duplicate",
    });
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toContain("/api/cases/merge");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/merge");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({
        main_pid: "main-1",
        duplicate_pid: "dup-1",
        reason: "confirmed duplicate",
      }),
    );
  });

  // Pins: recentMerges -> GET /api/cases/merges/recent.
  it("recentMerges() GETs the merge history", async () => {
    const { repo, calls } = spyClient();
    await repo.recentMerges();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/merges/recent");
  });

  // Cross-service links (`subject_of`, case → person). The per-case link
  // paths are nested under the case pid; the bulk `/api/cases/links` dump
  // is a different, privileged endpoint this repository deliberately does
  // not expose, so these pins also guard against collapsing the two.
  it("listLinks() GETs the case's links", async () => {
    const { repo, calls } = spyClient();
    await repo.listLinks("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1/links");
  });

  it("createLink() POSTs the edge with kind fixed to subject_of", async () => {
    const { repo, calls } = spyClient();
    await repo.createLink("p1", {
      kind: "subject_of",
      to_ref: "person:0c4f1e2a-0000-4000-8000-000000000000",
      confidence: 1,
      provenance: null,
      valid_from: null,
      valid_to: null,
    });
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1/links");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({
        kind: "subject_of",
        to_ref: "person:0c4f1e2a-0000-4000-8000-000000000000",
        confidence: 1,
        provenance: null,
        valid_from: null,
        valid_to: null,
      }),
    );
  });

  it("deleteLink() DELETEs the edge under the case", async () => {
    const { repo, calls } = spyClient();
    await repo.deleteLink("p1", "e1");
    expect(calls[0]?.init.method).toBe("DELETE");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1/links/e1");
  });

  it("link paths URL-encode both the pid and the link id", async () => {
    const { repo, calls } = spyClient();
    await repo.deleteLink("a/b", "c d");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/a%2Fb/links/c%20d");
  });

  // Full-text search (spec §13, PRO-P15). Pins the query string shape:
  // `q` first, then `fuzzy`/`phonetic` only when set, then `limit`/`offset`
  // appended by the shared `getPage()` pager.
  it("search() GETs the search endpoint with just q", async () => {
    const { repo, calls } = spyClient();
    await repo.search({ q: "housing" });
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/search?q=housing");
  });

  it("search() includes fuzzy/phonetic only when true", async () => {
    const { repo, calls } = spyClient();
    await repo.search({ q: "housing", fuzzy: true, phonetic: true });
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/cases/search?q=housing&fuzzy=true&phonetic=true",
    );
  });

  it("search() omits fuzzy/phonetic when false", async () => {
    const { repo, calls } = spyClient();
    await repo.search({ q: "housing", fuzzy: false, phonetic: false });
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/search?q=housing");
  });

  it("search() appends limit/offset via the page window", async () => {
    const { repo, calls } = spyClient();
    await repo.search({ q: "housing", limit: 10, offset: 20 });
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/cases/search?q=housing&limit=10&offset=20",
    );
  });

  it("search() URL-encodes the query", async () => {
    const { repo, calls } = spyClient();
    await repo.search({ q: "a b&c" });
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/search?q=a+b%26c");
  });

  // Audit / events (spec §13, PRO-P15).
  it("audit() GETs the case's audit trail", async () => {
    const { repo, calls } = spyClient();
    await repo.audit("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/p1/audit");
  });

  it("audit() URL-encodes the pid", async () => {
    const { repo, calls } = spyClient();
    await repo.audit("a/b 1");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/a%2Fb%201/audit");
  });

  it("recentAudit() GETs the system-wide recent audit log", async () => {
    const { repo, calls } = spyClient();
    await repo.recentAudit();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/audit/recent");
  });

  it("recentEvents() GETs the recent event stream", async () => {
    const { repo, calls } = spyClient();
    await repo.recentEvents();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/cases/events/recent");
  });
});
