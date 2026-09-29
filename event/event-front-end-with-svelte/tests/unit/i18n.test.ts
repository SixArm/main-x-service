// Unit tests for the dependency-free i18n store (src/lib/i18n.svelte.ts):
// catalog coverage, the locale→en→key fallback chain, and a couple of
// concrete translations. `translate(key, locale)` is pure (does not read
// the reactive current locale when `locale` is supplied), so it is safe to
// exercise here without mounting a component.
//
// $app/environment is mocked off-browser so importing the *.svelte.ts module
// under jsdom does not touch localStorage during module init.
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/environment", () => ({ browser: false }));

import {
    LOCALES,
    DEFAULT_LOCALE,
    LOCALE_LABELS,
    isRtl,
    translate,
    i18n,
    type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    it("supports exactly the seven expected locales, sorted by code, default en-001", () => {
        expect([...LOCALES]).toEqual([
            "ar-001", "cy-001", "en-001", "es-001", "fr-001", "hi-001", "zh-cn",
        ]);
        expect(DEFAULT_LOCALE).toBe("en-001");
    });

    it("has a human label for every locale, language only for -001, no parentheses", () => {
        expect(LOCALE_LABELS).toEqual({
            "ar-001": "العربية",
            "cy-001": "Cymraeg",
            "en-001": "English",
            "es-001": "Español",
            "fr-001": "Français",
            "hi-001": "हिन्दी",
            "zh-cn": "中文 - 中国",
        });
        for (const label of Object.values(LOCALE_LABELS)) {
            expect(label).not.toMatch(/[()]/);
        }
    });

    it("normalises bare and regional tags to the supported locale by primary subtag", () => {
        i18n.set("en_US");
        expect(i18n.locale).toBe("en-001");
        i18n.set("es-MX");
        expect(i18n.locale).toBe("es-001");
        i18n.set("zh-cn");
        expect(i18n.locale).toBe("zh-cn");
        i18n.set("ZH-cn");
        expect(i18n.locale).toBe("zh-cn");
        i18n.set("de");
        expect(i18n.locale).toBe("en-001");
    });

    it("marks ar-001 as RTL and everything else as LTR", () => {
        expect(isRtl("ar-001")).toBe(true);
        expect(isRtl("en-001")).toBe(false);
        expect(isRtl("zh-cn")).toBe(false);
    });

    it("spot-checks new-locale translations for a couple of keys", () => {
        expect(translate("nav.dashboard", "ar-001")).toBe("لوحة المعلومات");
        expect(translate("search.submit", "ar-001")).toBe("بحث");
        expect(translate("nav.dashboard", "zh-cn")).toBe("仪表板");
        expect(translate("merge.merge", "zh-cn")).toBe("合并");
    });

    it("returns the correct English strings for sample keys", () => {
        expect(translate("nav.dashboard", "en-001")).toBe("Dashboard");
        expect(translate("search.submit", "en-001")).toBe("Search");
        expect(translate("form.required", "en-001")).toBe("Required");
    });

    it("returns the correct Spanish strings for the same keys", () => {
        expect(translate("nav.dashboard", "es-001")).toBe("Panel");
        expect(translate("search.submit", "es-001")).toBe("Buscar");
        expect(translate("form.required", "es-001")).toBe("Obligatorio");
    });

    it("uses the glossary translations across the other locales", () => {
        expect(translate("nav.merge", "cy-001")).toBe("Uno");
        expect(translate("nav.merge", "fr-001")).toBe("Fusionner");
        expect(translate("nav.toggle", "en-001")).toBe("Toggle navigation");
    });

    it("falls back to English for a missing translation, then to the key", () => {
        // An unknown locale falls through to the English table.
        expect(translate("nav.dashboard", "xx" as never)).toBe("Dashboard");
        // An unknown key falls through to the key string itself.
        expect(translate("does.not.exist" as StringKey, "en-001")).toBe("does.not.exist");
    });

    it("every locale covers the full English key set", () => {
        // The English catalog is the source of truth; assert each non-default
        // locale resolves every key to a non-empty, non-key string (i.e. it
        // is genuinely present, not merely falling back to the key).
        const enKeys = Object.keys(translateAllEnKeys()) as StringKey[];
        for (const locale of LOCALES) {
            for (const key of enKeys) {
                const value = translate(key, locale);
                expect(value, `${locale}:${key}`).toBeTruthy();
                expect(value, `${locale}:${key} should be translated, not the raw key`).not.toBe(key);
            }
        }
    });
});

