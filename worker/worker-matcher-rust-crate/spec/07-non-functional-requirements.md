## 7. Non-Functional Requirements

**NFR-1 Performance** — pairwise match MUST complete in microseconds on commodity hardware. **NFR-2 Memory** — no persistent allocations between calls; bounded per-call allocations proportional to input size. **NFR-3 Concurrency** — public types MUST be `Send + Sync` where fields permit; engine is immutable after construction. **NFR-4 Stability** — public API MUST follow SemVer; pre-1.0 minors MAY break (documented in CHANGELOG). **NFR-5 Determinism** — see FR-11. **NFR-6 No IO** — library code MUST NOT perform file / network / stdin / stdout / stderr IO (only `main.rs` demo may print). **NFR-7 No unsafe** — no `unsafe` blocks. **NFR-8 Linting** — `cargo clippy --all-targets -- -D warnings` MUST pass. **NFR-9 Formatting** — `cargo fmt --check` MUST pass. **NFR-10 Documentation** — all public items MUST have rustdoc; doctests MUST compile. **NFR-11 i18n** — Latin-script diacritics handled via NFKD; the pipeline SHOULD cope with any Unicode combining mark without per-language special-casing. **NFR-12 Reproducibility** — `cargo test` MUST pass on a fresh checkout with no environment variables.

---

