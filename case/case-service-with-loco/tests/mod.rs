//! Integration-test crate root: pulls in the request-level suites under
//! [`requests`]. The DB-free matcher/storage checks live in the sibling
//! `tests/matching.rs` target; the request suites here need PostgreSQL and
//! are `#[ignore]`d by default.

mod requests;
