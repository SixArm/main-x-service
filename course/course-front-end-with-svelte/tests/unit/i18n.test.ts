import { describe, it, expect, vi } from "vitest";

// The i18n store reads `browser` from `$app/environment` to decide whether
// to touch localStorage. Stub it false so the module is pure under test.
vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    DEFAULT_LOCALE,
    LOCALE_LABELS,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    translate,
    isRtl,
    i18n,
    type StringKey,
} from "../../src/lib/i18n.svelte.js";

describe("i18n catalog", () => {
    it("translates known keys for en-001 and es-001", () => {
        expect(translate("nav.dashboard", "en-001")).toBe("Dashboard");
        expect(translate("nav.merge", "en-001")).toBe("Merge");
        expect(translate("courses.title", "en-001")).toBe("Courses");

        expect(translate("nav.dashboard", "es-001")).toBe("Panel");
        expect(translate("nav.merge", "es-001")).toBe("Fusionar");
        expect(translate("courses.title", "es-001")).toBe("Cursos");
    });

    it("uses the agreed glossary translations across locales", () => {
        // Toggle navigation must stay verbatim (en) — a layout test asserts it.
        expect(translate("nav.toggle", "en-001")).toBe("Toggle navigation");
        expect(translate("nav.toggle", "cy-001")).toBe("Toglo'r llywio");
        expect(translate("nav.toggle", "fr-001")).toBe(
            "Basculer la navigation",
        );
        expect(translate("chrome.language", "cy-001")).toBe("Iaith");
        expect(translate("share.copy_link", "en-001")).toBe("Copy Link");
    });

    it("falls back locale → en-001 → key", () => {
        // @ts-expect-error — intentionally passing an unsupported locale.
        expect(translate("courses.title", "xx")).toBe("Courses");
        expect(translate("nonexistent.key" as StringKey, "en-001")).toBe(
            "nonexistent.key",
        );
        expect(DEFAULT_LOCALE).toBe("en-001");
    });

    it("offers exactly the eight expected locales, sorted by code", () => {
        expect(LOCALES.length).toBe(8);
        expect([...LOCALES]).toEqual([
            "ar-001",
            "cy-001",
            "de-de",
            "en-001",
            "es-001",
            "fr-001",
            "hi-001",
            "zh-cn",
        ]);
        expect(LOCALE_LABELS).toEqual({
            "ar-001": "العربية",
            "cy-001": "Cymraeg",
            "de-de": "Deutsch - Deutschland",
            "en-001": "English",
            "es-001": "Español",
            "fr-001": "Français",
            "hi-001": "हिन्दी",
            "zh-cn": "中文 - 中国",
        });
        for (const label of Object.values(LOCALE_LABELS)) {
            expect(label).not.toMatch(/[()]/);
        }
    });

    it("resolves regional and legacy codes by primary language", () => {
        i18n.set("en_US");
        expect(i18n.locale).toBe("en-001");
        i18n.set("es-MX");
        expect(i18n.locale).toBe("es-001");
        i18n.set("zh");
        expect(i18n.locale).toBe("zh-cn");
        i18n.set("zh-CN");
        expect(i18n.locale).toBe("zh-cn");
        i18n.set("de-DE");
        expect(i18n.locale).toBe("de-de");
        i18n.set("ja");
        expect(i18n.locale).toBe("en-001");
    });

    it("spot-checks locales against the glossary", () => {
        expect(translate("nav.dashboard", "ar-001")).toBe("لوحة المعلومات");
        expect(translate("nav.merge", "ar-001")).toBe("دمج");
        expect(translate("nav.dashboard", "zh-cn")).toBe("仪表板");
        expect(translate("nav.matchCheck", "zh-cn")).toBe("匹配检查");
    });

    it("marks ar-001 as RTL and the rest as LTR", () => {
        expect(isRtl("ar-001")).toBe(true);
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ar-EG")).toBe(true);
        expect(isRtl("en-001")).toBe(false);
        expect(isRtl("zh-cn")).toBe(false);
        expect(isRtl("hi-001")).toBe(false);
    });

    it("every locale covers the full en-001 key set", () => {
        expect(STRING_KEYS.length).toBeGreaterThan(0);
        for (const locale of LOCALES) {
            const table = STRINGS_BY_LOCALE[locale];
            for (const key of STRING_KEYS) {
                expect(
                    Object.prototype.hasOwnProperty.call(table, key),
                    `${locale} missing ${key}`,
                ).toBe(true);
                expect(typeof table[key], `${locale} ${key} not a string`).toBe(
                    "string",
                );
            }
            expect(Object.keys(table).length).toBe(STRING_KEYS.length);
        }
    });
});
