import { describe, it, expect, vi } from "vitest";

// The i18n module reads `$app/environment`'s `browser`; stub it to the SSR
// value so the module loads under jsdom without a real SvelteKit runtime.
vi.mock("$app/environment", () => ({ browser: false }));

import {
  LOCALES,
  STRING_KEYS,
  STRINGS_BY_LOCALE,
  LOCALE_LABELS,
  DEFAULT_LOCALE,
  isRtl,
  translate,
  i18n,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
  it("supports exactly the 7 family locales, sorted by code", () => {
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
    expect(DEFAULT_LOCALE).toBe("en-001");
  });

  it("has full coverage: every key in all 7 locales", () => {
    expect(STRING_KEYS.length).toBeGreaterThan(0);
    for (const locale of LOCALES) {
      const table = STRINGS_BY_LOCALE[locale];
      for (const key of STRING_KEYS) {
        expect(table[key], `${locale} missing ${key}`).toBeTruthy();
      }
      // No extra keys beyond the English source-of-truth set.
      expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
    }
  });

  it("has a human label for every locale", () => {
    for (const locale of LOCALES) {
      expect(LOCALE_LABELS[locale]).toBeTruthy();
    }
  });

  it("labels -001 locales by language only, never with parentheses", () => {
    for (const [code, label] of Object.entries(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
      if (code.endsWith("-001")) expect(label).not.toContain(" - ");
    }
    expect(LOCALE_LABELS["zh-cn"]).toBe("中文 - 中国");
  });

  it("spot-checks a non-Latin locale (zh-cn)", () => {
    expect(translate("list.title", "zh-cn")).toBe("护理路径");
    expect(translate("form.save", "zh-cn")).toBe("保存");
  });

  it("marks ar-001 as RTL, others LTR", () => {
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("ar-EG")).toBe(true); // region subtag tolerated
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
    expect(isRtl("fr-001")).toBe(false);
  });

  it("falls back to English for a missing target translation", () => {
    // An unknown locale falls back to the English table.
    expect(translate("list.title", "xx" as never)).toBe(
      translate("list.title", "en-001"),
    );
  });

  it("uses Title Case for the copy-link label", () => {
    expect(translate("share.copyLink", "en-001")).toBe("Copy Link");
  });

  it("normalises legacy and regional codes to the supported locale of that language", () => {
    i18n.set("en_US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("es-MX");
    expect(i18n.locale).toBe("es-001");
    i18n.set("zh");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("ZH-CN");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("de-DE");
    expect(i18n.locale).toBe("de-de");
    i18n.set("ja");
    expect(i18n.locale).toBe("en-001");
  });
});

// /tour is a public walkthrough: every locale must carry the intro, six
// workflow sections of exactly four steps, and the generic opener, so the
// page never renders a raw key.
describe("tour catalog", () => {
  const keys = [
    "nav.tour",
    "splash.hero.tour",
    "tour.head",
    "tour.toc",
    "tour.open",
    "tour.top",
    "tour.intro",
    ...["start", "s1", "s2", "s3", "s4", "s5", "s6"].flatMap((s) => [
      `tour.${s}.title`,
      `tour.${s}.summary`,
      ...[1, 2, 3, 4].map((n) => `tour.${s}.step.${n}`),
    ]),
  ];
  it("has every tour key non-empty in all 7 locales", () => {
    for (const locale of LOCALES) {
      for (const k of keys) {
        const v = (STRINGS_BY_LOCALE[locale] as Record<string, string>)[k];
        expect(v, `${locale} ${k}`).toBeTruthy();
      }
    }
  });
});
