// Unit tests for the dependency-free i18n catalog. `translate` is pure
// (it takes an explicit locale), so these run without a Svelte component
// or a browser. Mock $app/environment so importing the store off-browser
// doesn't touch localStorage.
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    DEFAULT_LOCALE,
    CATALOG,
    STRING_KEYS,
    isRtl,
    translate,
    type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    it("returns the English source strings", () => {
        expect(translate("workers.heading", "en")).toBe("Workers");
        expect(translate("nav.dashboard", "en")).toBe("Dashboard");
        expect(translate("common.search", "en")).toBe("Search");
    });

    it("returns Spanish strings for the same keys", () => {
        expect(translate("workers.heading", "es")).toBe("Trabajadores");
        expect(translate("nav.dashboard", "es")).toBe("Panel");
        expect(translate("common.search", "es")).toBe("Buscar");
    });

    it("uses the glossary translations across locales", () => {
        expect(translate("nav.matchCheck", "cy")).toBe("Gwiriad cydweddu");
        expect(translate("nav.merge", "de")).toBe("Zusammenführen");
        expect(translate("common.loading", "fr")).toBe("Chargement…");
    });

    it("falls back to English for an unsupported locale", () => {
        // `translate` is typed to a Locale, but a bad locale at runtime
        // (e.g. from storage) must degrade to the English table.
        const bad = "xx" as unknown as typeof DEFAULT_LOCALE;
        expect(translate("workers.heading", bad)).toBe(translate("workers.heading", "en"));
    });

    it("falls back to the key itself for an unknown key", () => {
        const missing = "does.not.exist" as unknown as StringKey;
        expect(translate(missing, "es")).toBe("does.not.exist");
    });

    it("every locale covers the full English key set", () => {
        const enKeys = new Set(STRING_KEYS);
        for (const locale of LOCALES) {
            const localeKeys = new Set(Object.keys(CATALOG[locale]));
            // Same cardinality + every en key present = full coverage.
            expect(localeKeys.size).toBe(enKeys.size);
            for (const key of STRING_KEYS) {
                expect(localeKeys.has(key)).toBe(true);
                expect(translate(key, locale).length).toBeGreaterThan(0);
            }
        }
    });

    it("ships all 13 locales", () => {
        expect(LOCALES).toHaveLength(13);
        expect([...LOCALES]).toEqual([
            "en", "cy", "es", "fr", "de", "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
        ]);
    });

    it("spot-checks new locale glossary translations", () => {
        expect(translate("nav.dashboard", "ar")).toBe("لوحة المعلومات");
        expect(translate("common.search", "ar")).toBe("بحث");
        expect(translate("nav.dashboard", "zh")).toBe("仪表板");
        expect(translate("common.search", "zh")).toBe("搜索");
    });

    it("marks ar and ur as RTL, others LTR", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
        // Region subtags are tolerated.
        expect(isRtl("ar-EG")).toBe(true);
    });
});
