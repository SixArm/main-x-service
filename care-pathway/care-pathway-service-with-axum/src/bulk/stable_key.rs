//! Care-pathway's declared bulk-import **stable key**
//! (`agents/share/bulk-import-export.md` §10.1; crate spec §9.4).
//!
//! The stable key is what drives idempotent upsert: on import, a row
//! whose stable key matches an existing record updates it in place
//! rather than creating a duplicate, so re-running the same file is a
//! no-op.
//!
//! Care-pathway's precedence (most-to-least specific):
//!
//! 1. A **deterministic scheme-scoped identifier** — the same schemes the
//!    matcher's own R-0 short-circuit treats as globally unique
//!    (`IdentifierScheme::is_deterministic`: DOI, Wikidata,
//!    `GuidelineId`, URI, UUID) — with a non-empty value. Tried in
//!    declaration order (`Doi` → `Wikidata` → `GuidelineId` → `Uri` →
//!    `Uuid`); the shared doc does not mandate an order among several
//!    deterministic schemes on one row, so this crate declares one
//!    rather than leaving it to iteration order.
//! 2. Failing that, the **provider-scoped pathway code** — the
//!    `(provider_id, pathway_code)` pair, both present and non-blank
//!    after trimming. A pathway code is only unique *within* its
//!    provider (mirroring the matcher's own R-1 short-circuit and
//!    `agents/share/overview.md`'s "provider-scoped pathway code"
//!    description), so the pair together is the key, never
//!    `pathway_code` alone.
//! 3. Failing that, the row's own **explicit `pid`** — this is what makes
//!    re-importing an *export* (which always carries pids) idempotent
//!    even when a pathway has neither a strong identifier nor a
//!    provider-scoped code recorded.
//!
//! A row satisfying none of the above is **keyless** ([`is_keyless`]):
//! there is no target to upsert onto, so the import pipeline routes it
//! through duplicate detection instead of a blind create (§6).

use care_pathway_matcher::{CarePathway, IdentifierScheme};
use uuid::Uuid;

/// A resolved stable key for one import row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StableKey {
    /// Match on a scheme-scoped identifier `(scheme, value)`.
    Identifier {
        /// The identifier scheme (one of the five deterministic schemes
        /// this resolver considers).
        scheme: IdentifierScheme,
        /// The identifier value.
        value: String,
    },
    /// Match on the provider-scoped `(provider_id, pathway_code)` pair.
    ProviderPathwayCode {
        /// The issuing provider's identifier.
        provider_id: String,
        /// The provider's local pathway code.
        pathway_code: String,
    },
    /// Match on the record's own pid.
    Pid(Uuid),
}

