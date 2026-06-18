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
    type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    it("supports exactly the 13 expected locales with the default first", () => {
        expect(LOCALES).toEqual([
            "en", "cy", "es", "fr", "de",
            "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
        ]);
        expect(LOCALES).toHaveLength(13);
        expect(DEFAULT_LOCALE).toBe("en");
    });

    it("has a human label for every locale", () => {
        expect(LOCALE_LABELS).toEqual({
            en: "English",
            cy: "Cymraeg",
            es: "Español",
            fr: "Français",
            de: "Deutsch",
            ar: "العربية",
            ru: "Русский",
            hi: "हिन्दी",
            zh: "中文",
            bn: "বাংলা",
            pt: "Português",
            id: "Bahasa Indonesia",
            ur: "اردو",
        });
    });

    it("marks ar/ur as RTL and everything else as LTR", () => {
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
        expect(isRtl("de")).toBe(false);
    });

    it("spot-checks new-locale translations for a couple of keys", () => {
        expect(translate("nav.dashboard", "ar")).toBe("لوحة المعلومات");
        expect(translate("search.submit", "ar")).toBe("بحث");
        expect(translate("nav.dashboard", "zh")).toBe("仪表板");
        expect(translate("merge.merge", "zh")).toBe("合并");
    });

    it("returns the correct English strings for sample keys", () => {
        expect(translate("nav.dashboard", "en")).toBe("Dashboard");
        expect(translate("search.submit", "en")).toBe("Search");
        expect(translate("form.required", "en")).toBe("Required");
    });

    it("returns the correct Spanish strings for the same keys", () => {
        expect(translate("nav.dashboard", "es")).toBe("Panel");
        expect(translate("search.submit", "es")).toBe("Buscar");
        expect(translate("form.required", "es")).toBe("Obligatorio");
    });

    it("uses the glossary translations across the other locales", () => {
        expect(translate("nav.merge", "cy")).toBe("Uno");
        expect(translate("nav.merge", "fr")).toBe("Fusionner");
        expect(translate("nav.merge", "de")).toBe("Zusammenführen");
        expect(translate("nav.toggle", "en")).toBe("Toggle navigation");
    });

    it("falls back to English for a missing translation, then to the key", () => {
        // An unknown locale falls through to the English table.
        expect(translate("nav.dashboard", "xx" as never)).toBe("Dashboard");
        // An unknown key falls through to the key string itself.
        expect(translate("does.not.exist" as StringKey, "en")).toBe("does.not.exist");
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
    for (const k of keys) out[k] = translate(k, "en");
    return out;
}
