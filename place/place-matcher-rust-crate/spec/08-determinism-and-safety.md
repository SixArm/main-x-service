## 8. Determinism and safety

The crate MUST satisfy the following:

1. **No `unsafe`** — enforced by `#![forbid(unsafe_code)]` in `lib.rs`.
2. **`#![deny(missing_docs)]`** — every public item carries a `///` doc comment.
3. **No IO** — no `std::fs`, no `std::net`, no `tokio`, no `reqwest`, no `tracing`, no `log` from library code. `src/main.rs` is a demo binary (prints to stdout) and is **not** part of the library API.
4. **No global state** — no `static mut`, no `OnceCell` holding place data, no thread-local caches.
5. **No clocks, no RNGs, no environment-variable reads** — same inputs always produce the same outputs, byte-for-byte.
6. **No panics on user data** — library code MUST use `Option` / `Result` and MUST NOT unwrap on caller-derived values. The matching engine itself is infallible.
7. **`Send + Sync`** — every public type is `Send + Sync`. `MatchingEngine` is immutable after construction and cheap to clone.
8. **Serde-round-trippable** — every public data type MUST round-trip through `serde_json` (and any other `serde` format).

---

