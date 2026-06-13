# NHS Number rules

> Part of the [Svelte edition specification](index.md). The algorithm,
> format, and worked examples are shared:
> [root nhs-number](../../spec/nhs-number.md).

Client-side validation in
[`src/lib/store/nhs.ts`](../src/lib/store/nhs.ts) — Modulus 11 with
weights 10..2, `check == 10` → invalid, `check == 11` → digit 0.
**Pre-flight only**; the Loco API runs the identical validator in
[`src/nhs.rs`](../../case-folder-service-with-rust/src/nhs.rs) and is
authoritative.

## Helper usage

- Always **format on display** (`formatNhsNumber`).
- Always **pre-flight validate** before submitting (`isValidNhsNumber`).
- Never compare formatted strings directly — **normalise first**
  (`normaliseNhsNumber`).

See the [shared worked examples](../../spec/nhs-number.md#worked-examples)
for the canonical valid/invalid table.
