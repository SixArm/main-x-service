//! Request-level (HTTP) integration tests. Declares the organizations
//! suite, which boots the real loco app against the `test` config and is
//! therefore `#[ignore]`-gated on PostgreSQL.

use std::sync::Once;

use loco_rs::app::AppContext;
use serde_json::json;

mod bulk;
mod event_outbox;
mod fhir;
mod organizations;

/// Seed an organization **directly through the model layer**
/// (`streaming::create_and_emit`), bypassing `POST /api/organizations`'s
/// real-time duplicate check (ORG-T3) entirely.
///
/// Several fixtures below deliberately create near-duplicate (or
/// merely near-identical-looking) rows to exercise `merge`/
/// `deduplicate`/search/pagination — but ORG-T3's create-time check
/// runs the exact same matcher `is_match` call those features test, so
/// a pair similar enough to matter to them (or simply differing only
/// by a trailing digit in an otherwise-shared name) is, by
/// construction, also similar enough to now `409` on a second `POST`.
/// This mirrors person-service's own precedent for the identical
/// problem (see `create_minimal_person`'s callers there): seed past the
/// guard rather than let old fixtures collide with the new feature.
pub(crate) async fn seed_directly(
    ctx: &AppContext,
    payload: serde_json::Value,
) -> serde_json::Value {
    let org: organization_matcher::Organization =
        serde_json::from_value(payload).expect("payload should deserialize as an Organization");
    let model = organization_service::streaming::create_and_emit(&ctx.db, &org, None)
        .await
        .expect("direct seed should succeed");
    json!({ "pid": model.pid.to_string(), "name": model.name })
}

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
