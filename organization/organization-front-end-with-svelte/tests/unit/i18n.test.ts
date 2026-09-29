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
  i18n,
  type Locale,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
  it("supports exactly the 7 required locales, sorted by code", () => {
    expect([...LOCALES]).toEqual([
      "ar-001",
      "cy-001",
      "en-001",
      "es-001",
      "fr-001",
      "hi-001",
      "zh-cn",
    ]);
    expect(LOCALES.length).toBe(7);
  });

  it("has a human-readable label for every locale", () => {
    for (const locale of LOCALES) {
      expect(LOCALE_LABELS[locale]).toBeTruthy();
    }
  });

  it("every locale covers every key (full 7-locale coverage)", () => {
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
    expect(DEFAULT_LOCALE).toBe("en-001");
  });

  it("spot-checks a non-Latin locale (Chinese)", () => {
    expect(translate("nav.organizations", "zh-cn")).toBe("组织");
    expect(translate("form.save", "zh-cn")).toBe("保存");
  });

  it("spot-checks a right-to-left locale (Arabic)", () => {
    expect(translate("nav.organizations", "ar-001")).toBe("المنظمات");
  });

  it("falls back to English then to the key", () => {
    // A locale not present falls back to English.
    expect(translate("form.save", "xx" as unknown as Locale)).toBe(
      translate("form.save", "en-001"),
    );
  });

  it("labels -001 locales by language only and never uses parentheses", () => {
    for (const [code, label] of Object.entries(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
      if (code.endsWith("-001")) expect(label).not.toContain(" - ");
    }
    expect(LOCALE_LABELS["zh-cn"]).toBe("中文 - 中国");
  });

  it("isRtl is true for ar-001 only", () => {
    expect(isRtl("ar-001")).toBe(true);
    // Region subtags / bare primary subtags are tolerated.
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("ar-EG")).toBe(true);
    for (const locale of LOCALES) {
      if (locale !== "ar-001") expect(isRtl(locale)).toBe(false);
    }
  });

  it("resolves bare and regional codes to the supported locale", () => {
    i18n.set("es-MX");
    expect(i18n.locale).toBe("es-001");
    i18n.set("zh");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("zh_CN");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("en-US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("xx");
    expect(i18n.locale).toBe("en-001");
  });

  it("has the English Copy Link label and the splash copy in every locale", () => {
    expect(translate("share.copy_link", "en-001")).toBe("Copy Link");
    for (const locale of LOCALES) {
      expect(translate("splash.hero.title", locale)).toBeTruthy();
    }
  });
});
