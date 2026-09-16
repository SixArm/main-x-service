// Unit tests for the dependency-free i18n catalog + translate() fallbacks.
// Pins: correct per-locale strings, the locale → en → key fallback chain,
// and full key coverage across every supported locale.
import { describe, expect, it, vi } from "vitest";

// The i18n module reads `browser` from `$app/environment`; off the browser
// it seeds the default locale and skips localStorage.
vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    DEFAULT_LOCALE,
    LOCALE_LABELS,
    STRINGS,
    STRING_KEYS,
    translate,
    isRtl,
    i18n,
    type StringKey,
    type Locale,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    // English (source of truth) returns its literal strings.
    it("returns English strings for the default locale", () => {
        expect(translate("nav.dashboard", "en")).toBe("Dashboard");
        expect(translate("things.title", "en")).toBe("Things");
        expect(translate("nav.toggle", "en")).toBe("Toggle navigation");
    });

    // Spanish returns its translated strings (glossary-aligned where shared).
    it("returns Spanish strings for the es locale", () => {
        expect(translate("nav.dashboard", "es")).toBe("Panel");
        expect(translate("search.action", "es")).toBe("Buscar");
        expect(translate("merge.merge", "es")).toBe("Fusionar");
    });

    // An unknown locale falls back to the English table.
    it("falls back to English for an unknown locale", () => {
        // Force an unsupported locale through the type boundary.
        const bogus = "xx" as unknown as Locale;
        expect(translate("nav.dashboard", bogus)).toBe("Dashboard");
    });

    // An unknown key falls back to the key string itself (last resort).
    it("falls back to the key for an unknown key", () => {
        const missing = "does.not.exist" as unknown as StringKey;
        expect(translate(missing, "fr")).toBe("does.not.exist");
    });

    // DEFAULT_LOCALE is one of the supported locales.
    it("uses a supported default locale", () => {
        expect(LOCALES).toContain(DEFAULT_LOCALE);
    });

    // Every locale must natively cover the full English key set (no gaps).
    // Checked against the per-locale table directly so an `en`-fallback does
    // not mask a missing translation.
    it("covers every English key in every locale", () => {
        for (const locale of LOCALES) {
            const table = STRINGS[locale] as Record<string, string>;
            for (const key of STRING_KEYS) {
                expect(table[key], `${locale} missing ${key}`).toBeDefined();
                expect(table[key]!.length).toBeGreaterThan(0);
            }
        }
    });

    // The full fourteen-locale set is present after the PickerBar
    // consolidation added `en_US`.
    it("supports all fourteen locales", () => {
        expect(LOCALES).toHaveLength(14);
        for (const code of [
            "en", "en_US", "cy", "es", "fr", "de", "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
        ]) {
            expect(LOCALES).toContain(code as Locale);
        }
        expect(LOCALE_LABELS.en_US).toBe("English (United States)");
    });

    // `en_US`/`en-US` must resolve to the distinct `en_US` locale rather
    // than silently collapsing to `en`'s primary subtag.
    it("normalises en_US and en-US to the en_US locale rather than collapsing to en", () => {
        i18n.set("en_US");
        expect(i18n.locale).toBe("en_US");
        i18n.set("en-US");
        expect(i18n.locale).toBe("en_US");
        i18n.set("en");
        expect(i18n.locale).toBe("en");
    });

    // Spot-check two of the newly added locales against the shared glossary.
    it("returns Arabic strings for the ar locale", () => {
        expect(translate("nav.dashboard", "ar")).toBe("لوحة المعلومات");
        expect(translate("search.action", "ar")).toBe("بحث");
    });

    it("returns Chinese strings for the zh locale", () => {
        expect(translate("nav.merge", "zh")).toBe("合并");
        expect(translate("chrome.language", "zh")).toBe("语言");
    });

    // RTL detection: true for Arabic / Urdu, false otherwise.
    it("detects right-to-left locales", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
        // Region subtags are tolerated (only the primary subtag matters).
        expect(isRtl("ar-EG")).toBe(true);
    });
});
