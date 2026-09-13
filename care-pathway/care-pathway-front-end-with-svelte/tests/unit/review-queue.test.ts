// Unit tests for the duplicate review-queue repository methods (T-10):
// that `listReviewQueue`/`decideReview` hit the right endpoint/verb with
// the right query params and body encoding. Uses an injected fake fetch
// (no network).
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { CarePathwayRepository } from "../../src/lib/api/care-pathways";
import type { ReviewQueueItem } from "../../src/lib/api/types";

function mockFetch(
  impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
) {
  return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const sampleItem: ReviewQueueItem = {
  id: "r1",
  pathway_id_a: "a1",
  pathway_id_b: "a2",
  match_score: 0.82,
  match_quality: "high",
  detection_method: "import_duplicate_detection",
  score_breakdown: undefined,
  status: "pending",
  provenance: "import",
  reviewed_by: null,
  created_at: "2026-09-13T00:00:00Z",
  reviewed_at: null,
};

describe("CarePathwayRepository review-queue methods", () => {
  // Pins: an omitted filter/limit sends no corresponding query param,
  // and a supplied one is passed through as-is.
  it("GETs the review-queue list with only the supplied filters", async () => {
    const urls: string[] = [];
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input) => {
        urls.push(String(input));
        return jsonResponse({ items: [sampleItem], total: 1 });
      }),
    });
    const repo = new CarePathwayRepository(client);
    const result = await repo.listReviewQueue({
      status: "pending",
      limit: 25,
    });
    await repo.listReviewQueue();
    expect(urls[0]).toContain("/api/care-pathways/review-queue?");
    expect(urls[0]).toContain("status=pending");
    expect(urls[0]).toContain("limit=25");
    expect(urls[1]).toBe("http://test/api/care-pathways/review-queue");
    expect(result.items).toHaveLength(1);
    expect(result.total).toBe(1);
  });

  // Pins: the decision is a JSON POST to the item-scoped path, body
  // `{status}` only.
  it("POSTs a decision to the item-scoped path", async () => {
    let capturedUrl = "";
    let capturedBody = "";
    let capturedMethod = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedMethod = init?.method ?? "";
        capturedBody = init?.body as string;
        return jsonResponse({ ...sampleItem, status: "confirmed" });
      }),
    });
    const repo = new CarePathwayRepository(client);
    const result = await repo.decideReview("r1", "confirmed");
    expect(capturedUrl).toBe(
      "http://test/api/care-pathways/review-queue/r1/decision",
    );
    expect(capturedMethod).toBe("POST");
    expect(JSON.parse(capturedBody)).toEqual({ status: "confirmed" });
    expect(result.status).toBe("confirmed");
  });

  // Pins: an already-decided item surfaces its 422 as a catchable
  // ApiError, not a crash.
  it("surfaces a 422 decision conflict as an ApiError", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () =>
        jsonResponse({ error: "unprocessable_entity" }, 422),
      ),
    });
    const repo = new CarePathwayRepository(client);
    await expect(repo.decideReview("r1", "rejected")).rejects.toMatchObject({
      status: 422,
    });
  });
});
