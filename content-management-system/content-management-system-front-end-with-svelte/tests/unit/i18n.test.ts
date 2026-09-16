// The parity property: every locale covers the same key set.
//
// A missing key does not crash — it silently falls back to English
// inside an otherwise-translated page, which a reader experiences as a
// bug in the *content*. That is exactly the kind of defect a test
// catches for free and a person never reports.

import { describe, expect, it } from "vitest";
import {
  DEFAULT_LOCALE,
  LOCALES,
  LOCALE_LABELS,
  MESSAGE_KEYS,
  RTL_LOCALES,
  isRtl,
  normaliseLocale,
  translate,
} from "$lib/i18n.svelte";

describe("i18n", () => {
  it("offers fourteen locales, each with a label written in itself", () => {
    expect(LOCALES.length).toBe(14);
    for (const locale of LOCALES) {
      expect(LOCALE_LABELS[locale]?.length ?? 0).toBeGreaterThan(0);
    }
  });

  it("translates every key in every locale", () => {
    const missing: string[] = [];
    for (const locale of LOCALES) {
      for (const key of MESSAGE_KEYS) {
        const value = translate(locale, key);
        if (!value || value.trim().length === 0)
          missing.push(`${locale}:${key}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("actually translates rather than echoing English", () => {
    // A locale that returns the English string for every key has not
    // been translated — it has been declared. Brand and product names
    // are legitimately identical, so the check is on the rest.
    //
    // `en_US` is excluded too: it is a deliberate verbatim duplicate of
    // `en` (a distinct endonym for the locale picker, since `en`'s own
    // copy is already American-spelled), not a spelling fork, so it is
    // expected to repeat English for every key.
    const translatable = MESSAGE_KEYS.filter((key) => key !== "brand.name");
    for (const locale of LOCALES) {
      if (locale === DEFAULT_LOCALE || locale === "en_US") continue;
      const identical = translatable.filter(
        (key) => translate(locale, key) === translate(DEFAULT_LOCALE, key),
      );
      expect(
        identical.length,
        `${locale} repeats English for ${identical.join(", ")}`,
      ).toBeLessThan(translatable.length / 2);
    }
  });

  it("resolves a region subtag to its primary locale", () => {
    expect(normaliseLocale("fr-CA")).toBe("fr");
    expect(normaliseLocale("EN-GB")).toBe("en");
    expect(normaliseLocale("pt")).toBe("pt");
    expect(normaliseLocale("klingon")).toBeNull();
    expect(normaliseLocale(null)).toBeNull();
    expect(normaliseLocale("")).toBeNull();
  });

  it("normalises en_US and en-US to the en_US locale rather than collapsing to en", () => {
    expect(normaliseLocale("en_US")).toBe("en_US");
    expect(normaliseLocale("en-US")).toBe("en_US");
    expect(normaliseLocale("EN_US")).toBe("en_US");
    expect(normaliseLocale("en")).toBe("en");
  });

  it("knows which locales are written right to left", () => {
    for (const locale of RTL_LOCALES) expect(isRtl(locale)).toBe(true);
    expect(isRtl("en")).toBe(false);
    expect(isRtl("ar-EG")).toBe(true);
    expect(isRtl("nonsense")).toBe(false);
  });
});
