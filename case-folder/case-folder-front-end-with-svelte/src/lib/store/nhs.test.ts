import { describe, it, expect } from 'vitest';
import { normaliseNhsNumber, formatNhsNumber, isValidNhsNumber } from './nhs';

describe('normaliseNhsNumber', () => {
    it('strips spaces and punctuation to bare digits', () => {
        expect(normaliseNhsNumber('943 476 5919')).toBe('9434765919');
        expect(normaliseNhsNumber('943-476-5919')).toBe('9434765919');
    });
});

describe('formatNhsNumber', () => {
    it('groups ten digits as XXX XXX XXXX', () => {
        expect(formatNhsNumber('9434765919')).toBe('943 476 5919');
    });
    it('formats partial input as the user types', () => {
        expect(formatNhsNumber('943')).toBe('943');
        expect(formatNhsNumber('943476')).toBe('943 476');
        expect(formatNhsNumber('9434765')).toBe('943 476 5');
    });
});

describe('isValidNhsNumber (Modulus 11)', () => {
    it('accepts known-valid numbers', () => {
        expect(isValidNhsNumber('943 476 5919')).toBe(true);
        expect(isValidNhsNumber('987 654 3210')).toBe(true);
        expect(isValidNhsNumber('999 999 9999')).toBe(true);
    });
    it('rejects a bad check digit', () => {
        expect(isValidNhsNumber('943 476 5918')).toBe(false);
    });
    it('rejects the wrong length', () => {
        expect(isValidNhsNumber('943 476 591')).toBe(false);
    });
    it('rejects a number whose check digit computes to 10', () => {
        // 0136282963 → weighted sum mod 11 = 1 → check 10 → invalid.
        expect(isValidNhsNumber('013 628 2963')).toBe(false);
    });
});