/// Trim and treat a blank string as absent.
fn trimmed_non_empty(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

/// Resolve the stable key for `(pid, pathway)` per the precedence above.
///
/// Returns `None` exactly when the row is [`is_keyless`] — no
/// deterministic identifier, no provider-scoped pathway code, and no
/// explicit pid.
#[must_use]
pub fn resolve_stable_key(pid: Option<Uuid>, pathway: &CarePathway) -> Option<StableKey> {
    for scheme in [
        IdentifierScheme::Doi,
        IdentifierScheme::Wikidata,
        IdentifierScheme::GuidelineId,
        IdentifierScheme::Uri,
        IdentifierScheme::Uuid,
    ] {
        if let Some(id) = pathway
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
    if let (Some(provider_id), Some(pathway_code)) = (
        trimmed_non_empty(pathway.provider_id.as_deref()),
        trimmed_non_empty(pathway.pathway_code.as_deref()),
    ) {
        return Some(StableKey::ProviderPathwayCode {
            provider_id: provider_id.to_string(),
            pathway_code: pathway_code.to_string(),
        });
    }
    pid.map(StableKey::Pid)
}

/// Whether `(pid, pathway)`'s row is **keyless**
/// (`agents/share/bulk-import-export.md` §6): no deterministic
/// identifier, no provider-scoped pathway code, and no explicit `pid` of
/// its own.
///
/// A keyless row cannot be idempotently upserted — there is no stable
/// target to find — so the import pipeline routes it through duplicate
/// detection instead of a blind create.
#[must_use]
pub fn is_keyless(pid: Option<Uuid>, pathway: &CarePathway) -> bool {
    resolve_stable_key(pid, pathway).is_none()
}

#[cfg(test)]
mod tests {
    use super::{StableKey, is_keyless, resolve_stable_key};
    use care_pathway_matcher::{CarePathway, IdentifierScheme, PathwayIdentifier};
    use uuid::Uuid;

    fn ident(scheme: IdentifierScheme, value: &str) -> PathwayIdentifier {
        PathwayIdentifier {
            scheme,
            value: value.to_string(),
        }
    }

    #[test]
    fn prefers_the_first_deterministic_scheme_in_declared_order() {
        let pid = Uuid::new_v4();
        let pathway = CarePathway {
            identifiers: vec![
                ident(IdentifierScheme::Uri, "urn:example:1"),
                ident(IdentifierScheme::Doi, "10.1000/xyz123"),
            ],
            ..CarePathway::new("Acute Stroke Pathway")
        };
        assert_eq!(
            resolve_stable_key(Some(pid), &pathway),
            Some(StableKey::Identifier {
                scheme: IdentifierScheme::Doi,
                value: "10.1000/xyz123".to_string(),
            }),
            "Doi is declared before Uri"
        );
    }

    #[test]
    fn falls_back_to_provider_pathway_code_when_no_deterministic_identifier() {
        let pathway = CarePathway {
            provider_id: Some("nhs-trust-1".to_string()),
            pathway_code: Some("STROKE-01".to_string()),
            ..CarePathway::new("Acute Stroke Pathway")
        };
        assert_eq!(
            resolve_stable_key(None, &pathway),
            Some(StableKey::ProviderPathwayCode {
                provider_id: "nhs-trust-1".to_string(),
                pathway_code: "STROKE-01".to_string(),
            })
        );
    }

    #[test]
    fn falls_back_to_explicit_pid_when_no_strong_key() {
        let pid = Uuid::new_v4();
        let pathway = CarePathway::new("Acute Stroke Pathway");
        assert_eq!(
            resolve_stable_key(Some(pid), &pathway),
            Some(StableKey::Pid(pid))
        );
    }

    #[test]
    fn blank_provider_id_or_pathway_code_does_not_combine() {
        let pid = Uuid::new_v4();
        for (provider_id, pathway_code) in [
            (Some("   "), Some("STROKE-01")),
            (Some("nhs-trust-1"), Some("  ")),
            (None, Some("STROKE-01")),
            (Some("nhs-trust-1"), None),
        ] {
            let pathway = CarePathway {
                provider_id: provider_id.map(str::to_string),
                pathway_code: pathway_code.map(str::to_string),
                ..CarePathway::new("Acute Stroke Pathway")
            };
            assert_eq!(
                resolve_stable_key(Some(pid), &pathway),
                Some(StableKey::Pid(pid)),
                "a blank/absent provider_id or pathway_code must not combine"
            );
        }
    }

    #[test]
    fn ignores_blank_identifier_value() {
        let pid = Uuid::new_v4();
        let pathway = CarePathway {
            identifiers: vec![ident(IdentifierScheme::Doi, "   ")],
            ..CarePathway::new("Acute Stroke Pathway")
        };
        assert_eq!(
            resolve_stable_key(Some(pid), &pathway),
            Some(StableKey::Pid(pid))
        );
    }

    #[test]
    fn ignores_non_deterministic_schemes() {
        // A PathwayCode identifier entry is provider-scoped, not one of
        // the five deterministic schemes — it must not be picked up by
        // tier 1, even though `(provider_id, pathway_code)` (tier 2) is a
        // separate, real key.
        let pid = Uuid::new_v4();
        let pathway = CarePathway {
            identifiers: vec![ident(IdentifierScheme::PathwayCode, "STROKE-01")],
            ..CarePathway::new("Acute Stroke Pathway")
        };
        assert_eq!(
            resolve_stable_key(Some(pid), &pathway),
            Some(StableKey::Pid(pid))
        );
    }

    #[test]
    fn keyless_only_without_any_key() {
        let with_doi = CarePathway {
            identifiers: vec![ident(IdentifierScheme::Doi, "10.1000/xyz123")],
            ..CarePathway::new("Acute Stroke Pathway")
        };
        assert!(!is_keyless(None, &with_doi));

        let bare = CarePathway::new("Acute Stroke Pathway");
        assert!(is_keyless(None, &bare), "no key at all ⇒ keyless");
        assert!(
            !is_keyless(Some(Uuid::new_v4()), &bare),
            "an explicit pid is a real key even with no strong identifier"
        );
    }
}
