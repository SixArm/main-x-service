// NHS Number utilities for e2e tests.
//
// `generate()` produces a unique Modulus-11-valid 10-digit NHS Number
// so multiple test runs against the same API don't collide on
// patient identity. `format()` mirrors the production formatter.

export function generate(): string {
    while (true) {
        const first9 = Array.from({ length: 9 }, () => Math.floor(Math.random() * 10));
        let sum = 0;
        for (let i = 0; i < 9; i++) sum += first9[i] * (10 - i);
        const remainder = sum % 11;
        const check = 11 - remainder;
        if (check === 10) continue; // would need a 10 in a single digit — restart
        const last = check === 11 ? 0 : check;
        return [...first9, last].join('');
    }
}

export function format(digits: string): string {
    const d = digits.replace(/\D/g, '').slice(0, 10);
    if (d.length <= 3) return d;
    if (d.length <= 6) return `${d.slice(0, 3)} ${d.slice(3)}`;
    return `${d.slice(0, 3)} ${d.slice(3, 6)} ${d.slice(6)}`;
}

// Known invalid NHS Number (last digit deliberately wrong vs Modulus 11).
export const INVALID = '943 476 5918';

// Seed-data NHS Numbers populated by `cargo run -- task seed`.
// These match the constants in
// `../case-tracker-service-with-rust/src/tasks/seed.rs`.
export const SEED = {
    alice: '943 476 5919',
    bob: '987 654 3210',
    carol: '999 999 9999',
    david: '614 309 0432',
    eleanor: '630 162 4483',
    frank: '485 777 3457'
} as const;

export function slug(formatted: string): string {
    return formatted.replaceAll(' ', '');
}
