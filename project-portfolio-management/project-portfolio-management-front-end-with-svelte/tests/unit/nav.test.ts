// Unit tests for T-28f's pure helpers (repo `tasks.md` EV-1): role-tailored
// nav ordering and landing-route selection, driven by a deployment-declared
// `view` ABAC attribute. "Default is today's full nav" is the load-bearing
// contract — most of these pin that `view` absent/unmatched changes nothing.
import { describe, expect, it } from "vitest";
import {
    KNOWN_NAV_HREFS,
    landingRouteForView,
    orderNavForView,
    viewAttr,
    type NavItem,
} from "../../src/lib/nav";

const ITEMS: NavItem[] = [
    { href: "/", label: "Home" },
    { href: "/plans", label: "Plans" },
    { href: "/executive", label: "Executive" },
    { href: "/risk", label: "Risk" },
];

describe("orderNavForView", () => {
    it("returns items unchanged (same order) when view is absent", () => {
        expect(orderNavForView(ITEMS, null)).toEqual(ITEMS);
        expect(orderNavForView(ITEMS, undefined)).toEqual(ITEMS);
        expect(orderNavForView(ITEMS, "")).toEqual(ITEMS);
    });

    it("returns items unchanged when view matches no nav href", () => {
        expect(orderNavForView(ITEMS, "no-such-route")).toEqual(ITEMS);
    });

    it("moves the matching item to right after the first (brand/home) item", () => {
        const ordered = orderNavForView(ITEMS, "risk");
        expect(ordered.map((i) => i.href)).toEqual(["/", "/risk", "/plans", "/executive"]);
    });

    it("does not mutate the input array", () => {
        const copy = [...ITEMS];
        orderNavForView(ITEMS, "risk");
        expect(ITEMS).toEqual(copy);
    });

    it("is a no-op when view matches the first item itself", () => {
        // `/` has no attribute-shaped name ("view=" can't equal ""), but
        // guard the boundary anyway: an index-0 match must not reorder.
        expect(orderNavForView(ITEMS, "")).toEqual(ITEMS);
    });
});

describe("landingRouteForView", () => {
    const known = ["/plans", "/executive", "/risk"];

    it("returns the default route when view is absent", () => {
        expect(landingRouteForView(known, null, "/plans")).toBe("/plans");
        expect(landingRouteForView(known, undefined, "/plans")).toBe("/plans");
    });

    it("returns the default route when view matches no known href", () => {
        expect(landingRouteForView(known, "nonsense", "/plans")).toBe("/plans");
    });

    it("returns /{view} when it matches a known href — the acceptance example", () => {
        expect(landingRouteForView(known, "executive", "/plans")).toBe("/executive");
    });

    it("every route in KNOWN_NAV_HREFS is a plausible landing target (leading slash, no trailing slash)", () => {
        for (const href of KNOWN_NAV_HREFS) {
            expect(href.startsWith("/")).toBe(true);
            expect(href.endsWith("/")).toBe(false);
        }
    });
});

describe("viewAttr", () => {
    it("returns null when attrs is absent", () => {
        expect(viewAttr(null)).toBeNull();
        expect(viewAttr(undefined)).toBeNull();
    });

    it("returns null when attrs has no view key", () => {
        expect(viewAttr({ access: ["write"] })).toBeNull();
    });

    it("returns null when view is present but empty", () => {
        expect(viewAttr({ view: [] })).toBeNull();
    });

    it("returns the first view value", () => {
        expect(viewAttr({ view: ["executive", "pmo"] })).toBe("executive");
    });
});
