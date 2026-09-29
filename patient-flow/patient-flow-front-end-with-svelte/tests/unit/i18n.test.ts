// Unit tests for the dependency-free i18n catalog + translate() fallback.
// `translate` is pure (takes an explicit locale), so these run without a
// Svelte component or a browser. The store reads `browser` from
// `$app/environment`; mock it off so seeding is deterministic.
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/environment", () => ({ browser: false }));

import {
  LOCALES,
  DEFAULT_LOCALE,
  LOCALE_LABELS,
  RTL_LOCALES,
  isRtl,
  translate,
  i18n,
  type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
  it("returns the English source strings", () => {
    expect(translate("brand", "en-001")).toBe("Patient Flow");
    expect(translate("nav.wards", "en-001")).toBe("Wards");
    expect(translate("auth.signin", "en-001")).toBe("Sign in");
    expect(translate("share.copy_link", "en-001")).toBe("Copy Link");
  });

  it("returns translated strings for the same keys", () => {
    expect(translate("nav.wards", "es-001")).toBe("Salas");
    expect(translate("nav.wards", "cy-001")).toBe("Wardiau");
    expect(translate("nav.wards", "fr-001")).toBe("Services");
  });

  it("falls back to English for an unknown locale, then to the key", () => {
    expect(translate("nav.wards", "xx" as never)).toBe("Wards");
    expect(translate("does.not.exist" as StringKey, "en-001")).toBe(
      "does.not.exist",
    );
  });

  it("default locale is English", () => {
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

  it("marks ar-001 as RTL and everything else LTR", () => {
    expect([...RTL_LOCALES]).toEqual(["ar-001"]);
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
  });

  it("every locale covers the chrome and splash keys", () => {
    const keys: StringKey[] = [
      "brand",
      "brand.tagline",
      "nav.toggle",
      "auth.signin",
      "auth.signout",
      "splash.hero.title",
      "splash.hero.subtitle",
      "splash.cta.title",
    ];
    for (const area of ["benefits", "features", "trust"] as const) {
      for (let n = 1; n <= 6; n++) {
        keys.push(`splash.${area}.${n}.title` as StringKey);
        keys.push(`splash.${area}.${n}.body` as StringKey);
      }
    }
    for (const locale of LOCALES) {
      for (const key of keys) {
        const value = translate(key, locale);
        expect(value.length).toBeGreaterThan(0);
        expect(value).not.toBe(key);
      }
    }
  });
});
