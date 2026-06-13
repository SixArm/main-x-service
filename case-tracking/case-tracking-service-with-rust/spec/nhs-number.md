# NHS Number rules

> Part of the [Loco edition specification](index.md). The algorithm,
> format, and worked examples are shared:
> [root nhs-number](../../spec/nhs-number.md).

Identical to the Svelte sibling — Modulus 11 with weights 10..2,
`check == 10` → invalid, `check == 11` → digit 0. Reference
implementation in [`src/nhs.rs`](../src/nhs.rs); covered by unit tests in
the same module.

## Worked examples (Rust)

The Rust validator returns the same answers as the
[shared worked examples](../../spec/nhs-number.md#worked-examples).

```rust
use case_tracking_with_loco::nhs::*;

assert_eq!(format_nhs_number("9434765919"), "943 476 5919");
assert!(is_valid_nhs_number("943 476 5919"));
assert!(!is_valid_nhs_number("943 476 5918"));
```
