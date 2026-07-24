// Unit tests: the one-shot localStorage migration for the 2026-07-23
// rename (`mxi.hcm.*` -> `mxi.wpm.*`).
//
// Without it a returning user silently loses their language and theme —
// the app looks like it forgot them. The i18n module reads the key at
// module-evaluation time, so each case resets the module registry and
// re-imports it with localStorage already primed.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The module only touches localStorage when it believes it is running in
// a browser; this supplies the flag.
vi.mock("$app/environment", () => ({ browser: true }));

// jsdom is configured with an opaque origin here, so it exposes no
// `localStorage`. Stub a minimal in-memory one rather than widen the
// project's vitest config for a single suite — the app only uses
// getItem / setItem / removeItem.
function memoryStorage() {
  const entries = new Map<string, string>();
  return {
    getItem: (key: string) => entries.get(key) ?? null,
    setItem: (key: string, value: string) => void entries.set(key, String(value)),
    removeItem: (key: string) => void entries.delete(key),
    clear: () => entries.clear(),
  };
}

vi.stubGlobal("localStorage", memoryStorage());

const CURRENT = "mxi.wpm.locale";
const LEGACY = "mxi.hcm.locale";

/** Re-import i18n with a fresh module registry, so its module-scope
 *  `readStoredLocale()` runs against the localStorage set up by a test. */
async function freshI18n() {
  vi.resetModules();
  return import("../../src/lib/i18n.svelte");
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  localStorage.clear();
});

describe("locale key migration", () => {
  it("adopts the pre-rename locale and re-persists it under the new key", async () => {
    localStorage.setItem(LEGACY, "fr");

    const { i18n } = await freshI18n();

    expect(i18n.locale).toBe("fr");
    expect(localStorage.getItem(CURRENT)).toBe("fr");
    expect(localStorage.getItem(LEGACY)).toBeNull();
  });

  it("prefers the current key when both are present", async () => {
    localStorage.setItem(CURRENT, "de");
    localStorage.setItem(LEGACY, "fr");

    const { i18n } = await freshI18n();

    expect(i18n.locale).toBe("de");
    // The stale key is left alone: the current one already answered.
    expect(localStorage.getItem(CURRENT)).toBe("de");
  });

  it("falls back to the default when neither key is set", async () => {
    const { i18n, DEFAULT_LOCALE } = await freshI18n();

    expect(i18n.locale).toBe(DEFAULT_LOCALE);
    expect(localStorage.getItem(CURRENT)).toBeNull();
  });

  it("ignores an unsupported legacy value rather than adopting it", async () => {
    localStorage.setItem(LEGACY, "not-a-locale");

    const { i18n, DEFAULT_LOCALE } = await freshI18n();

    expect(i18n.locale).toBe(DEFAULT_LOCALE);
    expect(localStorage.getItem(CURRENT)).toBeNull();
  });

  it("exports the legacy key as the OLD name (a rename sweep must not touch it)", async () => {
    const { LEGACY_LOCALE_KEY, LOCALE_KEY } = await freshI18n();

    expect(LEGACY_LOCALE_KEY).toBe("mxi.hcm.locale");
    expect(LOCALE_KEY).toBe("mxi.wpm.locale");
  });
});
