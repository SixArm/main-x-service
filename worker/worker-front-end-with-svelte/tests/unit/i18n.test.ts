// Unit tests for the dependency-free i18n catalog. `translate` is pure
// (it takes an explicit locale), so these run without a Svelte component
// or a browser. Mock $app/environment so importing the store off-browser
// doesn't touch localStorage.
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/environment", () => ({ browser: false }));

import {
  LOCALES,
  LOCALE_LABELS,
  RTL_LOCALES,
  DEFAULT_LOCALE,
  CATALOG,
  STRING_KEYS,
  isRtl,
  translate,
  i18n,
  type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
  it("returns the English source strings", () => {
    expect(translate("workers.heading", "en-001")).toBe("Workers");
    expect(translate("nav.dashboard", "en-001")).toBe("Dashboard");
    expect(translate("common.search", "en-001")).toBe("Search");
  });

  it("returns Spanish strings for the same keys", () => {
    expect(translate("workers.heading", "es-001")).toBe("Trabajadores");
    expect(translate("nav.dashboard", "es-001")).toBe("Panel");
    expect(translate("common.search", "es-001")).toBe("Buscar");
  });

  it("uses the glossary translations across locales", () => {
    expect(translate("nav.matchCheck", "cy-001")).toBe("Gwiriad cydweddu");
    expect(translate("nav.merge", "es-001")).toBe("Fusionar");
    expect(translate("common.loading", "fr-001")).toBe("Chargement…");
  });

  it("falls back to English for an unsupported locale", () => {
    // `translate` is typed to a Locale, but a bad locale at runtime
    // (e.g. from storage) must degrade to the English table.
    const bad = "xx" as unknown as typeof DEFAULT_LOCALE;
    expect(translate("workers.heading", bad)).toBe(
      translate("workers.heading", "en-001"),
    );
  });

  it("falls back to the key itself for an unknown key", () => {
    const missing = "does.not.exist" as unknown as StringKey;
    expect(translate(missing, "es-001")).toBe("does.not.exist");
  });

  it("every locale covers the full English key set", () => {
    const enKeys = new Set(STRING_KEYS);
    for (const locale of LOCALES) {
      const localeKeys = new Set(Object.keys(CATALOG[locale]));
      // Same cardinality + every en key present = full coverage.
      expect(localeKeys.size).toBe(enKeys.size);
      for (const key of STRING_KEYS) {
        expect(localeKeys.has(key)).toBe(true);
        expect(translate(key, locale).length).toBeGreaterThan(0);
      }
    }
  });

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

  it("labels -001 locales by language only and never uses parentheses", () => {
    for (const [code, label] of Object.entries(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
      if (code.endsWith("-001")) expect(label).not.toContain(" - ");
    }
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

  it("spot-checks new locale glossary translations", () => {
    expect(translate("nav.dashboard", "ar-001")).toBe("لوحة المعلومات");
    expect(translate("common.search", "ar-001")).toBe("بحث");
    expect(translate("nav.dashboard", "zh-cn")).toBe("仪表板");
    expect(translate("common.search", "zh-cn")).toBe("搜索");
  });

  it("marks ar-001 as RTL and everything else LTR", () => {
    expect([...RTL_LOCALES]).toEqual(["ar-001"]);
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
    expect(isRtl("hi-001")).toBe(false);
    // Underscore / regional tags are tolerated.
    expect(isRtl("ar_001")).toBe(true);
    expect(isRtl("ar-EG")).toBe(true);
  });
});
