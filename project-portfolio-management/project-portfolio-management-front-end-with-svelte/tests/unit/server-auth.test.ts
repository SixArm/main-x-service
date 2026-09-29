// Unit tests for the new `currentUser` BFF helper (T-28f, repo
// `tasks.md` EV-1): exchanges the session for a bearer, then calls
// `GET /api/auth/me`. Against a mocked fetch — no real auth service.
import { describe, expect, it, vi } from "vitest";
import { currentUser, oidcLoginUrl } from "../../src/lib/server/auth";

describe("currentUser", () => {
    it("returns null when the session->bearer exchange fails", async () => {
        const fetchFn = vi.fn().mockResolvedValue(new Response(null, { status: 401 }));
        await expect(currentUser(fetchFn, "sid-1")).resolves.toBeNull();
    });

    it("returns null when /me itself fails after a successful exchange", async () => {
        const fetchFn = vi
            .fn()
            .mockResolvedValueOnce(
                new Response(JSON.stringify({ token: "bearer-1" }), { status: 200 }),
            )
            .mockResolvedValueOnce(new Response(null, { status: 401 }));
        await expect(currentUser(fetchFn, "sid-1")).resolves.toBeNull();
    });

    it("returns the current user's pid/name/email/attrs on success", async () => {
        const body = {
            pid: "pid-1",
            name: "Alice",
            email: "alice@example.test",
            attrs: { view: ["executive"] },
        };
        const fetchFn = vi
            .fn()
            .mockResolvedValueOnce(
                new Response(JSON.stringify({ token: "bearer-1" }), { status: 200 }),
            )
            .mockResolvedValueOnce(new Response(JSON.stringify(body), { status: 200 }));
        await expect(currentUser(fetchFn, "sid-1")).resolves.toEqual(body);
    });

    it("calls /me with the exchanged bearer as an Authorization header", async () => {
        const fetchFn = vi
            .fn()
            .mockResolvedValueOnce(
                new Response(JSON.stringify({ token: "bearer-1" }), { status: 200 }),
            )
            .mockResolvedValueOnce(
                new Response(
                    JSON.stringify({ pid: "p", name: "n", email: "e", attrs: {} }),
                    { status: 200 },
                ),
            );
        await currentUser(fetchFn, "sid-1");
        expect(fetchFn).toHaveBeenLastCalledWith(
            expect.stringContaining("/api/auth/me"),
            expect.objectContaining({
                headers: { authorization: "Bearer bearer-1" },
            }),
        );
    });
});

describe("oidcLoginUrl", () => {
    it("points at the auth service's /api/auth/oidc/login with an encoded return_url", () => {
        const url = oidcLoginUrl("https://ppm.example.test");
        expect(url).toContain("/api/auth/oidc/login?");
        expect(url).toContain("return_url=https%3A%2F%2Fppm.example.test");
        expect(new URL(url).searchParams.get("return_url")).toBe("https://ppm.example.test");
    });
});
