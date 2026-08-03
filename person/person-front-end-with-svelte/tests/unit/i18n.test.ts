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
    type StringKey,
} from "../../src/lib/i18n.svelte";

describe("i18n catalog", () => {
    it("returns the English source strings", () => {
        expect(translate("nav.dashboard", "en")).toBe("Dashboard");
        expect(translate("search.submit", "en")).toBe("Search");
        expect(translate("form.save", "en")).toBe("Save");
    });

    it("returns Spanish strings for the same keys", () => {
        expect(translate("nav.dashboard", "es")).toBe("Panel");
        expect(translate("search.submit", "es")).toBe("Buscar");
        expect(translate("form.save", "es")).toBe("Guardar");
    });

    it("uses the glossary translations across locales", () => {
        // Spot-check the shared UI glossary (consistency across all 6 apps).
        expect(translate("nav.dashboard", "cy")).toBe("Dangosfwrdd");
        expect(translate("nav.dashboard", "fr")).toBe("Tableau de bord");
        expect(translate("nav.dashboard", "de")).toBe("Übersicht");
        expect(translate("nav.merge", "cy")).toBe("Uno");
        expect(translate("nav.language", "de")).toBe("Sprache");
    });

    it("the hamburger toggle key keeps its exact English value", () => {
        // An existing layout test asserts the aria-label "Toggle navigation".
        expect(translate("nav.toggle", "en")).toBe("Toggle navigation");
    });

    it("falls back to English for an unknown locale, then to the key", () => {
        // Unknown locale → English table.
        expect(translate("form.save", "xx" as never)).toBe("Save");
        // Unknown key (cast past the type guard) → the key string itself.
        expect(translate("does.not.exist" as StringKey, "en")).toBe("does.not.exist");
    });

    it("default locale is English", () => {
        expect(DEFAULT_LOCALE).toBe("en");
    });

    it("supports exactly the thirteen expected locales with labels", () => {
        expect([...LOCALES]).toEqual([
            "en", "cy", "es", "fr", "de",
            "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
        ]);
        expect(LOCALES.length).toBe(13);
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

    it("spot-checks the new-locale translations", () => {
        // Arabic and Chinese cover the same glossary keys.
        expect(translate("nav.dashboard", "ar")).toBe("لوحة المعلومات");
        expect(translate("search.submit", "ar")).toBe("بحث");
        expect(translate("nav.dashboard", "zh")).toBe("仪表板");
        expect(translate("form.save", "zh")).toBe("保存");
    });

    it("marks ar/ur as RTL and everything else LTR", () => {
        expect([...RTL_LOCALES]).toEqual(["ar", "ur"]);
        expect(isRtl("ar")).toBe(true);
        expect(isRtl("ur")).toBe(true);
        expect(isRtl("en")).toBe(false);
        expect(isRtl("zh")).toBe(false);
        expect(isRtl("de")).toBe(false);
    });

    it("every locale covers the full English key set", () => {
        // Every catalog key must resolve to a non-empty, non-raw-key string in
        // every locale (a missing key would silently fall back to English, but
        // here we require each locale to define its own entry).
        for (const locale of LOCALES) {
            for (const key of CANONICAL_KEYS) {
                const value = translate(key, locale);
                expect(typeof value).toBe("string");
                expect(value.length).toBeGreaterThan(0);
                expect(value).not.toBe(key);
            }
        }
    });
});

// Canonical key list mirrored from STRINGS.en. Kept in sync with the catalog;
// the "every locale covers the key set" assertion guards against drift.
const CANONICAL_KEYS: StringKey[] = [
    "brand", "brand.tagline", "nav.toggle", "nav.dashboard", "nav.persons",
    "nav.new", "nav.match", "nav.merge", "nav.theme", "nav.language",
    "dashboard.head.title", "dashboard.title", "dashboard.serviceStatus",
    "dashboard.recentActivity", "dashboard.noRecent", "persons.head.title",
    "persons.title", "persons.new", "persons.searchPlaceholder", "persons.fuzzy",
    "persons.phonetic", "persons.loading", "persons.recordCount.one",
    "persons.recordCount.other", "grid.id", "grid.family", "grid.given",
    "grid.dob", "grid.gender", "grid.active", "grid.yes", "grid.no",
    "search.submit", "search.placeholder", "new.head.title", "new.title",
    "new.create", "new.possibleDuplicates", "new.duplicatesDetected.prefix",
    "new.duplicatesDetected.suffix", "match.head.title", "match.title",
    "match.family", "match.given", "match.givenHint", "match.birthDate",
    "match.gender", "match.taxId", "match.threshold", "match.thresholdHint",
    "match.matching", "match.find", "merge.head.title", "merge.title",
    "merge.mainId", "merge.mainIdHint", "merge.dupId", "merge.dupIdHint",
    "merge.reason", "merge.reasonHint", "merge.reasonPlaceholder",
    "merge.loadPreview", "merge.merging", "merge.merge", "merge.bothIdsRequired",
    "merge.mustDiffer", "merge.preview", "merge.main", "merge.duplicate",
    "merge.completed", "merge.recordPrefix", "merge.recordCreatedAt",
    "merge.viewMain", "merge.noRecord", "merge.noDob", "merge.confirm.prefix",
    "merge.confirm.into", "merge.confirm.suffix", "detail.head.title.prefix",
    "detail.loading", "detail.edit", "detail.audit", "detail.delete",
    "detail.identity", "detail.id", "detail.active", "detail.yes", "detail.no",
    "detail.gender", "detail.birthDate", "detail.taxId", "detail.deceased",
    "detail.deceasedYes", "detail.deceasedNo", "detail.identifiers",
    "detail.addresses", "detail.telecom", "detail.emergencyContacts",
    "detail.primary", "detail.confirmDelete", "edit.head.title.prefix",
    "edit.title", "edit.cancel", "edit.loading", "edit.save",
    "audit.head.title.prefix", "audit.title", "audit.back", "audit.loading",
    "audit.noEntries", "audit.by", "audit.payload", "form.birthDate",
    "form.gender", "form.taxId", "form.taxIdHint", "form.saving", "form.save",
    "form.reset", "form.required", "form.atLeastOneGiven", "form.futureBirthDate",
    "name.family", "name.given", "name.givenHint", "matchResults.title",
    "matchResults.noCandidates", "matchResults.dob", "matchResults.breakdown",
    "links.title", "links.loading", "links.empty", "links.assertHeading",
    "links.kind", "links.kind.sameIdentity", "links.kind.worksAt",
    "links.kind.memberOf", "links.toRef", "links.toRefHint", "links.role",
    "links.confidence", "links.confidenceHint", "links.provenance",
    "links.provenanceHint", "links.validFrom", "links.validTo", "links.submit",
    "links.submitting", "links.withdraw", "links.withdrawing",
    "links.confirmWithdraw", "links.error.required", "links.error.malformedRef",
    "links.error.wrongType.prefix", "links.error.wrongType.suffix",
];
