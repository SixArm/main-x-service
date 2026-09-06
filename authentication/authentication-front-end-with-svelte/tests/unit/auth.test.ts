// Unit tests for the BFF's magic-link-issuing calls (AFE-4): pins that
// requestMagicLink()/signup() classify the upstream response into
// "sent" / "rateLimited" / "failed" rather than collapsing every
// non-2xx outcome into a single boolean, against a mocked fetch. No
// SvelteKit runtime or real authentication service involved.
import { describe, expect, it, vi } from "vitest";
import { requestMagicLink, signup } from "../../src/lib/server/auth";

describe("requestMagicLink", () => {
    it("returns 'sent' on a 2xx upstream response", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
        await expect(requestMagicLink(fetchFn, "a@example.test")).resolves.toBe("sent");
    });

    it("returns 'rateLimited' on a 429 upstream response", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 429 }));
        await expect(requestMagicLink(fetchFn, "a@example.test")).resolves.toBe(
            "rateLimited",
        );
    });

    it("returns 'failed' on any other non-2xx response", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 500 }));
        await expect(requestMagicLink(fetchFn, "a@example.test")).resolves.toBe("failed");
    });

    it("posts the email and locale as JSON", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
        await requestMagicLink(fetchFn, "a@example.test", "cy");
        expect(fetchFn).toHaveBeenCalledWith(
            expect.stringContaining("/api/auth/magic-link"),
            expect.objectContaining({
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify({ email: "a@example.test", locale: "cy" }),
            }),
        );
    });
});

describe("signup", () => {
    it("returns 'sent' on a 2xx upstream response", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
        await expect(signup(fetchFn, "a@example.test")).resolves.toBe("sent");
    });

    it("returns 'rateLimited' on a 429 upstream response", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 429 }));
        await expect(signup(fetchFn, "a@example.test")).resolves.toBe("rateLimited");
    });

    it("returns 'failed' on any other non-2xx response", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 500 }));
        await expect(signup(fetchFn, "a@example.test")).resolves.toBe("failed");
    });

    it("posts the email, name, and locale as JSON", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
        await signup(fetchFn, "a@example.test", "Alice", "cy");
        expect(fetchFn).toHaveBeenCalledWith(
            expect.stringContaining("/api/auth/signup"),
            expect.objectContaining({
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify({ email: "a@example.test", name: "Alice", locale: "cy" }),
            }),
        );
    });
});
