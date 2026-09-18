import { describe, it, expect } from "vitest";
import { existsSync } from "node:fs";
import { join } from "node:path";

import { ONBOARDING_ROLES } from "../../src/lib/onboarding";

// T-28p acceptance: every route named in the guide exists — walk the
// guide's steps against the real route tree rather than trusting the
// data file to stay in sync with `src/routes` by hand.
function pagePathFor(route: string): string {
    const segments = route.split("/").filter(Boolean);
    return join(process.cwd(), "src", "routes", ...segments, "+page.svelte");
}

describe("operator onboarding guide (T-28p)", () => {
    it("names at least the three required roles", () => {
        const ids = ONBOARDING_ROLES.map((role) => role.id);
        expect(ids).toEqual(
            expect.arrayContaining(["executive", "pmo", "resource-manager"]),
        );
    });

    it("gives every role at least one step", () => {
        for (const role of ONBOARDING_ROLES) {
            expect(role.steps.length, `${role.id} has no steps`).toBeGreaterThan(0);
        }
    });

    it("every step's route resolves to a real page in the route tree", () => {
        for (const role of ONBOARDING_ROLES) {
            for (const step of role.steps) {
                const path = pagePathFor(step.route);
                expect(
                    existsSync(path),
                    `${role.id}'s step "${step.label}" names ${step.route}, but no +page.svelte exists at ${path}`,
                ).toBe(true);
            }
        }
    });

    it("makes no time-to-productivity claim", () => {
        const banned = /\b\d+\s*(minute|hour|min|hr)s?\b/i;
        for (const role of ONBOARDING_ROLES) {
            expect(
                banned.test(role.summary),
                `${role.id}'s summary reads like a timing claim: "${role.summary}"`,
            ).toBe(false);
            for (const step of role.steps) {
                expect(
                    banned.test(step.note),
                    `${role.id}'s step "${step.label}" reads like a timing claim: "${step.note}"`,
                ).toBe(false);
            }
        }
    });
});
