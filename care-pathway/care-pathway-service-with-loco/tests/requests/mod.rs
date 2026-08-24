//! Request-level (HTTP) integration tests, grouped by controller.

mod care_pathways;
mod compliance;
mod event_outbox;
mod insights;
mod instances;
mod links;
mod tba;

use std::sync::Once;

/// Point the full-text index at a per-process temp directory, and switch
/// off the boot rebuild.
///
/// Must run before the first request that writes a record, because the
/// engine reads `CARE_PATHWAY_SEARCH_INDEX_PATH` once into a `OnceLock`.
/// Without it the suite would index into the crate's working directory
/// and accumulate documents across runs; the boot rebuild would also
/// race the test body's own writes.
pub(crate) fn isolate_search_index() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!(
            "care-pathway-service-test-index-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        // `set_var` is `unsafe` in edition 2024; single-threaded setup.
        unsafe {
            std::env::set_var("CARE_PATHWAY_SEARCH_INDEX_PATH", &dir);
            std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
        }
    });
}
