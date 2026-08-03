// Unit tests for PersonRepository: that each method hits the right
// endpoint/verb, and that search response normalization handles every
// envelope shape. Uses an injected fake fetch (no network).
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { PersonRepository } from "../../src/lib/api/persons";
import type { EntityLink, Person } from "../../src/lib/api/types";

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
            repo.createLink("p1", { kind: "same_identity", to_ref: "organization:x" }),
        ).rejects.toThrow(/does not permit person/);
    });
});
