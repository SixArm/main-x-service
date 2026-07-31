//! Closed token vocabularies for the string-typed domain columns
//! (CMS-D2 keeps those columns plain strings so a vocabulary can grow
//! by data migration, not DDL).

/// Site delivery visibility. `public` is the **only** value that puts a
/// site's published delivery reads on the anonymous allow-list
/// (spec `auth.md`); `restricted` requires a credential like everything
/// else.
pub const VISIBILITIES: &[&str] = &["public", "restricted"];

/// Default `robots` directives a site may declare.
pub const ROBOTS: &[&str] = &[
    "index,follow",
    "index,nofollow",
    "noindex,follow",
    "noindex,nofollow",
];

/// Editorial statuses of a variant (the lifecycle machine lands with
/// CMS-T11; the vocabulary is fixed here so the audit and event kinds
/// agree from the start).
pub const VARIANT_STATUSES: &[&str] = &["draft", "in_review", "approved", "published", "archived"];

/// Translation statuses of a variant (CMS-T15).
pub const TRANSLATION_STATUSES: &[&str] = &["requested", "in_translation", "translated"];

/// Rendition states. A rendition is *declared* until something
/// produces bytes for it; delivery serves only `produced` ones, so a
/// declared variant never becomes a URL that 404s.
pub const RENDITION_STATES: &[&str] = &["declared", "produced", "failed"];

/// Asset kinds (CMS-T8).
pub const ASSET_KINDS: &[&str] = &["image", "video", "audio", "document", "other"];

/// Block kinds permitted in a block document (CMS-T6).
pub const BLOCK_KINDS: &[&str] = &[
    "heading",
    "paragraph",
    "list",
    "quote",
    "code",
    "image",
    "embed",
    "callout",
    "divider",
    "reference",
];

/// Whether `value` is a member of the closed set `set`.
#[must_use]
pub fn is_token(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_is_exact_and_case_sensitive() {
        assert!(is_token(VISIBILITIES, "public"));
        assert!(!is_token(VISIBILITIES, "Public"));
        assert!(!is_token(VISIBILITIES, "world-readable"));
        assert!(is_token(VARIANT_STATUSES, "in_review"));
        assert!(!is_token(VARIANT_STATUSES, ""));
    }

    /// The visibility vocabulary is exactly two values: the public
    /// delivery allow-list keys off `public`, so a third value silently
    /// added here would widen an anonymous read path.
    #[test]
    fn visibility_vocabulary_is_closed_at_two() {
        assert_eq!(VISIBILITIES, ["public", "restricted"]);
    }
}
