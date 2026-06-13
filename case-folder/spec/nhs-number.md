# NHS Number rules

> Part of the [Case Tracking specification](index.md). Both editions
> implement the **identical** algorithm: the Loco edition in
> [`src/nhs.rs`](../case-folder-service-with-rust/src/nhs.rs), the Svelte
> edition in [`src/lib/store/nhs.ts`](../case-folder-front-end-with-svelte/src/lib/store/nhs.ts).
> The API is authoritative; the client validates only for fast UX.

## Format

An NHS Number is **10 digits**, displayed in a `XXX XXX XXXX` grouping
(e.g. `943 476 5919`). Inputs are normalised to bare digits before
validation; either grouped or bare form is accepted in path/query params.

## Modulus 11 check

1. Take the first 9 digits.
2. Multiply each by a weight running from **10 down to 2** (digit 1 × 10,
   digit 2 × 9, …, digit 9 × 2).
3. Sum the products; take `sum mod 11`; the check digit is `11 − remainder`.
4. **`check == 11` → check digit is `0`.**
5. **`check == 10` → the number is invalid** (no valid number produces 10).
6. The number is valid iff the computed check digit equals the 10th digit.

## Worked examples

| Input          | Digits     | Weighted sum | Sum mod 11 | Check | Valid? |
| -------------- | ---------- | ------------ | ---------- | ----- | ------ |
| `943 476 5919` | 9434765919 | 299          | 2          | 9     | ✓      |
| `987 654 3210` | 9876543210 | 330          | 0          | 0     | ✓      |
| `999 999 9999` | 9999999999 | 486          | 2          | 9     | ✓      |
| `943 476 5918` | 9434765918 | —            | —          | ≠8    | ✗      |
| `614 309 0431` | 6143090431 | 185          | 9          | 2     | ✗      |
| `013 628 2963` | 0136282963 | —            | 1          | 10    | ✗      |

## Helper contract

Both editions expose the same three operations:

- `normalise(input) -> digits` — strip spaces/punctuation to 10 bare digits.
- `format(digits) -> "XXX XXX XXXX"` — group for display (also formats
  partial input for live typing).
- `isValid(input) -> bool` — normalise, then run Modulus 11.

**Never compare formatted strings directly** — always normalise first.
