## 13. References

- `src/lib.rs` — re-exports and top-level crate docs.
- `examples/basic_usage.rs`, `examples/custom_config.rs` — runnable end-to-end examples.
- `benches/match_pair.rs` — criterion harness exercising hot paths.
- `tests/integration_tests.rs`, `tests/property_tests.rs`, `tests/adapter_contract.rs` — pinned behaviour suite (§9 Adapter-Contract Tests).
- `fuzz/` — `cargo-fuzz` coverage-guided targets (SEC-I2); standalone, not a workspace member.
- `CHANGELOG.md` — version history, including tracked-but-unimplemented design work (e.g. `setting`/`tags`, §3.1.3).
- `AGENTS.md` and `AGENTS/*.md` — contributor and agent guidance.
- `index.md` — documentation entry point.

