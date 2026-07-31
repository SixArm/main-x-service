//! Integration-test crate root: pulls in the request-level suites
//! under [`requests`]. These boot the app against the `test`
//! environment (needs PostgreSQL via `config/test.yaml` /
//! `DATABASE_URL`) and are `#[ignore]`d by default — run with
//! `cargo test -- --ignored`. The auth-activation matrix lives in the
//! sibling `tests/enforcement.rs` target (its own process, because
//! `CMS_REQUIRE_AUTH` is cached in a `OnceLock`).

mod requests;
