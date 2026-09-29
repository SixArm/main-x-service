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
  it("supports exactly the seven expected locales, sorted by code", () => {
    expect([...LOCALES]).toEqual([
      "ar-001",
      "cy-001",
      "en-001",
      "es-001",
      "fr-001",
      "hi-001",
      "zh-cn",
    ]);
    expect(LOCALE_LABELS).toEqual({
      "ar-001": "العربية",
      "cy-001": "Cymraeg",
      "en-001": "English",
      "es-001": "Español",
      "fr-001": "Français",
      "hi-001": "हिन्दी",
      "zh-cn": "中文 - 中国",
    });
  });

  it("labels never use parentheses", () => {
    for (const label of Object.values(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
    }
  });

  it("has a human-readable label for every locale", () => {
    for (const locale of LOCALES) {
      expect(LOCALE_LABELS[locale]).toBeTruthy();
    }
  });

  it("every locale covers every key (all seven locales)", () => {
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
    expect(translate("nav.cases", "zh-cn")).toBe("案件");
    expect(translate("form.save", "zh-cn")).toBe("保存");
  });

  it("spot-checks a right-to-left locale (Arabic)", () => {
    expect(translate("nav.cases", "ar-001")).toBe("القضايا");
  });

  it("covers the merge page keys in every locale", () => {
    const mergeKeys = STRING_KEYS.filter(
      (k) => k.startsWith("merge.") || k === "nav.merge",
    );
    // The merge UI is a whole page; a partial catalog would render
    // raw keys, so assert the block exists rather than a single key.
    expect(mergeKeys.length).toBeGreaterThan(20);
    for (const locale of LOCALES) {
      for (const key of mergeKeys) {
        expect(
          STRINGS_BY_LOCALE[locale][key],
          `${locale} missing ${key}`,
        ).toBeTruthy();
      }
    }
    // The confirm prompt keeps both placeholders the page substitutes.
    for (const locale of LOCALES) {
      const confirm = translate("merge.confirm", locale);
      expect(confirm, `${locale} confirm missing {dup}`).toContain("{dup}");
      expect(confirm, `${locale} confirm missing {main}`).toContain("{main}");
    }
  });

  it("covers the cross-service links panel keys in every locale", () => {
    const linkKeys = STRING_KEYS.filter((k) => k.startsWith("links."));
    // The panel is a whole section of the detail route; a partial
    // catalog would render raw keys next to sensitive data.
    expect(linkKeys.length).toBeGreaterThan(20);
    for (const locale of LOCALES) {
      for (const key of linkKeys) {
        expect(
          STRINGS_BY_LOCALE[locale][key],
          `${locale} missing ${key}`,
        ).toBeTruthy();
      }
    }
    // The withdraw prompt keeps the placeholder the panel substitutes;
    // losing it would confirm a sensitive withdrawal without naming
    // which person reference is being retracted.
    for (const locale of LOCALES) {
      expect(
        translate("links.withdrawConfirm", locale),
        `${locale} withdraw confirm missing {ref}`,
      ).toContain("{ref}");
    }
  });

  it("falls back to English then to the key", () => {
    // A locale not present falls back to English.
    expect(translate("form.save", "xx" as unknown as Locale)).toBe(
      translate("form.save", "en-001"),
    );
  });

  it("isRtl is true only for ar-001", () => {
    expect(isRtl("ar-001")).toBe(true);
    // Region subtags and legacy codes are tolerated.
    expect(isRtl("ar-EG")).toBe(true);
    expect(isRtl("ar")).toBe(true);
    for (const locale of LOCALES) {
      if (locale !== "ar-001") {
        expect(isRtl(locale)).toBe(false);
      }
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
    i18n.set("de");
    expect(i18n.locale).toBe("en-001");
  });

  it("has exactly six tiles in each splash area", () => {
    for (const area of ["benefits", "features", "trust"]) {
      const titles = STRING_KEYS.filter((k) =>
        new RegExp(`^splash\\.${area}\\.\\d+\\.title$`).test(k),
      );
      expect(titles).toHaveLength(6);
    }
  });
});
