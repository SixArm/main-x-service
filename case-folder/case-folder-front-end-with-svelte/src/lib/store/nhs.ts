// NHS Number utilities.
//
// The UK NHS Number is a 10-digit identifier displayed as "XXX XXX XXXX".
// The tenth digit is a Modulus 11 check digit calculated over the first nine
// digits with weights 10..2. A computed remainder of 10 means the number is
// invalid; a remainder of 11 yields a check digit of 0.
//
// Reference: https://en.wikipedia.org/wiki/NHS_number

export function normaliseNhsNumber(raw: string): string {
    return raw.replace(/\D/g, '');
}

export function formatNhsNumber(raw: string): string {
    const digits = normaliseNhsNumber(raw).slice(0, 10);
    if (digits.length <= 3) return digits;
    if (digits.length <= 6) return `${digits.slice(0, 3)} ${digits.slice(3)}`;
    return `${digits.slice(0, 3)} ${digits.slice(3, 6)} ${digits.slice(6)}`;
}

export function isValidNhsNumber(raw: string): boolean {
    const digits = normaliseNhsNumber(raw);
    if (digits.length !== 10) return false;

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
