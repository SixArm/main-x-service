// Unit tests for PersonRepository: that each method hits the right
// endpoint/verb, and that search response normalization handles every
// envelope shape. Uses an injected fake fetch (no network).
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { PersonRepository } from "../../src/lib/api/persons";
import type {
  EntityLink,
  MatchResult,
  Person,
  ReviewQueueItem,
} from "../../src/lib/api/types";

// Cast a plain async function to the `fetch` type so it can be injected.
function mockFetch(
  impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
) {
  return impl as unknown as typeof fetch;
}

// Build a JSON Response with the given body/status for the fake fetch.
function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

// Reusable fixture person returned by the mocked endpoints.
const samplePerson: Person = {
  id: "p1",
  name: { family: "Smith", given: ["John"] },
  gender: "male",
  birth_date: "1980-01-15",
  active: true,
};

// Reusable fixture cross-service edge (the `same_identity` backbone).
const sampleLink: EntityLink = {
  id: "l1",
  from_ref: "person:p1",
  kind: "same_identity",
  to_ref: "worker:0c4f1e2a-0000-4000-8000-000000000000",
  role: null,
  confidence: 1.0,
  provenance: "operator",
  valid_from: null,
  valid_to: null,
};

// Reusable fixture review-queue row, matching the service's wire shape
// (note `provenance`, which is never null server-side).
const sampleReviewItem: ReviewQueueItem = {
  id: "r1",
  person_id_a: "aaaaaaaa-0000-4000-8000-000000000001",
  person_id_b: "bbbbbbbb-0000-4000-8000-000000000002",
  match_score: 0.91,
  match_quality: "probable",
  detection_method: "batch_deduplication",
  score_breakdown: { name_score: 0.94, birth_date_score: 1.0 },
  status: "pending",
  provenance: "operator",
  reviewed_by: null,
  created_at: "2026-08-04T09:00:00Z",
  reviewed_at: null,
};

