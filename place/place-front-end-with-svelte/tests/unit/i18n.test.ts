// Unit tests for the dependency-free i18n store: per-locale lookup, the
// locale → en → key fallback chain, and full key coverage across every
// supported locale (so a missing translation is caught here, not in prod).
import { describe, expect, it, vi } from "vitest";

// The store imports `browser` from $app/environment; off-browser keeps it
// from touching localStorage during the test.
vi.mock("$app/environment", () => ({ browser: false }));

import {
  translate,
  isRtl,
  i18n,
  LOCALES,
  LOCALE_LABELS,
  DEFAULT_LOCALE,
  STRING_KEYS,
  STRINGS_FOR_TEST,
  type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n translate()", () => {
  it("returns the English source strings", () => {
    expect(translate("nav.dashboard", "en-001")).toBe("Dashboard");
    expect(translate("places.title", "en-001")).toBe("Places");
    expect(translate("form.save", "en-001")).toBe("Save");
  });

  it("returns the Spanish translations", () => {
    expect(translate("nav.dashboard", "es-001")).toBe("Panel");
    expect(translate("places.title", "es-001")).toBe("Lugares");
    expect(translate("form.save", "es-001")).toBe("Guardar");
  });

  it("uses the glossary translations consistently", () => {
    // Glossary terms must match verbatim across the family.
    expect(translate("nav.matchCheck", "cy-001")).toBe("Gwiriad cydweddu");
    expect(translate("nav.merge", "fr-001")).toBe("Fusionner");
    expect(translate("nav.toggle", "en-001")).toBe("Toggle navigation");
  });

  it("falls back to English for an unknown locale", () => {
    // An unsupported locale code resolves to the English table.
    expect(translate("places.title", "xx" as never)).toBe("Places");
  });

  it("falls back to the key itself for an unknown key", () => {
    expect(translate("does.not.exist" as StringKey, "en-001")).toBe(
      "does.not.exist",
    );
  });

  it("every locale defines every English key", () => {
    // The English catalog is the source of truth; every other locale must
    // define the exact same key set (no missing, no extra).
    const enKeys = [...STRING_KEYS].sort();
    for (const locale of LOCALES) {
      const localeKeys = Object.keys(STRINGS_FOR_TEST[locale]).sort();
      expect(localeKeys, `${locale} key set differs from en`).toEqual(enKeys);
    }
  });

  it("every locale returns a non-empty translation for every key", () => {
    for (const locale of LOCALES) {
      for (const key of STRING_KEYS) {
        const value = translate(key as StringKey, locale);
        expect(value.length, `${locale} empty ${key}`).toBeGreaterThan(0);
      }
    }
  });

  it("DEFAULT_LOCALE is English", () => {
    expect(DEFAULT_LOCALE).toBe("en-001");
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
      expect(label, code).not.toMatch(/[()]/);
    }
  });

  it("resolves legacy and regional codes by primary language", () => {
    i18n.set("en_US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("es-MX");
    expect(i18n.locale).toBe("es-001");
    i18n.set("zh");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("ZH-CN");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("xx");
    expect(i18n.locale).toBe("en-001");
  });

  it("has the Copy Link and auth keys", () => {
    expect(translate("share.copy_link", "en-001")).toBe("Copy Link");
    expect(translate("auth.signin", "en-001")).toBe("Sign in");
    expect(translate("auth.signout", "en-001")).toBe("Sign out");
  });

  it("spot-checks the new Arabic translations", () => {
    expect(translate("nav.dashboard", "ar-001")).toBe("لوحة المعلومات");
    expect(translate("nav.merge", "ar-001")).toBe("دمج");
  });

  it("spot-checks the new Chinese translations", () => {
    expect(translate("nav.dashboard", "zh-cn")).toBe("仪表板");
    expect(translate("form.save", "zh-cn")).toBe("保存");
  });

  it("isRtl is true for ar-001 and false for en-001 and zh-cn", () => {
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
  });
});
