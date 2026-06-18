import { describe, it, expect, vi } from "vitest";

// The i18n store reads `browser` from `$app/environment` to decide whether
// to touch localStorage. Stub it false so the module is pure under test.
vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    DEFAULT_LOCALE,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    translate,
    isRtl,
    type StringKey,
} from "../../src/lib/i18n.svelte.js";

describe("i18n catalog", () => {
    it("translates known keys for en and es", () => {
        expect(translate("nav.dashboard", "en")).toBe("Dashboard");
        expect(translate("nav.merge", "en")).toBe("Merge");
        expect(translate("courses.title", "en")).toBe("Courses");

        expect(translate("nav.dashboard", "es")).toBe("Panel");
        expect(translate("nav.merge", "es")).toBe("Fusionar");
        expect(translate("courses.title", "es")).toBe("Cursos");
    });

    it("uses the agreed glossary translations across locales", () => {
        // Toggle navigation must stay verbatim (en) — a layout test asserts it.
        expect(translate("nav.toggle", "en")).toBe("Toggle navigation");
        expect(translate("nav.toggle", "cy")).toBe("Toglo'r llywio");
        expect(translate("nav.toggle", "fr")).toBe("Basculer la navigation");
        expect(translate("nav.dashboard", "de")).toBe("Übersicht");
        expect(translate("chrome.language", "cy")).toBe("Iaith");
    });

    it("falls back locale → en → key", () => {
        // Unknown locale falls back to the English table.
        // @ts-expect-error — intentionally passing an unsupported locale.
        expect(translate("courses.title", "xx")).toBe("Courses");
        // Unknown key falls back to the key string itself.
        expect(translate("nonexistent.key" as StringKey, "en")).toBe("nonexistent.key");
        expect(DEFAULT_LOCALE).toBe("en");
    });

    it("offers all 13 locales", () => {
        expect(LOCALES.length).toBe(13);
        expect([...LOCALES]).toEqual([
            "en", "cy", "es", "fr", "de",
            "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
        ]);
    });

    it("spot-checks new locales against the glossary", () => {
        expect(translate("nav.dashboard", "ar")).toBe("لوحة المعلومات");
        expect(translate("nav.merge", "ar")).toBe("دمج");
        expect(translate("nav.dashboard", "zh")).toBe("仪表板");
        expect(translate("nav.matchCheck", "zh")).toBe("匹配检查");
        expect(translate("chrome.language", "ru")).toBe("Язык");
    });

    it("marks ar/ur as RTL and the rest as LTR", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("ar-EG")).toBe(true);
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
        expect(isRtl("hi")).toBe(false);
    });

    it("every locale covers the full en key set", () => {
        expect(STRING_KEYS.length).toBeGreaterThan(0);
        for (const locale of LOCALES) {
            const table = STRINGS_BY_LOCALE[locale];
            for (const key of STRING_KEYS) {
                expect(
                    Object.prototype.hasOwnProperty.call(table, key),
                    `${locale} missing ${key}`,
                ).toBe(true);
                expect(typeof table[key], `${locale} ${key} not a string`).toBe("string");
            }
            // No extra keys beyond the en universe.
            expect(Object.keys(table).length).toBe(STRING_KEYS.length);
        }
    });
});