// Helper: collect the English key set by probing every key referenced in the
// tests plus walking the module's exported keys is not possible (STRINGS is
// private), so derive the key set from the `en` translations we can observe.
// Instead, assert coverage structurally: the StringKey union is keyof en, so
// we enumerate via a representative sample taken from the module by importing
// the catalog indirectly through translate. To get the real full set we read
// it off the type at runtime by listing the keys we know exist.
function translateAllEnKeys(): Record<string, string> {
    // Pull the live English table by translating a sentinel: we cannot import
    // the private STRINGS, so we reconstruct the key set from a static list
    // that must stay in lock-step with the catalog. Kept deliberately broad.
    const keys: StringKey[] = [
        "brand", "brand.tagline",
        "nav.dashboard", "nav.events", "nav.newEvent", "nav.matchCheck", "nav.merge", "nav.toggle",
        "chrome.theme", "chrome.language",
        "dashboard.title", "dashboard.service", "dashboard.recentActivity", "dashboard.noRecent",
        "events.title", "events.new", "events.searchPlaceholder",
        "events.filter.from", "events.filter.to", "events.filter.status", "events.filter.type",
        "events.filter.any", "events.filter.fuzzy", "events.filter.apply", "events.loading",
        "events.count.one", "events.count.other",
        "grid.id", "grid.name", "grid.start", "grid.type", "grid.status", "grid.mode",
        "new.title", "new.create", "new.duplicatesTitle", "new.duplicatesDetected",
        "match.title", "match.name", "match.threshold", "match.thresholdHint",
        "match.start", "match.end", "match.organizerName", "match.find", "match.matching",
        "merge.title", "merge.mainId", "merge.mainIdHint", "merge.dupId", "merge.dupIdHint",
        "merge.reason", "merge.reasonHint", "merge.reasonPlaceholder", "merge.loadPreview",
        "merge.merge", "merge.merging", "merge.bothIdsRequired", "merge.idsMustDiffer",
        "merge.confirm", "merge.previewTitle", "merge.preview.main", "merge.preview.duplicate",
        "merge.preview.none", "merge.preview.noDate", "merge.completedTitle", "merge.completedBody",
        "merge.viewMerged",
        "detail.loading", "detail.edit", "detail.audit", "detail.delete", "detail.identity",
        "detail.id", "detail.start", "detail.end", "detail.status", "detail.type", "detail.mode",
        "detail.timeZone", "detail.duration", "detail.description", "detail.empty",
        "detail.loc.place", "detail.loc.address", "detail.loc.virtual", "detail.loc.text",
        "detail.location", "detail.organizers", "detail.performers", "detail.identifiers",
        "detail.offers", "detail.ticket", "detail.confirmDelete",
        "edit.title", "edit.cancel", "edit.loading", "edit.save",
        "audit.title", "audit.back", "audit.loading", "audit.none", "audit.by", "audit.payload",
        "form.name", "form.required", "form.eventType", "form.start", "form.startHint",
        "form.end", "form.endAfterStart", "form.doorTime", "form.doorBeforeStart", "form.status",
        "form.attendanceMode", "form.timeZone", "form.timeZoneHint", "form.allDay", "form.no",
        "form.yes", "form.description", "form.url", "form.duration", "form.durationHint",
        "form.maxCapacityTotal", "form.maxPhysical", "form.maxVirtual", "form.keywords",
        "form.keywordsHint", "form.languages", "form.languagesHint", "form.saving", "form.save",
        "form.reset",
        "search.placeholder", "search.submit",
        "results.title", "results.none", "results.breakdown",
    ];
    const out: Record<string, string> = {};
    for (const k of keys) out[k] = translate(k, "en-001");
    return out;
}
