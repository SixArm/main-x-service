// Pins the i18n catalog + reactive locale store: en/cy lookups, the
// fallback chain (unknown locale → en, unknown key → the key itself),
// that every one of the 13 locales covers every English key, a spot-check
// of a non-Latin locale, RTL detection for ar/ur, and that the reactive
// `i18n` store reflects switches, reduces region subtags (cy-GB → cy), and
// falls back to the default for an unsupported locale.
import { describe, it, expect, beforeEach } from "vitest";
import {
    translate,
    t,
    i18n,
    isRtl,
    LOCALES,
    LOCALE_LABELS,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    RTL_LOCALES,
    DEFAULT_LOCALE,
    type StringKey,
} from "$lib/i18n.svelte";

describe("i18n catalog", () => {
    beforeEach(() => {
        // Reset to the default locale between tests (state is module-global).
        i18n.set(DEFAULT_LOCALE);
    });

    it("returns the English string for a known key", () => {
        expect(translate("signin.title", "en")).toBe("Sign in");
        expect(translate("signup.title", "en")).toBe("Create account");
        expect(translate("account.signout", "en")).toBe("Sign out");
    });

    it("returns the Welsh string for a known key", () => {
        expect(translate("signin.title", "cy")).toBe("Mewngofnodi");
        expect(translate("signup.title", "cy")).toBe("Creu cyfrif");
        expect(translate("account.signout", "cy")).toBe("Allgofnodi");
    });

    it("falls back to English for an unknown locale", () => {
        // Cast through unknown: an unsupported locale must still resolve.
        const unknown = "zz" as unknown as (typeof LOCALES)[number];
        expect(translate("signin.title", unknown)).toBe(translate("signin.title", "en"));
    });

    it("falls back to the key itself for an unknown key", () => {
        const bogus = "does.not.exist" as unknown as StringKey;
        expect(translate(bogus, "cy")).toBe("does.not.exist");
        expect(translate(bogus, "en")).toBe("does.not.exist");
    });

    it("supports exactly the 13 expected locales with endonym labels", () => {
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
        for (const locale of LOCALES) {
            expect(LOCALE_LABELS[locale], `${locale} needs an endonym label`).toBeTruthy();
        }
    });

    it("every one of the 13 locales covers every English key (full coverage)", () => {
        expect(LOCALES.length).toBe(13);
        for (const locale of LOCALES) {
            const table = STRINGS_BY_LOCALE[locale];
            for (const key of STRING_KEYS) {
                const value = table[key];
                expect(value, `${locale}:${key} should be present`).toBeTruthy();
            }
            // No stray keys beyond the English source-of-truth set.
            expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
        }
    });

    it("spot-checks a non-Latin locale (Arabic)", () => {
        expect(translate("nav.signin", "ar")).toBe("تسجيل الدخول");
        expect(translate("signup.title", "ar")).toBe("إنشاء حساب");
        // Hindi too, to cover a Devanagari script.
        expect(translate("nav.home", "hi")).toBe("होम");
    });
});

describe("i18n RTL", () => {
    it("marks ar and ur as right-to-left", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        // Region subtags reduce to their primary language.
        expect(isRtl("ar-EG")).toBe(true);
        expect([...RTL_LOCALES].sort()).toEqual(["ar", "ur"]);
    });

    it("marks every other locale as left-to-right", () => {
        for (const locale of LOCALES) {
            if (locale === "ar" || locale === "ur") continue;
            expect(isRtl(locale), `${locale} should be ltr`).toBe(false);
        }
    });
});

describe("i18n reactive locale", () => {
    beforeEach(() => {
        i18n.set(DEFAULT_LOCALE);
    });

    it("t() reflects the current locale", () => {
        expect(t("signin.title")).toBe("Sign in");
        i18n.set("cy");
        expect(i18n.locale).toBe("cy");
        expect(t("signin.title")).toBe("Mewngofnodi");
    });

    it("set() reduces a region subtag to its primary language", () => {
        i18n.set("cy-GB");
        expect(i18n.locale).toBe("cy");
    });

    it("set() falls back to the default for an unsupported locale", () => {
        i18n.set("cy");
        i18n.set("zz");
        expect(i18n.locale).toBe(DEFAULT_LOCALE);
    });

    it("exposes the supported locale list", () => {
        expect(i18n.locales).toEqual(LOCALES);
        expect(i18n.locales).toContain("en");
        expect(i18n.locales).toContain("ar");
        expect(i18n.locales).toContain("zh");
    });
});
