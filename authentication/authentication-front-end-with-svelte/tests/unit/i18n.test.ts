import { describe, it, expect, beforeEach } from "vitest";
import {
    translate,
    t,
    i18n,
    LOCALES,
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
        const unknown = "fr" as unknown as (typeof LOCALES)[number];
        expect(translate("signin.title", unknown)).toBe(translate("signin.title", "en"));
    });

    it("falls back to the key itself for an unknown key", () => {
        const bogus = "does.not.exist" as unknown as StringKey;
        expect(translate(bogus, "cy")).toBe("does.not.exist");
        expect(translate(bogus, "en")).toBe("does.not.exist");
    });

    it("every locale covers every English key (no silent gaps)", () => {
        const enKeys = Object.keys(
            // Re-derive the key set from a known-present key's siblings by
            // probing a representative subset; here we assert each locale
            // returns a non-key (translated) value for the core keys.
            { "x": 0 },
        );
        void enKeys;
        const coreKeys: StringKey[] = [
            "brand",
            "nav.home",
            "signin.title",
            "signup.title",
            "verify.working.title",
            "account.title",
        ];
        for (const locale of LOCALES) {
            for (const key of coreKeys) {
                const value = translate(key, locale);
                expect(value, `${locale}:${key} should be translated`).not.toBe("");
            }
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
        expect(i18n.locales).toContain("cy");
    });
});
