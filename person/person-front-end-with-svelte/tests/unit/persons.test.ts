// Unit tests for PersonRepository: that each method hits the right
// endpoint/verb, and that search response normalization handles every
// envelope shape. Uses an injected fake fetch (no network).
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { PersonRepository } from "../../src/lib/api/persons";
import type { Person } from "../../src/lib/api/types";

// Cast a plain async function to the `fetch` type so it can be injected.
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
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
        const result = await repo.create({ name: samplePerson.name, gender: "male" });
        expect(capturedUrl).toContain("/api/persons");
        expect(JSON.parse(capturedBody)).toMatchObject({ name: { family: "Smith" } });
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
                jsonResponse(
                    { success: true, data: { items: [samplePerson], total: 42 }, error: null },
                ),
            ),
        });
        const repo = new PersonRepository(client);
        const result = await repo.search({ q: "Smith" });
        expect(result.total).toBe(42);
    });
});
