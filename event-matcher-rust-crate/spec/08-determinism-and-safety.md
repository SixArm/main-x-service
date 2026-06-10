## 8. Determinism and safety

The crate MUST satisfy the following:

1. **No `unsafe`.** Enforced by `#![forbid(unsafe_code)]` in `lib.rs`.
2. **`#![deny(missing_docs)]`.** Every public item carries a `///` doc comment; the lint denies builds otherwise.
3. **No IO.** No `std::fs`, no `std::net`, no `tokio`, no `reqwest`, no `tracing`, no `log` from library code. `src/main.rs` is a demo binary and prints to stdout; it is **not** part of the library API.
4. **No global state.** No `static mut`, no `OnceCell` holding place data, no thread-local caches.
5. **No clocks, no RNGs, no environment-variable reads.** Same inputs always produce the same outputs, byte-for-byte.
6. **No panics on user data.** Library code MUST use `Option` / `Result` and MUST NOT unwrap on values derived from caller input. The matching engine itself is infallible.
7. **`Send + Sync`.** Every public type is `Send + Sync`. `MatchingEngine` is immutable after construction and cheap to clone.
8. **Serde-round-trippable.** Every public data type MUST round-trip through `serde_json` (and any other `serde` format).

---

