// The /tour page: every workflow section has copy in all locales, and the
// component renders four steps per section with no raw keys.
import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { existsSync } from "node:fs";

vi.mock("$app/environment", () => ({ browser: false }));

import Tour from "../../src/lib/components/Tour.svelte";
import { LOCALES, STRINGS } from "../../src/lib/i18n.svelte";

afterEach(cleanup);

const HREFS = ["/things/new", "/things", "/things/match", "/review", "/things/merge", "/things"];

describe("Tour", () => {
    it("renders the opener plus six sections of four steps, no raw keys", () => {
        const { container } = render(Tour, { sections: HREFS.map((href) => ({ href })) });
        expect(container.querySelectorAll("section[id^=s], section#start").length).toBe(7);
        expect(container.querySelectorAll(".steps").length).toBe(7);
        expect(container.querySelectorAll(".steps li").length).toBe(28);
        expect(container.textContent).not.toMatch(/\btour\.[a-z0-9.]+/);
        expect(container.querySelectorAll("a.open").length).toBe(6);
    });

    it("has every tour key in every locale", () => {
        const keys = ["tour.intro", "tour.head", "nav.tour", "splash.hero.tour"];
        for (let n = 1; n <= 6; n++) {
            keys.push(`tour.s${n}.title`, `tour.s${n}.summary`);
            for (let s = 1; s <= 4; s++) keys.push(`tour.s${n}.step.${s}`);
        }
        for (const l of LOCALES) {
            for (const k of keys) {
                expect((STRINGS as unknown as Record<string, Record<string, string>>)[l]![k], `${l} ${k}`).toBeTruthy();
            }
        }
    });

    it("links only to screens that exist", () => {
        for (const href of new Set(HREFS)) {
            const dir = href === "/review" ? "review" : href.slice(1);
            expect(existsSync(`src/routes/${dir}/+page.svelte`), href).toBe(true);
        }
    });
});
