//! Request-level test suites, grouped by controller: the `/api/cases`
//! suite in [`cases`] and the durable event-bus Phase-2 outbox atomicity
//! suite in [`event_outbox`].

mod cases;
mod entity_links;
mod event_outbox;
mod review_queue;

use std::sync::Once;

/// Point the full-text index at a per-process temp directory, and switch
/// off the boot rebuild.
///
/// Must run before the first request that writes a record, because the
/// engine reads `CASE_SEARCH_INDEX_PATH` once into a `OnceLock`. Without
/// it the suite would index into the crate's working directory and
/// accumulate documents across runs; the boot rebuild would also race
/// the test body's own writes.
pub(crate) fn isolate_search_index() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("case-service-test-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // `set_var` is `unsafe` in edition 2024; single-threaded setup.
        unsafe {
            std::env::set_var("CASE_SEARCH_INDEX_PATH", &dir);
            std::env::set_var("CASE_SEARCH_BOOT_REINDEX", "0");
        }
    });
}
