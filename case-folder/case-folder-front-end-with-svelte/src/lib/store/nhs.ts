// NHS Number utilities.
//
// The UK NHS Number is a 10-digit identifier displayed as "XXX XXX XXXX".
// The tenth digit is a Modulus 11 check digit calculated over the first nine
// digits with weights 10..2. A computed remainder of 10 means the number is
// invalid; a remainder of 11 yields a check digit of 0.
//
// Reference: https://en.wikipedia.org/wiki/NHS_number

/**
 * Strip everything except digits from an NHS Number string.
 *
 * Used to turn user/display input (e.g. "943 476 5919", "943-476-5919")
 * into the bare 10 digits that validation and API lookups expect. Leading
 * zeros are preserved.
 *
 * @param raw - Any string that may contain an NHS Number with spaces/punctuation.
 * @returns The digits only, in order.
 */
export function normaliseNhsNumber(raw: string): string {
    return raw.replace(/\D/g, '');
}

/**
 * Format an NHS Number for display as the canonical "XXX XXX XXXX" groups.
 *
 * Partial input is grouped progressively as the user types (e.g. "943476"
 * → "943 476"), and input longer than 10 digits is truncated to 10. Always
 * normalises first, so spaced/punctuated input is accepted.
 *
 * @param raw - Raw or partial NHS Number input.
 * @returns The number grouped 3-3-4 (or the partial prefix grouped so far).
 */
export function formatNhsNumber(raw: string): string {
    const digits = normaliseNhsNumber(raw).slice(0, 10);
    if (digits.length <= 3) return digits;
    if (digits.length <= 6) return `${digits.slice(0, 3)} ${digits.slice(3)}`;
    return `${digits.slice(0, 3)} ${digits.slice(3, 6)} ${digits.slice(6)}`;
}

/**
 * Validate an NHS Number using the official Modulus 11 check-digit rule.
 *
 * The number must be exactly 10 digits. The first nine digits are each
 * multiplied by a descending weight (10, 9, …, 2); the weighted sum modulo
 * 11 gives a remainder. The expected check digit is `11 - remainder`, with
 * two special cases:
 *   - a result of 11 means the check digit is 0;
 *   - a result of 10 is invalid (no such NHS Number exists).
 * The number is valid only when this computed check digit equals the
 * actual tenth digit.
 *
 * @param raw - The NHS Number to test (spaces/punctuation tolerated).
 * @returns `true` if the number is 10 digits and the checksum matches.
 */
export function isValidNhsNumber(raw: string): boolean {
    const digits = normaliseNhsNumber(raw);
    if (digits.length !== 10) return false;

    // Weighted sum of the first nine digits: weights 10 down to 2.
    let total = 0;
    for (let i = 0; i < 9; i++) {
        total += Number(digits[i]) * (10 - i);
    }
    const remainder = total % 11;
    const check = 11 - remainder;
    // check === 10 → invalid by rule; check === 11 → check digit is 0.
    if (check === 10) return false;
    const checkDigit = check === 11 ? 0 : check;
    return checkDigit === Number(digits[9]);
}
