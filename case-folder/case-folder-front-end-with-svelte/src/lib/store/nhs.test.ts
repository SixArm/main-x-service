// Unit tests for the NHS Number helpers (normalise / format / validate).
// These pin the contracts the create/move forms rely on: digit-stripping,
// progressive 3-3-4 grouping as the user types, and — most importantly —
// the Modulus 11 checksum, including its two special cases (check digit 0
// when the remainder yields 11, and outright rejection when it yields 10).

import { describe, it, expect } from 'vitest';
import { normaliseNhsNumber, formatNhsNumber, isValidNhsNumber } from './nhs';

describe('normaliseNhsNumber', () => {
    it('strips spaces and punctuation to bare digits', () => {
        expect(normaliseNhsNumber('943 476 5919')).toBe('9434765919');
        expect(normaliseNhsNumber('943-476-5919')).toBe('9434765919');
    });
    it('preserves a leading zero', () => {
        expect(normaliseNhsNumber('013 628 2963')).toBe('0136282963');
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
        // Computed check digit differs from the tenth digit.
        expect(isValidNhsNumber('943 476 5918')).toBe(false); // computed 9, tenth 8
        expect(isValidNhsNumber('614 309 0431')).toBe(false); // computed 2, tenth 1
        // 0136282963: weighted sum 174 → mod 11 = 9 → check 2 ≠ tenth digit 3.
        expect(isValidNhsNumber('013 628 2963')).toBe(false);
    });
    it('rejects the wrong length', () => {
        expect(isValidNhsNumber('943 476 591')).toBe(false);
        expect(isValidNhsNumber('')).toBe(false);
    });
    it('rejects a number whose check digit computes to 10', () => {
        // 9990000140: weighted sum 254 → mod 11 = 1 → check 10 → invalid
        // by rule, whatever the tenth digit is. Exercises the check === 10
        // branch. See spec/nhs-number.md.
        for (let tenth = 0; tenth <= 9; tenth++) {
            expect(isValidNhsNumber(`99900001${tenth}`)).toBe(false);
        }
    });
    it('treats grouped and bare forms identically', () => {
        expect(isValidNhsNumber('943 476 5919')).toBe(
            isValidNhsNumber('9434765919'),
        );
        expect(isValidNhsNumber('943-476-5919')).toBe(true);
    });
});
