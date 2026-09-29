// The public /tour page: its copy lives in the i18n catalog, so pin that
// all six workflow sections carry the full set of keys in every locale and
// that each "Open this screen" target is a real route file.
import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    translate,
    type StringKey,
} from "../../src/lib/i18n.svelte";

const KEYS = [
    "nav.tour",
    "splash.hero.tour",
    "tour.head",
    "tour.intro",
    "tour.toc",
    "tour.open",
    "tour.top",
    ...["start", "s1", "s2", "s3", "s4", "s5", "s6"].flatMap((s) => [
        `tour.${s}.title`,
        `tour.${s}.summary`,
        ...[1, 2, 3, 4].map((n) => `tour.${s}.step.${n}`),
    ]),
] as StringKey[];

describe("tour catalog", () => {
    for (const locale of LOCALES) {
        it(`${locale} has every tour key, non-empty`, () => {
            for (const k of KEYS) {
                const v = translate(k, locale);
                expect(v, `${locale} ${k}`).toBeTruthy();
                expect(v, `${locale} ${k}`).not.toBe(k);
            }
        });
    }
});

describe("tour route", () => {
    const page = readFileSync("src/routes/tour/+page.svelte", "utf8");
    const hrefs = [...page.matchAll(/href: "([^"]+)"/g)].map((m) => m[1]);

    it("declares six sections", () => {
        expect(hrefs).toHaveLength(6);
    });

    it("links each section to an existing route", () => {
        for (const h of hrefs) {
            const file = `src/routes${h === "/" ? "" : h}/+page.svelte`;
            expect(existsSync(file), h).toBe(true);
        }
    });

    it("is public: the server load does not require a session", () => {
        const server = readFileSync("src/routes/tour/+page.server.ts", "utf8");
        expect(server).not.toContain("requireSignedIn");
    });
});
