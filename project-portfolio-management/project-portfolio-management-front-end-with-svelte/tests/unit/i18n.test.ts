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
  it("supports exactly the seven required locales, sorted by code", () => {
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

  it("labels never use parentheses, and -001 locales are language-only", () => {
    for (const [code, label] of Object.entries(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
      if (code.endsWith("-001")) expect(label).not.toContain(" - ");
    }
  });

  it("normalises legacy and regional codes to the supported locale of that language", () => {
    i18n.set("en_US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("en-US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("es-MX");
    expect(i18n.locale).toBe("es-001");
    i18n.set("zh-cn");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("ZH-CN");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("de");
    expect(i18n.locale).toBe("en-001");
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

  it("spot-checks the PPM keys in non-Latin locales", () => {
    expect(translate("ppm.dashboard.title", "zh-cn")).toBe("组合仪表盘");
    expect(translate("ppm.gov.title", "ar-001")).toBe("الحوكمة");
    expect(translate("ppm.nav.proposals", "hi-001")).toBe("प्रस्ताव");
  });

  // The merge page's keys: present in English, genuinely translated
  // elsewhere (not English copies), and carrying both placeholders in
  // the confirmation prompt so the substitution cannot silently break.
  it("covers the merge keys in every locale", () => {
    const mergeKeys = STRING_KEYS.filter(
      (key) => key === "nav.merge" || key.startsWith("merge."),
    );
    expect(mergeKeys.length).toBeGreaterThanOrEqual(24);
    for (const locale of LOCALES) {
      for (const key of mergeKeys) {
        expect(translate(key, locale), `${locale} missing ${key}`).toBeTruthy();
      }
      const confirm = translate("merge.confirm", locale);
      expect(confirm, `${locale} merge.confirm {dup}`).toContain("{dup}");
      expect(confirm, `${locale} merge.confirm {main}`).toContain("{main}");
    }
    expect(translate("merge.title", "fr-001")).toBe("Fusionner des plans");
    expect(translate("merge.title", "zh-cn")).toBe("合并计划");
    expect(translate("nav.merge", "ar-001")).toBe("دمج");
  });

  it("spot-checks a non-Latin locale (Chinese)", () => {
    expect(translate("nav.cases", "zh-cn")).toBe("案件");
    expect(translate("form.save", "zh-cn")).toBe("保存");
  });

  it("spot-checks a right-to-left locale (Arabic)", () => {
    expect(translate("nav.cases", "ar-001")).toBe("القضايا");
  });

  it("falls back to English then to the key", () => {
    // A locale not present falls back to English.
    expect(translate("form.save", "xx" as unknown as Locale)).toBe(
      translate("form.save", "en-001"),
    );
  });

  it("isRtl is true for ar-001 only", () => {
    expect(isRtl("ar-001")).toBe(true);
    // Legacy / region subtags are tolerated.
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("ar-EG")).toBe(true);
    for (const locale of LOCALES) {
      if (locale !== "ar-001") expect(isRtl(locale)).toBe(false);
    }
  });

  it("has the splash copy in every locale (six benefits and six features)", () => {
    for (const locale of LOCALES) {
      for (const area of ["benefits", "features", "trust"]) {
        for (let n = 1; n <= 6; n++) {
          for (const part of ["title", "body"]) {
            const key = `splash.${area}.${n}.${part}` as never;
            expect(translate(key, locale), `${locale} ${key}`).not.toBe(key);
          }
        }
      }
    }
  });
});
