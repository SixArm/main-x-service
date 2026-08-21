//! SEC-I2 fuzz target: the `EntityType` / `EdgeKind` wire-token registry.
//!
//! Both tokens arrive from the wire (`?kind=`, an edge row, a link body).
//! The invariants pinned here are the ones a silent registry edit would
//! break:
//!
//! - `from_token` never panics and never accepts a token outside `ALL`;
//! - the token round-trips (`from_token(k.as_str()) == Some(k)`), so a
//!   value written by one service is readable by another;
//! - `permits` is total, and a **symmetric** kind permits an ordered pair
//!   only if it permits the reverse — the aggregator canonicalises the
//!   pair order, so an asymmetric `permits` on a symmetric kind would
//!   accept an edge on write and reject its own stored form on read.

#![no_main]

use entity_ref::{EdgeKind, EntityType};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Arbitrary tokens must be rejected cleanly, and an accepted one must
    // be a member of the closed registry.
    if let Some(t) = EntityType::from_token(s) {
        assert!(EntityType::ALL.contains(&t), "from_token invented a type");
        assert_eq!(t.as_str(), s, "accepted token is not the type's own token");
    }
    if let Some(k) = EdgeKind::from_token(s) {
        assert!(EdgeKind::ALL.contains(&k), "from_token invented a kind");
        assert_eq!(k.as_str(), s, "accepted token is not the kind's own token");
    }

    for k in EdgeKind::ALL {
        assert_eq!(EdgeKind::from_token(k.as_str()), Some(k));
        // A symmetric kind is its own inverse, so it must carry no
        // inverse label; an asymmetric one must carry one, or the far
        // endpoint has nothing to store.
        assert_eq!(
            k.is_symmetric(),
            k.inverse().is_none(),
            "{k}: symmetry and inverse disagree"
        );

        for from in EntityType::ALL {
            for to in EntityType::ALL {
                let ok = k.permits(from, to);
                if k.is_symmetric() {
                    assert_eq!(
                        ok,
                        k.permits(to, from),
                        "symmetric {k} permits {from}->{to} but not the reverse"
                    );
                }
            }
        }
    }

    for t in EntityType::ALL {
        assert_eq!(EntityType::from_token(t.as_str()), Some(t));
        assert!(!t.service().is_empty(), "{t} has no owning service");
    }
});
