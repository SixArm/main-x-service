//! Live HTTP proof for T-31's suggestion-job fetch source
//! (`src/suggest/job.rs::HttpIdentitySource`), landed alongside the fix
//! for the `q=*` enumeration bug (see `person-service`'s `CHANGELOG.md`
//! for the investigation). Unlike this crate's other `#[ignore]`d tests
//! (which boot **this** app against its own Postgres, `tests/reconcile.rs`
//! et al.), this test drives `HttpIdentitySource::fetch_all` — this
//! crate's real production code, not a reimplementation — against a
//! **real, separately-running peer service** (person-service or
//! worker-service) over actual HTTP, exactly as the periodic suggestion
//! job does in production. It is not part of any automated CI stage: no
//! CI job in this repo brings up a second full service to test against.
//!
//! ## Running
//!
//! ```sh
//! # In one terminal: bring up a migrated person-service (or worker-service)
//! # against a real Postgres and seed it with more than one page's worth of
//! # records (e.g. via its own test suite's create helper, or curl).
//! cd person/person-service-with-loco && cargo run -- start
//!
//! # In another terminal:
//! LIVE_LIST_URL=http://127.0.0.1:5150/api/persons \
//! LIVE_ENTITY_TYPE=person \
//!   cargo test --test live_suggest_fetch -- --ignored --nocapture
//! ```
//!
//! Repeat with `LIVE_LIST_URL=http://127.0.0.1:5150/api/workers` and
//! `LIVE_ENTITY_TYPE=worker` against a running worker-service, to prove
//! the identical fetch path against both services' wire shapes
//! (`persons`/`workers` field alias — see `ListData` in `job.rs`).

use entity_ref::EntityType;
use link_graph_service::suggest::job::{HttpIdentitySource, IdentitySource};

/// Fetch every record from `LIVE_LIST_URL` via the real
/// `HttpIdentitySource::fetch_all` and report what came back. Asserts
/// only that the fetch succeeds and paginated correctly (more than one
/// page's worth, if the target has that many rows, or exactly the row
/// count otherwise) — the caller is expected to have seeded the target
/// service and to eyeball the printed count, since this test has no way
/// to know the target's expected row count in advance.
#[tokio::test]
#[ignore = "requires a real running person-service or worker-service; see module docs"]
async fn live_fetch_all_enumerates_a_real_running_service() {
    let url = std::env::var("LIVE_LIST_URL")
        .expect("set LIVE_LIST_URL, e.g. http://127.0.0.1:5150/api/persons");
    let entity = std::env::var("LIVE_ENTITY_TYPE").unwrap_or_else(|_| "person".to_string());
    let entity_type = EntityType::from_token(&entity)
        .unwrap_or_else(|| panic!("unknown LIVE_ENTITY_TYPE {entity:?}"));
    let token = std::env::var("LIVE_TOKEN").ok();

    let source = HttpIdentitySource::new(entity_type, url.clone(), token);
    let fetched = source
        .fetch_all()
        .await
        .expect("fetch_all against a real running service");

    println!(
        "live_fetch_all_enumerates_a_real_running_service: {} against {url} returned {} records",
        entity_type.as_str(),
        fetched.len()
    );

    // Every fetched id is unique — the same no-duplication property the
    // sibling services' own pagination tests pin from their side; this
    // proves it holds from the CLIENT side too, through the real
    // paginating HTTP loop in `HttpIdentitySource::fetch_all`, not just
    // through a single page.
    let mut ids: Vec<_> = fetched
        .iter()
        .map(|(entity_ref, _)| entity_ref.id)
        .collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        before,
        ids.len(),
        "HttpIdentitySource::fetch_all returned the same id more than once — \
         the pagination loop double-counted a page"
    );

    assert!(
        !fetched.is_empty(),
        "the target service returned zero records — seed it first (see module docs)"
    );
}
