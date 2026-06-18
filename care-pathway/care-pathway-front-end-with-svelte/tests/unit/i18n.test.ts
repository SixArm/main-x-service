import { describe, it, expect, vi } from "vitest";

// The i18n module reads `$app/environment`'s `browser`; stub it to the SSR
// value so the module loads under jsdom without a real SvelteKit runtime.
vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    LOCALE_LABELS,
    isRtl,
    translate,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    it("supports exactly the 13 family locales", () => {
        expect([...LOCALES]).toEqual([
            "en",
            "cy",
            "es",
            "fr",
            "de",
            "ar",
            "ru",
            "hi",
            "zh",
            "bn",
            "pt",
            "id",
            "ur",
        ]);
    });

    it("has full coverage: every key in all 13 locales", () => {
        expect(STRING_KEYS.length).toBeGreaterThan(0);
        for (const locale of LOCALES) {
            const table = STRINGS_BY_LOCALE[locale];
            for (const key of STRING_KEYS) {
                expect(table[key], `${locale} missing ${key}`).toBeTruthy();
            }
            // No extra keys beyond the English source-of-truth set.
            expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
        }
    });

    it("has a human label for every locale", () => {
        for (const locale of LOCALES) {
            expect(LOCALE_LABELS[locale]).toBeTruthy();
        }
    });

    it("spot-checks a non-Latin locale (zh)", () => {
        expect(translate("list.title", "zh")).toBe("护理路径");
        expect(translate("form.save", "zh")).toBe("保存");
    });

    it("marks ar and ur as RTL, others LTR", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("ar-EG")).toBe(true); // region subtag tolerated
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
        expect(isRtl("fr")).toBe(false);
    });

    it("falls back to English for a missing target translation", () => {
        // An unknown locale falls back to the English table.
        expect(translate("list.title", "xx" as never)).toBe(translate("list.title", "en"));
    });
});
