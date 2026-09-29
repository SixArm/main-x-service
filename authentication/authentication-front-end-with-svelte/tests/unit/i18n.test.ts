// Pins the i18n catalog + reactive locale store: en-001/cy-001 lookups, the
// fallback chain (unknown locale → en-001, unknown key → the key itself),
// that every one of the 8 locales covers every English key, a spot-check
// of a non-Latin locale, RTL detection for ar-001, and that the reactive
// `i18n` store reflects switches, reduces region and legacy codes (cy-GB → cy-001), and
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
    expect(translate("signin.title", "en-001")).toBe("Sign in");
    expect(translate("signup.title", "en-001")).toBe("Create account");
    expect(translate("account.signout", "en-001")).toBe("Sign out");
  });

  it("returns the Welsh string for a known key", () => {
    expect(translate("signin.title", "cy-001")).toBe("Mewngofnodi");
    expect(translate("signup.title", "cy-001")).toBe("Creu cyfrif");
    expect(translate("account.signout", "cy-001")).toBe("Allgofnodi");
  });

  it("falls back to English for an unknown locale", () => {
    // Cast through unknown: an unsupported locale must still resolve.
    const unknown = "zz" as unknown as (typeof LOCALES)[number];
    expect(translate("signin.title", unknown)).toBe(
      translate("signin.title", "en-001"),
    );
  });

  it("falls back to the key itself for an unknown key", () => {
    const bogus = "does.not.exist" as unknown as StringKey;
    expect(translate(bogus, "cy-001")).toBe("does.not.exist");
    expect(translate(bogus, "en-001")).toBe("does.not.exist");
  });

  it("supports exactly the 8 expected locales, sorted by code, with endonym labels", () => {
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

  it("every one of the 8 locales covers every English key (full coverage)", () => {
    expect(LOCALES.length).toBe(8);
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
    expect(translate("nav.signin", "ar-001")).toBe("تسجيل الدخول");
    expect(translate("signup.title", "ar-001")).toBe("إنشاء حساب");
    // Hindi too, to cover a Devanagari script.
    expect(translate("nav.home", "hi-001")).toBe("होम");
  });
});

describe("i18n RTL", () => {
  it("marks ar-001 as right-to-left", () => {
    expect(isRtl("ar-001")).toBe(true);
    // Region subtags and legacy codes reduce to their primary language.
    expect(isRtl("ar-EG")).toBe(true);
    expect(isRtl("ar")).toBe(true);
    expect([...RTL_LOCALES]).toEqual(["ar-001"]);
  });

  it("marks every other locale as left-to-right", () => {
    for (const locale of LOCALES) {
      if (locale === "ar-001") continue;
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
    i18n.set("cy-001");
    expect(i18n.locale).toBe("cy-001");
    expect(t("signin.title")).toBe("Mewngofnodi");
  });

  it("set() reduces a region subtag or legacy code to the supported locale", () => {
    i18n.set("cy-GB");
    expect(i18n.locale).toBe("cy-001");
    i18n.set("en_US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("zh");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("zh-CN");
    expect(i18n.locale).toBe("zh-cn");
  });

  it("set() falls back to the default for an unsupported locale", () => {
    i18n.set("cy-001");
    i18n.set("zz");
    expect(i18n.locale).toBe(DEFAULT_LOCALE);
  });

  it("exposes the supported locale list", () => {
    expect(i18n.locales).toEqual(LOCALES);
    expect(i18n.locales).toContain("en-001");
    expect(i18n.locales).toContain("ar-001");
    expect(i18n.locales).toContain("zh-cn");
  });
});
