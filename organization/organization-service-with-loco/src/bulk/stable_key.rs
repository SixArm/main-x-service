//! Organization's declared bulk-import **stable key**
//! (`agents/share/bulk-import-export.md` §10.1; crate spec §10.7).
//!
//! The stable key is what drives idempotent upsert: on import, a row
//! whose stable key matches an existing record updates it in place
//! rather than creating a duplicate, so re-running the same file is a
//! no-op.
//!
//! Organization's precedence (most-to-least specific), reflecting that
//! LEI and DUNS are both **deterministic** identifiers
//! (`organization_matcher::IdentifierScheme::is_deterministic` —
//! globally unique by construction) while `pid` is merely
//! server-assigned:
//!
//! 1. An **LEI** identifier (`IdentifierScheme::Lei`) with a non-empty
//!    value.
//! 2. Failing that, a **DUNS** identifier (`IdentifierScheme::Duns`)
//!    with a non-empty value.
//! 3. Failing that, the row's own **explicit `pid`**
//!    ([`super::columns::from_row_value`]'s `had_explicit_pid`) — this is
//!    what makes re-importing an *export* (which carries pids) idempotent.
//!
//! A row satisfying none of the above is **keyless** ([`is_keyless`]):
//! there is no target to upsert onto, so the import pipeline routes it
//! through duplicate detection instead of a blind create (§6).

use organization_matcher::{IdentifierScheme, Organization};
use uuid::Uuid;

/// A resolved stable key for one import row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StableKey {
    /// Match on the record's own pid.
    Pid(Uuid),
    /// Match on a scheme-scoped identifier `(scheme, value)`.
    Identifier {
        /// The identifier scheme (`Lei` or `Duns` — the only schemes this
        /// resolver considers).
        scheme: IdentifierScheme,
        /// The identifier value.
        value: String,
    },
}

/// Resolve the stable key for `(pid, org)` per the precedence above.
///
/// Returns `None` exactly when the row is [`is_keyless`] — no LEI, no
/// DUNS, and no explicit pid.
#[must_use]
pub fn resolve_stable_key(pid: Option<Uuid>, org: &Organization) -> Option<StableKey> {
    for scheme in [IdentifierScheme::Lei, IdentifierScheme::Duns] {
        if let Some(id) = org
            .identifiers
            .iter()
            .find(|i| i.scheme == scheme && !i.value.trim().is_empty())
        {
            return Some(StableKey::Identifier {
                scheme: id.scheme.clone(),
                value: id.value.clone(),
            });
        }
    }
    pid.map(StableKey::Pid)
}

/// Whether `(pid, org)`'s row is **keyless**
/// (`agents/share/bulk-import-export.md` §6): no LEI, no DUNS, and no
/// explicit `pid` of its own.
///
/// A keyless row cannot be idempotently upserted — there is no stable
/// target to find — so the import pipeline routes it through duplicate
/// detection instead of a blind create.
#[must_use]
pub fn is_keyless(pid: Option<Uuid>, org: &Organization) -> bool {
    resolve_stable_key(pid, org).is_none()
}

#[cfg(test)]
mod tests {
    use super::{StableKey, is_keyless, resolve_stable_key};
    use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization};
    use uuid::Uuid;

    fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme,
            value: value.to_string(),
        }
    }

    #[test]
    fn prefers_lei_over_duns_and_pid() {
        let pid = Uuid::new_v4();
        let org = Organization {
            identifiers: vec![
                ident(IdentifierScheme::Duns, "150483782"),
                ident(IdentifierScheme::Lei, "5493001KJTIIGC8Y1R12"),
            ],
            ..Organization::new("Acme")
        };
        assert_eq!(
            resolve_stable_key(Some(pid), &org),
            Some(StableKey::Identifier {
                scheme: IdentifierScheme::Lei,
                value: "5493001KJTIIGC8Y1R12".to_string(),
            })
        );
    }

    #[test]
    fn falls_back_to_duns_when_no_lei() {
        let org = Organization {
            identifiers: vec![ident(IdentifierScheme::Duns, "150483782")],
            ..Organization::new("Acme")
        };
        assert_eq!(
            resolve_stable_key(None, &org),
            Some(StableKey::Identifier {
                scheme: IdentifierScheme::Duns,
                value: "150483782".to_string(),
            })
        );
    }

    #[test]
    fn falls_back_to_explicit_pid_when_no_strong_identifier() {
        let pid = Uuid::new_v4();
        let org = Organization::new("Acme");
        assert_eq!(
            resolve_stable_key(Some(pid), &org),
            Some(StableKey::Pid(pid))
        );
    }

    #[test]
    fn ignores_blank_identifier_value() {
        let pid = Uuid::new_v4();
        let org = Organization {
            identifiers: vec![ident(IdentifierScheme::Lei, "   ")],
            ..Organization::new("Acme")
        };
        assert_eq!(
            resolve_stable_key(Some(pid), &org),
            Some(StableKey::Pid(pid))
        );
    }

    #[test]
    fn ignores_other_schemes() {
        // A Wikidata or VAT identifier is deterministic in the matcher's
        // own sense, but is NOT one of this crate's declared stable-key
        // schemes (§10.7 names only LEI/DUNS) — it must not be picked up.
        let pid = Uuid::new_v4();
        let org = Organization {
            identifiers: vec![ident(IdentifierScheme::Wikidata, "Q312")],
            ..Organization::new("Acme")
        };
        assert_eq!(
            resolve_stable_key(Some(pid), &org),
            Some(StableKey::Pid(pid))
        );
    }

    #[test]
    fn keyless_only_without_any_key() {
        let org_with_lei = Organization {
            identifiers: vec![ident(IdentifierScheme::Lei, "5493001KJTIIGC8Y1R12")],
            ..Organization::new("Acme")
        };
        assert!(!is_keyless(None, &org_with_lei));

        let bare = Organization::new("Acme");
        assert!(is_keyless(None, &bare), "no identifier, no pid ⇒ keyless");
        assert!(
            !is_keyless(Some(Uuid::new_v4()), &bare),
            "an explicit pid is a real key even with no strong identifier"
        );
    }
}
