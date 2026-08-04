// Unit tests for WorkerRepository: that it targets the right endpoints and
// normalizes the two possible search response shapes. Uses an injected
// fetch, so no running service is needed.
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { WorkerRepository } from "../../src/lib/api/workers";
import type { EntityLink, Worker } from "../../src/lib/api/types";

// Cast a plain async impl to the structural `fetch` type for injection.
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Build a JSON Response with the given body and status for the mock fetch.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

// Minimal valid Worker reused across cases.
const sampleWorker: Worker = {
    id: "p1",
    name: { family: "Smith", given: ["John"] },
    gender: "male",
    birth_date: "1980-01-15",
    active: true,
};

// A cross-service `same_identity` edge (worker → person), as the service's
// `LinkView` serialises it.
const personRef = "person:0c4f1e2a-0000-4000-8000-000000000000";
const sampleLink: EntityLink = {
    id: "e1",
    from_ref: "worker:p1",
    kind: "same_identity",
    to_ref: personRef,
    role: null,
    confidence: 1,
    provenance: "operator",
    valid_from: null,
    valid_to: null,
};

describe("WorkerRepository", () => {
    // Pins: create() POSTs the worker body to /api/workers and returns the
    // unwrapped created record.
    it("POSTs to /api/workers on create", async () => {
        let capturedBody = "";
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedBody = init?.body as string;
                return jsonResponse({ success: true, data: sampleWorker, error: null });
            }),
        });
        const repo = new WorkerRepository(client);
        const result = await repo.create({ name: sampleWorker.name, gender: "male" });
        expect(capturedUrl).toContain("/api/workers");
        expect(JSON.parse(capturedBody)).toMatchObject({ name: { family: "Smith" } });
        expect(result.id).toBe("p1");
    });

    // Pins: a bare-array search payload becomes { items, total } with total
    // derived from array length.
    it("normalises bare-array search responses to {items, total}", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: [sampleWorker], error: null }),
            ),
        });
        const repo = new WorkerRepository(client);
        const result = await repo.search({ q: "Smith" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(1);
    });

    // Pins: an already-{items,total} payload passes its `total` through
    // untouched (here 42, not the item count of 1).
    it("normalises {items,total} search responses unchanged", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse(
                    { success: true, data: { items: [sampleWorker], total: 42 }, error: null },
                ),
            ),
        });
        const repo = new WorkerRepository(client);
        const result = await repo.search({ q: "Smith" });
        expect(result.total).toBe(42);
    });

    // Pins: the cross-service link endpoints — URL, method, and that the
    // envelope's `data` is what the caller receives.
    it("GETs the links sub-resource on listLinks", async () => {
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
        const repo = new WorkerRepository(client);
        const links = await repo.listLinks("p1");
        expect(capturedMethod).toBe("GET");
        expect(capturedUrl).toContain("/api/workers/p1/links");
        expect(links).toHaveLength(1);
        expect(links[0]?.to_ref).toBe(personRef);
    });

    // Pins: createLink POSTs the edge body to the worker's links collection.
    it("POSTs the edge body on createLink", async () => {
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
        const repo = new WorkerRepository(client);
        const created = await repo.createLink("p1", {
            kind: "same_identity",
            to_ref: personRef,
            confidence: 1,
        });
        expect(capturedMethod).toBe("POST");
        expect(capturedUrl).toContain("/api/workers/p1/links");
        expect(JSON.parse(capturedBody)).toMatchObject({
            kind: "same_identity",
            to_ref: personRef,
            confidence: 1,
        });
        expect(created.id).toBe("e1");
    });

    // Pins: deleteLink DELETEs the per-edge path and tolerates the
    // service's empty `{}` success envelope (there is nothing to read).
    it("DELETEs the per-link path on deleteLink", async () => {
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
        const repo = new WorkerRepository(client);
        await expect(repo.deleteLink("p1", "e1")).resolves.toBeUndefined();
        expect(capturedMethod).toBe("DELETE");
        expect(capturedUrl).toContain("/api/workers/p1/links/e1");
    });

    // Pins: a 422 from the edge validator surfaces as an ApiError carrying
    // the service's human-readable reason, which the panel shows inline.
    it("surfaces a 422 edge-validation reason from createLink", async () => {
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
                                "edge kind `same_identity` does not permit worker → organization",
                        },
                    },
                    422,
                ),
            ),
        });
        const repo = new WorkerRepository(client);
        await expect(
            repo.createLink("p1", { kind: "same_identity", to_ref: "organization:x" }),
        ).rejects.toMatchObject({ status: 422, code: "VALIDATION_ERROR" });
    });

    // Pins: a bare call sends no query string at all, so the endpoint
    // applies its own defaults (every status, limit 100) — passing
    // `status=` explicitly would be a 422, since there is no "all" token.
    it("GETs /api/workers/review-queue with no query when given no options", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({
                    success: true,
                    data: {
                        items: [
                            {
                                id: "r1",
                                worker_id_a: "aaaaaaaa-0000-4000-8000-000000000001",
                                worker_id_b: "bbbbbbbb-0000-4000-8000-000000000002",
                                match_score: 0.91,
                                match_quality: "probable",
                                detection_method: "batch_deduplication",
                                score_breakdown: { name_score: 0.94 },
                                status: "pending",
                                reviewed_by: null,
                                created_at: "2026-08-04T09:00:00Z",
                                reviewed_at: null,
                            },
                        ],
                        total: 1,
                    },
                    error: null,
                });
            }),
        });
        const repo = new WorkerRepository(client);
        const items = await repo.listReviewQueue();
        expect(capturedUrl).toContain("/api/workers/review-queue");
        expect(new URL(capturedUrl).search).toBe("");
        // The envelope's `items` array is unwrapped for the caller.
        expect(items).toHaveLength(1);
        expect(items[0]?.status).toBe("pending");
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
        const repo = new WorkerRepository(client);
        await repo.listReviewQueue({ status: "pending", limit: 25 });
        const url = new URL(capturedUrl);
        expect(url.pathname).toContain("/api/workers/review-queue");
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
        const repo = new WorkerRepository(client);
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
                    data: {
                        id: "r1",
                        worker_id_a: "aaaaaaaa-0000-4000-8000-000000000001",
                        worker_id_b: "bbbbbbbb-0000-4000-8000-000000000002",
                        match_score: 0.91,
                        match_quality: "probable",
                        detection_method: "batch_deduplication",
                        score_breakdown: { name_score: 0.94 },
                        status: "confirmed",
                        reviewed_by: null,
                        created_at: "2026-08-04T09:00:00Z",
                        reviewed_at: "2026-08-04T09:05:00Z",
                    },
                    error: null,
                });
            }),
        });
        const repo = new WorkerRepository(client);
        const decided = await repo.decideReview("r1", "confirmed");
        expect(capturedMethod).toBe("POST");
        expect(capturedUrl).toContain("/api/workers/review-queue/r1/decision");
        expect(JSON.parse(capturedBody)).toEqual({ status: "confirmed" });
        expect(decided.status).toBe("confirmed");
    });
});
