import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { WorkerRepository } from "../../src/lib/api/workers";
import type { Worker } from "../../src/lib/api/types";

function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

const sampleWorker: Worker = {
    id: "p1",
    name: { family: "Smith", given: ["John"] },
    gender: "male",
    birth_date: "1980-01-15",
    active: true,
};

describe("WorkerRepository", () => {
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
});
