//! SEC-I2 fuzz target: `EntityRef` URN parsing.
//!
//! An `EntityRef` arrives as one opaque string — a `to_ref` column, a
//! link-creation request body, a graph query path segment — so
//! `FromStr` runs on input this family does not control. This pins:
//!
//! - **never-panic** on arbitrary UTF-8, and
//! - **round-trip**: whatever parses must `Display` back to something
//!   that parses again to the identical value. A `Display` that lost or
//!   rewrote a byte would silently repoint an edge at a different record,
//!   which no unit test over hand-picked strings would notice.

#![no_main]

use entity_ref::EntityRef;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(parsed) = s.parse::<EntityRef>() else {
        // A rejection is a valid outcome; the invariant is only that we
        // got here without aborting.
        return;
    };

    // Display must be re-parseable, and to the same value.
    let rendered = parsed.to_string();
    let reparsed = rendered
        .parse::<EntityRef>()
        .expect("Display output must re-parse");
    assert_eq!(parsed, reparsed, "round-trip changed the ref: {rendered:?}");
    assert_eq!(
        rendered,
        reparsed.to_string(),
        "Display is not idempotent: {rendered:?}"
    );

    // The type→service map is total over parsed refs: an owning service
    // that came back empty would route a link write nowhere.
    assert!(
        !parsed.service().is_empty(),
        "parsed ref has no owning service: {rendered:?}"
    );
});
