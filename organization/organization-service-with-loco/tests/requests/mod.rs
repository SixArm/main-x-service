//! Request-level (HTTP) integration tests. Declares the organizations
//! suite, which boots the real loco app against the `test` config and is
//! therefore `#[ignore]`-gated on PostgreSQL.

use std::sync::Once;

mod bulk;
mod event_outbox;
mod organizations;

/// Point the full-text index at a per-process temp directory.
///
/// Must run before the first request that writes a record, because the
/// engine reads `ORGANIZATION_SEARCH_INDEX_PATH` once into a
/// `OnceLock`; every test therefore calls this first (the `Once` makes
/// repeats free). Without it the suite would index into the crate's
/// working directory and **accumulate documents across runs** — stale
/// hits still resolve to nothing, so results stay correct, but they
/// consume slots in the result cap and would eventually crowd out the
/// record a test just created.
pub(crate) fn isolate_search_index() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!(
            "organization-service-test-index-{}",
            std::process::id()
        ));
        // Start from an empty index even if a previous run with the same
        // pid left one behind.
        let _ = std::fs::remove_dir_all(&dir);
        // `set_var` is `unsafe` in edition 2024; single-threaded setup,
        // before any thread reads the variable.
        unsafe {
            std::env::set_var("ORGANIZATION_SEARCH_INDEX_PATH", &dir);
            // Switch off the boot-time rebuild for the suite. It fires on
            // every `request::<App, _, _>` boot, sees whatever rows the
            // previous run left behind, and then races the test body's
            // own index writes — which made an earlier version of the
            // rebuild test pass even with the rebuild removed. The
            // rebuild has its own test, which calls it directly.
            std::env::set_var("ORGANIZATION_SEARCH_BOOT_REINDEX", "0");
        }
    });
}
