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
  RTL_LOCALES,
  i18n,
  type StringKey,
  type Locale,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
  // English (source of truth) returns its literal strings.
  it("returns English strings for the default locale", () => {
    expect(translate("nav.dashboard", "en-001")).toBe("Dashboard");
    expect(translate("things.title", "en-001")).toBe("Things");
    expect(translate("nav.toggle", "en-001")).toBe("Toggle navigation");
  });

  // Spanish returns its translated strings (glossary-aligned where shared).
  it("returns Spanish strings for the es locale", () => {
    expect(translate("nav.dashboard", "es-001")).toBe("Panel");
    expect(translate("search.action", "es-001")).toBe("Buscar");
    expect(translate("merge.merge", "es-001")).toBe("Fusionar");
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
    expect(translate(missing, "fr-001")).toBe("does.not.exist");
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

  // The family-wide seven-locale set, sorted by code, with labels.
  it("supports exactly the seven expected locales, sorted by code, with labels", () => {
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
  });

  // -001 locales are labelled by language alone; never parentheses.
  it("labels -001 locales by language only and never uses parentheses", () => {
    for (const [code, label] of Object.entries(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
      if (code.endsWith("-001")) expect(label).not.toContain(" - ");
    }
  });

  // Legacy and regional codes resolve by primary subtag.
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

  // Spot-check two of the newly added locales against the shared glossary.
  it("returns Arabic strings for the ar locale", () => {
    expect(translate("nav.dashboard", "ar-001")).toBe("لوحة المعلومات");
    expect(translate("search.action", "ar-001")).toBe("بحث");
  });

  it("returns Chinese strings for the zh locale", () => {
    expect(translate("nav.merge", "zh-cn")).toBe("合并");
    expect(translate("chrome.language", "zh-cn")).toBe("语言");
  });

  // RTL detection: true for Arabic only.
  it("detects right-to-left locales", () => {
    expect([...RTL_LOCALES]).toEqual(["ar-001"]);
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("ur")).toBe(false);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
    // Region subtags are tolerated (only the primary subtag matters).
    expect(isRtl("ar-EG")).toBe(true);
  });
});
