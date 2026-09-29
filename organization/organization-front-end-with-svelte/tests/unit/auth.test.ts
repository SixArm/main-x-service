// EV-2: the OIDC entry-point URL is a browser NAVIGATION target, not a
// fetch — this pins only the pure string-building.
import { describe, expect, it } from "vitest";
import { oidcLoginUrl } from "../../src/lib/server/auth";

describe("oidcLoginUrl", () => {
    it("points at the auth service's /api/auth/oidc/login", () => {
        expect(oidcLoginUrl("https://organization-front-end.example.test")).toContain(
            "/api/auth/oidc/login?",
        );
    });

    it("carries the caller's origin, encoded, as return_url", () => {
        const url = oidcLoginUrl("https://organization-front-end.example.test");
        expect(url).toContain(
            `return_url=${encodeURIComponent("https://organization-front-end.example.test")}`,
        );
        expect(new URL(url).searchParams.get("return_url")).toBe(
            "https://organization-front-end.example.test",
        );
    });
});
