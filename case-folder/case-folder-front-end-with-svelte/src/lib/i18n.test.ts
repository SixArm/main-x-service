// Coverage + behaviour pins for the case-folder i18n store.
//
// - Every one of the 13 locales must define every key in the English
//   catalog (a missing key is a UI bug: it would silently fall back to
//   English for that locale).
// - A spot-check of a non-Latin locale proves the tables are real
//   translations, not English placeholders.
// - isRtl must be true for the two RTL locales (ar, ur) and false for the
//   left-to-right ones, since the layout drives <html dir> off it.

import { describe, it, expect } from 'vitest';
import {
    LOCALES,
    STRING_KEYS,
    STRINGS_BY_LOCALE,
    isRtl,
    translate,
    type Locale,
} from './i18n.svelte';

describe('i18n catalog', () => {
    it('supports exactly the 13 required locales', () => {
        expect([...LOCALES]).toEqual([
            'en',
            'cy',
            'es',
            'fr',
            'de',
            'ar',
            'ru',
            'hi',
            'zh',
            'bn',
            'pt',
            'id',
            'ur',
        ]);
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

    it('spot-checks a non-Latin locale (zh) against English', () => {
        // Chinese must differ from English on a representative chrome key.
        expect(translate('nav.dashboard', 'zh')).toBe('仪表板');
        expect(translate('nav.dashboard', 'zh')).not.toBe(
            translate('nav.dashboard', 'en'),
        );
        // And the placeholder shape is preserved across locales.
        expect(STRINGS_BY_LOCALE.zh['scan.matches']).toContain('{n}');
    });

    it('marks ar and ur as right-to-left, others as left-to-right', () => {
        expect(isRtl('ar')).toBe(true);
        expect(isRtl('ur')).toBe(true);
        // Region subtags still resolve (ar-EG → ar).
        expect(isRtl('ar-EG')).toBe(true);
        for (const locale of LOCALES.filter(
            (l): l is Locale => l !== 'ar' && l !== 'ur',
        )) {
            expect(isRtl(locale), `${locale} should be LTR`).toBe(false);
        }
    });
});
