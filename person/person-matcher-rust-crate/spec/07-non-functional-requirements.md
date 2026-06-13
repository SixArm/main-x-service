## 7. Non-Functional Requirements

- **NFR-1** Performance — single pairwise match MUST complete in microseconds on commodity hardware.
- **NFR-2** Memory — no persistent allocations between calls; bounded per-call allocations proportional to input size.
- **NFR-3** Concurrency — all public types MUST be `Send + Sync` where their fields permit; engine is immutable post-construction.
- **NFR-4** Stability — public API MUST follow SemVer; pre-1.0 minors MAY break (document in CHANGELOG).
- **NFR-5** Determinism — see FR-11.
- **NFR-6** No IO — no file / network / stdio from library code (only `main.rs` demo prints).
- **NFR-7** No `unsafe` blocks.
- **NFR-8 / NFR-9** `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` MUST pass.
- **NFR-10** All public items MUST have rustdoc; doctests MUST compile.
- **NFR-11** i18n — Latin-script diacritics handled via NFKD; the same pipeline copes with any Unicode combining mark without per-language special-casing.
- **NFR-12** `cargo test` MUST pass on a fresh checkout with no environment variables.

---

