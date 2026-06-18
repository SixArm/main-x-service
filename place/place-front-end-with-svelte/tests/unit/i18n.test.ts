// Unit tests for the dependency-free i18n store: per-locale lookup, the
// locale → en → key fallback chain, and full key coverage across every
// supported locale (so a missing translation is caught here, not in prod).
import { describe, expect, it, vi } from "vitest";

// The store imports `browser` from $app/environment; off-browser keeps it
// from touching localStorage during the test.
vi.mock("$app/environment", () => ({ browser: false }));

import {
    translate,
    isRtl,
    LOCALES,
    DEFAULT_LOCALE,
    STRING_KEYS,
    STRINGS_FOR_TEST,
    type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n translate()", () => {
    it("returns the English source strings", () => {
        expect(translate("nav.dashboard", "en")).toBe("Dashboard");
        expect(translate("places.title", "en")).toBe("Places");
        expect(translate("form.save", "en")).toBe("Save");
    });

    it("returns the Spanish translations", () => {
        expect(translate("nav.dashboard", "es")).toBe("Panel");
        expect(translate("places.title", "es")).toBe("Lugares");
        expect(translate("form.save", "es")).toBe("Guardar");
    });

    it("uses the glossary translations consistently", () => {
        // Glossary terms must match verbatim across the family.
        expect(translate("nav.matchCheck", "cy")).toBe("Gwiriad cydweddu");
        expect(translate("nav.merge", "fr")).toBe("Fusionner");
        expect(translate("nav.dashboard", "de")).toBe("Übersicht");
        expect(translate("nav.toggle", "en")).toBe("Toggle navigation");
    });

    it("falls back to English for an unknown locale", () => {
        // An unsupported locale code resolves to the English table.
        expect(translate("places.title", "xx" as never)).toBe("Places");
    });

    it("falls back to the key itself for an unknown key", () => {
        expect(translate("does.not.exist" as StringKey, "en")).toBe("does.not.exist");
    });

    it("every locale defines every English key", () => {
        // The English catalog is the source of truth; every other locale must
        // define the exact same key set (no missing, no extra).
        const enKeys = [...STRING_KEYS].sort();
        for (const locale of LOCALES) {
            const localeKeys = Object.keys(STRINGS_FOR_TEST[locale]).sort();
            expect(localeKeys, `${locale} key set differs from en`).toEqual(enKeys);
        }
    });

    it("every locale returns a non-empty translation for every key", () => {
        for (const locale of LOCALES) {
            for (const key of STRING_KEYS) {
                const value = translate(key as StringKey, locale);
                expect(value.length, `${locale} empty ${key}`).toBeGreaterThan(0);
            }
        }
    });

    it("DEFAULT_LOCALE is English", () => {
        expect(DEFAULT_LOCALE).toBe("en");
    });

    it("supports all 13 locales", () => {
        expect(LOCALES).toHaveLength(13);
        expect([...LOCALES]).toEqual([
            "en", "cy", "es", "fr", "de", "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
        ]);
    });

    it("spot-checks the new Arabic translations", () => {
        expect(translate("nav.dashboard", "ar")).toBe("لوحة المعلومات");
        expect(translate("nav.merge", "ar")).toBe("دمج");
    });

    it("spot-checks the new Chinese translations", () => {
        expect(translate("nav.dashboard", "zh")).toBe("仪表板");
        expect(translate("form.save", "zh")).toBe("保存");
    });

    it("isRtl is true for ar/ur and false for en", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
    });
});
