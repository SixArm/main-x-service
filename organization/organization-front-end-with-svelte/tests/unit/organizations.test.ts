import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import { OrganizationRepository } from "$lib/api/organizations";
import type { Organization } from "$lib/api/types";

// Pins the repository->HTTP contract: every method maps to the correct
// verb + path (with pid encoding) and body. The fetch is faked, so these
// run without the Rust service.

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
  const repo = new OrganizationRepository(
    new ApiClient({ baseUrl: "http://svc.test", fetch: fetchFn }),
  );
  return { repo, calls };
}

const org: Organization = { name: "Acme, Inc." } as Organization;

describe("OrganizationRepository", () => {
  it("list() GETs the collection", async () => {
    const { repo, calls } = spyClient();
    await repo.list();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations");
  });

  it("get() GETs a single record by pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("p1");
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/p1");
  });

  it("get() URL-encodes the pid", async () => {
    const { repo, calls } = spyClient();
    await repo.get("a/b 1");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/a%2Fb%201");
  });

  it("create() POSTs the payload", async () => {
    const { repo, calls } = spyClient();
    await repo.create(org);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations");
    expect(calls[0]?.init.body).toBe(JSON.stringify(org));
  });

  it("update() PUTs to the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.update("p1", org);
    expect(calls[0]?.init.method).toBe("PUT");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/p1");
    expect(calls[0]?.init.body).toBe(JSON.stringify(org));
  });

  it("remove() DELETEs the pid path", async () => {
    const { repo, calls } = spyClient();
    await repo.remove("p1");
    expect(calls[0]?.init.method).toBe("DELETE");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/p1");
  });

  it("checkDuplicates() POSTs to the right endpoint (regression: not /duplicates)", async () => {
    const { repo, calls } = spyClient();
    await repo.checkDuplicates(org);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/organizations/check-duplicates",
    );
  });

  it("deduplicate() POSTs an empty-object body to the scan endpoint", async () => {
    const { repo, calls } = spyClient();
    await repo.deduplicate();
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/deduplicate");
    expect(calls[0]?.init.body).toBe("{}");
  });

  it("listReviewQueue() GETs the stored queue with no query string by default", async () => {
    const { repo, calls } = spyClient();
    await repo.listReviewQueue();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/organizations/review-queue",
    );
  });

  it("listReviewQueue() sends status and limit as query params", async () => {
    const { repo, calls } = spyClient();
    await repo.listReviewQueue({ status: "confirmed", limit: 25 });
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/organizations/review-queue?status=confirmed&limit=25",
    );
  });

  // Pins: "all" is the *absence* of the status param, not a literal
  // value — the endpoint has no "all" token and answers 422 for one it
  // does not recognise.
  it("listReviewQueue() omits status entirely when undefined", async () => {
    const { repo, calls } = spyClient();
    await repo.listReviewQueue({ limit: 100 });
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/organizations/review-queue?limit=100",
    );
  });

  it("decideReview() POSTs the verdict to the item's decision path", async () => {
    const { repo, calls } = spyClient();
    await repo.decideReview("item-1", "confirmed");
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/organizations/review-queue/item-1/decision",
    );
    expect(calls[0]?.init.body).toBe(JSON.stringify({ status: "confirmed" }));
  });

  it("merge() POSTs the two pids to the merge endpoint", async () => {
    const { repo, calls } = spyClient();
    await repo.merge({
      main_pid: "p1",
      duplicate_pid: "p2",
      reason: "same company",
    });
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/merge");
    // The service's MergeRequest is {main_pid, duplicate_pid, reason} —
    // NOT the {main_*_id, duplicate_*_id, merge_reason} shape some
    // sibling services use.
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({
        main_pid: "p1",
        duplicate_pid: "p2",
        reason: "same company",
      }),
    );
  });

  it("merge() sends a null reason when none is given", async () => {
    const { repo, calls } = spyClient();
    await repo.merge({ main_pid: "p1", duplicate_pid: "p2", reason: null });
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ main_pid: "p1", duplicate_pid: "p2", reason: null }),
    );
  });

  it("recentMerges() GETs the merge history", async () => {
    const { repo, calls } = spyClient();
    await repo.recentMerges();
    expect(calls[0]?.init.method).toBe("GET");
    expect(calls[0]?.url).toBe(
      "http://svc.test/api/organizations/merges/recent",
    );
  });

  // Pins the `/review` comparison panel's live-breakdown path: the stored
  // review-queue item never carries `score_breakdown` on the wire, so the
  // panel calls `/match` against the loaded pair instead.
  it("matchAgainst() POSTs the query and candidates to the match endpoint", async () => {
    const { repo, calls } = spyClient();
    const dup: Organization = { name: "Acme Inc" } as Organization;
    await repo.matchAgainst(org, [dup]);
    expect(calls[0]?.init.method).toBe("POST");
    expect(calls[0]?.url).toBe("http://svc.test/api/organizations/match");
    expect(calls[0]?.init.body).toBe(
      JSON.stringify({ query: org, candidates: [dup] }),
    );
  });
});
