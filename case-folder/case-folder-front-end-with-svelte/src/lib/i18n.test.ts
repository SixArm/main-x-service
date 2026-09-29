// Coverage + behaviour pins for the case-folder i18n store.
//
// - Every one of the 7 locales must define every key in the English
//   catalog (a missing key is a UI bug: it would silently fall back to
//   English for that locale).
// - A spot-check of a non-Latin locale proves the tables are real
//   translations, not English placeholders.
// - isRtl must be true for the RTL locale (ar-001) and false for the
//   left-to-right ones, since the layout drives <html dir> off it.

import { describe, it, expect } from 'vitest';
import {
    DEFAULT_LOCALE,
    LOCALES,
    LOCALE_LABELS,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    isRtl,
    translate,
    i18n,
    type Locale,
} from './i18n.svelte';

describe('i18n catalog', () => {
    it('supports exactly the 8 required locales, sorted by code', () => {
        expect([...LOCALES]).toEqual([
            'ar-001',
            'cy-001',
            'de-de',
            'en-001',
            'es-001',
            'fr-001',
            'hi-001',
            'zh-cn',
        ]);
        expect(LOCALES.map((l) => LOCALE_LABELS[l])).toEqual([
            'العربية',
            'Cymraeg',
            'Deutsch - Deutschland',
            'English',
            'Español',
            'Français',
            'हिन्दी',
            '中文 - 中国',
        ]);
        for (const label of Object.values(LOCALE_LABELS)) {
            expect(label).not.toMatch(/[()]/);
        }
    });

    it('resolves a bare language or region variant to the supported locale', () => {
        i18n.set('en_US');
        expect(i18n.locale).toBe('en-001');
        i18n.set('es-MX');
        expect(i18n.locale).toBe('es-001');
        i18n.set('zh');
        expect(i18n.locale).toBe('zh-cn');
        i18n.set('de-DE');
        expect(i18n.locale).toBe('de-de');
        i18n.set('ja');
        expect(i18n.locale).toBe(DEFAULT_LOCALE);
        i18n.set('en-001');
        expect(i18n.locale).toBe('en-001');
    });

    it('every locale defines every key (full coverage)', () => {
        for (const locale of LOCALES) {
            const table = STRINGS_BY_LOCALE[locale];
            const missing = STRING_KEYS.filter((key) => !(key in table));
            expect(missing, `locale "${locale}" is missing keys`).toEqual([]);
            // No key should be left as an empty string either.
            const empty = STRING_KEYS.filter((key) => table[key] === '');
            expect(empty, `locale "${locale}" has empty values`).toEqual([]);
        }
    });

    it('spot-checks a non-Latin locale (zh-cn) against English', () => {
        // Chinese must differ from English on a representative chrome key.
        expect(translate('nav.dashboard', 'zh-cn')).toBe('仪表板');
        expect(translate('nav.dashboard', 'zh-cn')).not.toBe(
            translate('nav.dashboard', 'en-001'),
        );
        // And the placeholder shape is preserved across locales.
        expect(STRINGS_BY_LOCALE['zh-cn']['scan.matches']).toContain('{n}');
    });

    it('marks ar-001 as right-to-left, others as left-to-right', () => {
        expect(isRtl('ar-001')).toBe(true);
        // Region subtags still resolve (ar-EG -> ar-001).
        expect(isRtl('ar-EG')).toBe(true);
        for (const locale of LOCALES.filter(
            (l): l is Locale => l !== 'ar-001',
        )) {
            expect(isRtl(locale), `${locale} should be LTR`).toBe(false);
        }
    });
});
