//! Request-level test suites, grouped by controller: the
//! `/api/plans` plan suite in [`plans`], the durable
//! outbox suite in [`event_outbox`], and the PPM Phase-A governance
//! suite (intake / gates / risks / budgets) in [`governance`].

mod capabilities;
mod event_outbox;
mod governance;
mod insights;
mod plans;
mod strategy;
mod tba;
mod visibility;

use std::sync::Once;

/// Point the full-text index at a per-process temp directory, and switch
/// off the boot rebuild.
///
/// Must run before the first request that writes a record, because the
/// engine reads `PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_INDEX_PATH` once
/// into a `OnceLock` — shared across every test in this binary, not just
/// [`plans`]'s. Without it the suite would index into the crate's
/// working directory and accumulate documents across runs; the boot
/// rebuild would also race the test bodies' own writes.
pub(crate) fn isolate_search_index() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!(
            "project-portfolio-management-service-test-index-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        // `set_var` is `unsafe` in edition 2024; single-threaded setup.
        unsafe {
            std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_INDEX_PATH", &dir);
            std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_BOOT_REINDEX", "0");
        }
    });
}
