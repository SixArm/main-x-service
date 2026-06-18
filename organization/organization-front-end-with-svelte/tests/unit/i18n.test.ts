import { describe, it, expect, vi } from "vitest";

// The i18n store seeds from localStorage behind `$app/environment`'s
// `browser`; stub it to the server value so the module is deterministic.
vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    LOCALE_LABELS,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    DEFAULT_LOCALE,
    translate,
    isRtl,
    type Locale,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    it("supports exactly the 13 required locales", () => {
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

    it("has a human-readable label for every locale", () => {
        for (const locale of LOCALES) {
            expect(LOCALE_LABELS[locale]).toBeTruthy();
        }
    });

    it("every locale covers every key (full 13-locale coverage)", () => {
        for (const locale of LOCALES) {
            const table = STRINGS_BY_LOCALE[locale];
            for (const key of STRING_KEYS) {
                expect(table[key], `${locale} missing ${key}`).toBeTruthy();
            }
            // No extra keys beyond the English source of truth.
            expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
        }
    });

    it("default locale is English", () => {
        expect(DEFAULT_LOCALE).toBe("en");
    });

    it("spot-checks a non-Latin locale (Chinese)", () => {
        expect(translate("nav.organizations", "zh")).toBe("组织");
        expect(translate("form.save", "zh")).toBe("保存");
    });

    it("spot-checks a right-to-left locale (Arabic)", () => {
        expect(translate("nav.organizations", "ar")).toBe("المنظمات");
    });

    it("falls back to English then to the key", () => {
        // A locale not present falls back to English.
        expect(translate("form.save", "xx" as unknown as Locale)).toBe(
            translate("form.save", "en"),
        );
    });

    it("isRtl is true for ar and ur, false otherwise", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        // Region subtags are tolerated.
        expect(isRtl("ar-EG")).toBe(true);
        for (const locale of LOCALES) {
            if (locale !== "ar" && locale !== "ur") {
                expect(isRtl(locale)).toBe(false);
            }
        }
    });
});