describe("PersonRepository", () => {
  // Pins: create() POSTs the person body to /api/persons and returns the
  // created record (with server-assigned id).
  it("POSTs to /api/persons on create", async () => {
    let capturedBody = "";
    let capturedUrl = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedBody = init?.body as string;
        return jsonResponse({ success: true, data: samplePerson, error: null });
      }),
    });
    const repo = new PersonRepository(client);
    const result = await repo.create({
      name: samplePerson.name,
      gender: "male",
    });
    expect(capturedUrl).toContain("/api/persons");
    expect(JSON.parse(capturedBody)).toMatchObject({
      name: { family: "Smith" },
    });
    expect(result.id).toBe("p1");
  });

  // Pins: a bare-array search payload is normalized to {items, total} with
  // total derived from the array length.
  it("normalises bare-array search responses to {items, total}", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse({ success: true, data: [samplePerson], error: null }),
      ),
    });
    const repo = new PersonRepository(client);
    const result = await repo.search({ q: "Smith" });
    expect(result.items).toHaveLength(1);
    expect(result.total).toBe(1);
  });

  // Pins: an {items,total} payload preserves the server-supplied total
  // (rather than recomputing it from the page length).
  it("normalises {items,total} search responses unchanged", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse({
          success: true,
          data: { items: [samplePerson], total: 42 },
          error: null,
        }),
      ),
    });
    const repo = new PersonRepository(client);
    const result = await repo.search({ q: "Smith" });
    expect(result.total).toBe(42);
  });

  // ─── Match / duplicate detection ───────────────────────────────

  // Pins the shape the live service actually returns (found via
  // PRO-P4's live-integration run: `/persons/match` rendered no
  // results at all because this method returned the raw `{matches,
  // total}` envelope object where a `MatchResult[]` was expected).
  const sampleMatch: MatchResult = {
    person: samplePerson,
    score: 1.0,
    quality: "certain",
  };

  it("unwraps {matches, total} from POST /api/persons/match", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse({
          success: true,
          data: { matches: [sampleMatch], total: 1 },
          error: null,
        }),
      ),
    });
    const repo = new PersonRepository(client);
    const result = await repo.match({ name: { family: "Smith" } });
    expect(result).toEqual([sampleMatch]);
  });

  it("accepts a bare-array /api/persons/match response too", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse({ success: true, data: [sampleMatch], error: null }),
      ),
    });
    const repo = new PersonRepository(client);
    const result = await repo.match({ name: { family: "Smith" } });
    expect(result).toEqual([sampleMatch]);
  });

  it("unwraps {has_duplicates, potential_matches} from POST /api/persons/check-duplicates", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse({
          success: true,
          data: { has_duplicates: true, potential_matches: [sampleMatch] },
          error: null,
        }),
      ),
    });
    const repo = new PersonRepository(client);
    const result = await repo.checkDuplicates({ name: samplePerson.name });
    expect(result).toEqual([sampleMatch]);
  });

  // ─── Cross-service links ────────────────────────────────────────

  // Pins: listLinks GETs the per-person links collection and returns the
  // unwrapped array (the service wraps it in the standard envelope).
  it("GETs /api/persons/{id}/links on listLinks", async () => {
    let capturedUrl = "";
    let capturedMethod = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedMethod = init?.method ?? "";
        return jsonResponse({ success: true, data: [sampleLink], error: null });
      }),
    });
    const repo = new PersonRepository(client);
    const links = await repo.listLinks("p1");
    expect(capturedMethod).toBe("GET");
    expect(capturedUrl).toContain("/api/persons/p1/links");
    expect(links).toHaveLength(1);
    expect(links[0]?.to_ref).toBe(sampleLink.to_ref);
  });

  // Pins: createLink POSTs the edge body to the same collection and
  // returns the stored edge.
  it("POSTs the edge body to /api/persons/{id}/links on createLink", async () => {
    let capturedUrl = "";
    let capturedMethod = "";
    let capturedBody = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedMethod = init?.method ?? "";
        capturedBody = init?.body as string;
        return jsonResponse({ success: true, data: sampleLink, error: null });
      }),
    });
    const repo = new PersonRepository(client);
    const created = await repo.createLink("p1", {
      kind: "same_identity",
      to_ref: sampleLink.to_ref,
    });
    expect(capturedMethod).toBe("POST");
    expect(capturedUrl).toContain("/api/persons/p1/links");
    expect(JSON.parse(capturedBody)).toEqual({
      kind: "same_identity",
      to_ref: sampleLink.to_ref,
    });
    expect(created.id).toBe("l1");
  });

  // Pins: deleteLink DELETEs the edge-scoped path. The service answers
  // 200 with an empty payload rather than 204, so the method must
  // tolerate a body it does not read.
  it("DELETEs /api/persons/{id}/links/{linkId} on deleteLink", async () => {
    let capturedUrl = "";
    let capturedMethod = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedMethod = init?.method ?? "";
        return jsonResponse({ success: true, data: {}, error: null });
      }),
    });
    const repo = new PersonRepository(client);
    await expect(repo.deleteLink("p1", "l1")).resolves.toBeUndefined();
    expect(capturedMethod).toBe("DELETE");
    expect(capturedUrl).toContain("/api/persons/p1/links/l1");
  });

  // ─── Review queue ───────────────────────────────────────────────

  // Pins: a bare call sends no query string at all, so the endpoint
  // applies its own defaults (every status, limit 100) — passing
  // `status=` explicitly would be a 422, since there is no "all" token.
  it("GETs /api/persons/review-queue with no query when given no options", async () => {
    let capturedUrl = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input) => {
        capturedUrl = String(input);
        return jsonResponse({
          success: true,
          data: { items: [sampleReviewItem], total: 1 },
          error: null,
        });
      }),
    });
    const repo = new PersonRepository(client);
    const items = await repo.listReviewQueue();
    expect(capturedUrl).toContain("/api/persons/review-queue");
    expect(new URL(capturedUrl).search).toBe("");
    // The envelope's `items` array is unwrapped for the caller.
    expect(items).toHaveLength(1);
    expect(items[0]?.provenance).toBe("operator");
  });

  // Pins: both filters reach the wire under the names the service reads.
  it("passes status and limit through to the query string", async () => {
    let capturedUrl = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input) => {
        capturedUrl = String(input);
        return jsonResponse({
          success: true,
          data: { items: [], total: 0 },
          error: null,
        });
      }),
    });
    const repo = new PersonRepository(client);
    await repo.listReviewQueue({ status: "pending", limit: 25 });
    const url = new URL(capturedUrl);
    expect(url.pathname).toContain("/api/persons/review-queue");
    expect(url.searchParams.get("status")).toBe("pending");
    expect(url.searchParams.get("limit")).toBe("25");
  });

  // Pins: an absent field is omitted rather than sent as the string
  // "undefined", which the service would reject as an unknown status.
  it("omits an absent status while still sending limit", async () => {
    let capturedUrl = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input) => {
        capturedUrl = String(input);
        return jsonResponse({
          success: true,
          data: { items: [], total: 0 },
          error: null,
        });
      }),
    });
    const repo = new PersonRepository(client);
    await repo.listReviewQueue({ limit: 500 });
    const url = new URL(capturedUrl);
    expect(url.searchParams.has("status")).toBe(false);
    expect(url.searchParams.get("limit")).toBe("500");
  });

  // Pins: the decision body's field is `status` (not `decision`), and
  // `reviewed_by` is never sent — the service takes the reviewer from
  // the caller's token.
  it("POSTs {status} to the decision endpoint", async () => {
    let capturedUrl = "";
    let capturedMethod = "";
    let capturedBody = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedMethod = init?.method ?? "";
        capturedBody = init?.body as string;
        return jsonResponse({
          success: true,
          data: { ...sampleReviewItem, status: "confirmed" },
          error: null,
        });
      }),
    });
    const repo = new PersonRepository(client);
    const decided = await repo.decideReview("r1", "confirmed");
    expect(capturedMethod).toBe("POST");
    expect(capturedUrl).toContain("/api/persons/review-queue/r1/decision");
    expect(JSON.parse(capturedBody)).toEqual({ status: "confirmed" });
    expect(decided.status).toBe("confirmed");
  });

  // Pins: a 422 from the link endpoint surfaces the server's own reason
  // (the message the panel shows inline), not a generic failure.
  it("surfaces the server's 422 reason from createLink", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse(
          {
            success: false,
            data: null,
            error: {
              code: "VALIDATION_ERROR",
              message:
                "edge kind `same_identity` does not permit person → organization",
            },
          },
          422,
        ),
      ),
    });
    const repo = new PersonRepository(client);
    await expect(
      repo.createLink("p1", {
        kind: "same_identity",
        to_ref: "organization:x",
      }),
    ).rejects.toThrow(/does not permit person/);
  });
});
