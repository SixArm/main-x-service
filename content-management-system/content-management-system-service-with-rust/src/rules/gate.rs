//! Publish gates (CMS-R11) — pure, DB-free.
//!
//! What stands between a variant and publication. Each blocker names
//! the rule, the offending thing, and what would clear it, because
//! "cannot publish" without a reason is the single most infuriating
//! message a CMS can show.
//!
//! ## Why these run at publish and not at save
//!
//! A draft is allowed to be incomplete — that is what a draft is. Every
//! check here would, applied at save time, make an editor fight the
//! system from the first keystroke. Applied at publish they are
//! actionable: the work is finished, and this is the list of what is
//! still missing.
//!
//! ## Alt text
//!
//! Missing alt text blocks publication (spec `regulatory.md`). It is
//! the one WCAG obligation a CMS can genuinely enforce at the source —
//! the rest (contrast, focus order, heading semantics) belongs to the
//! rendering channel — and enforcing it here is the difference between
//! an accessibility policy and an accessibility aspiration.
//!
//! The gate lands with the publish transition (CMS-T12). It is exposed
//! now as a **read** (`publish-check`) so an editor can see the list
//! before the transition exists, and the transition will call this same
//! function rather than reimplementing it.

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::rules::schema::{self, FieldSpec};

/// One reason a variant cannot be published.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Blocker {
    /// Stable rule key, so a client can group or explain them.
    pub rule: &'static str,
    /// What the rule is about: a field key, an asset id, a path.
    pub subject: String,
    /// What would clear it, in words an editor can act on.
    pub remedy: String,
}

/// An asset referenced by the variant, as the caller resolved it.
#[derive(Debug, Clone)]
pub struct ReferencedAsset {
    /// The asset id the content points at.
    pub pid: Uuid,
    /// Whether the asset still exists (live, not soft-deleted).
    pub exists: bool,
    /// Its kind, when it exists.
    pub kind: Option<String>,
    /// Its alt text, when it exists.
    pub alt_text: Option<String>,
}

/// An entry referenced by the variant, as the caller resolved it.
#[derive(Debug, Clone)]
pub struct ReferencedEntry {
    /// The entry id the content points at.
    pub pid: Uuid,
    /// Whether the entry still exists (live, not soft-deleted).
    pub exists: bool,
    /// Its key, when it exists.
    pub key: Option<String>,
}

/// Everything the gate needs, already resolved by the caller — so the
/// rules stay pure and testable without a database.
#[derive(Debug, Clone)]
pub struct Candidate<'a> {
    /// Whether this content type is routable (has an address at all).
    pub routable: bool,
    /// Whether this variant currently has one.
    pub has_path: bool,
    /// The revision's title.
    pub title: &'a str,
    /// Its typed field values.
    pub fields: &'a Map<String, Value>,
    /// The content type's declared fields.
    pub specs: &'a [FieldSpec],
    /// Referenced assets, resolved.
    pub assets: &'a [ReferencedAsset],
    /// Referenced entries, resolved.
    pub entries: &'a [ReferencedEntry],
}

/// Every reason this candidate cannot be published (empty ⇒ ready).
#[must_use]
pub fn publish_blockers(candidate: &Candidate<'_>) -> Vec<Blocker> {
    let mut blockers = Vec::new();

    if candidate.title.trim().is_empty() {
        blockers.push(Blocker {
            rule: "title_required",
            subject: "title".to_string(),
            remedy: "give the entry a title".to_string(),
        });
    }

    // A routable page with no address would publish into nowhere: the
    // sitemap could not list it, a menu could not link to it, and
    // delivery could not find it (CMS-R11).
    if candidate.routable && !candidate.has_path {
        blockers.push(Blocker {
            rule: "route_missing",
            subject: "path".to_string(),
            remedy: "give the page an address before publishing it".to_string(),
        });
    }

    for key in schema::missing_required(candidate.specs, candidate.fields) {
        blockers.push(Blocker {
            rule: "required_field_empty",
            subject: key.clone(),
            remedy: format!("fill in {key}"),
        });
    }

    for asset in candidate.assets {
        if !asset.exists {
            blockers.push(Blocker {
                rule: "reference_missing",
                subject: asset.pid.to_string(),
                remedy: "the referenced asset no longer exists; remove or replace it".to_string(),
            });
            continue;
        }
        // Alt text is required for images specifically: a PDF or an
        // audio file has no equivalent obligation, and demanding one
        // would train editors to type nonsense into the box.
        if asset.kind.as_deref() == Some("image")
            && asset
                .alt_text
                .as_ref()
                .is_none_or(|text| text.trim().is_empty())
        {
            blockers.push(Blocker {
                rule: "image_alt_text_missing",
                subject: asset.pid.to_string(),
                remedy: "describe the image in its alt text (a screen reader has nothing else)"
                    .to_string(),
            });
        }
    }

    for entry in candidate.entries {
        if !entry.exists {
            blockers.push(Blocker {
                rule: "reference_missing",
                subject: entry.pid.to_string(),
                remedy: "the referenced entry no longer exists; remove or replace the link"
                    .to_string(),
            });
        }
    }

    blockers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::schema::Validation;
    use serde_json::json;

    fn spec(key: &str, required: bool) -> FieldSpec {
        FieldSpec {
            key: key.to_string(),
            label: key.to_string(),
            kind: "text".to_string(),
            required,
            repeatable: false,
            validation: Validation::default(),
        }
    }

    fn image(alt: Option<&str>) -> ReferencedAsset {
        ReferencedAsset {
            pid: Uuid::new_v4(),
            exists: true,
            kind: Some("image".to_string()),
            alt_text: alt.map(ToString::to_string),
        }
    }

    fn candidate<'a>(
        title: &'a str,
        fields: &'a Map<String, Value>,
        specs: &'a [FieldSpec],
        assets: &'a [ReferencedAsset],
        entries: &'a [ReferencedEntry],
    ) -> Candidate<'a> {
        Candidate {
            routable: false,
            has_path: false,
            title,
            fields,
            specs,
            assets,
            entries,
        }
    }

    #[test]
    fn a_complete_candidate_has_no_blockers() {
        let fields = json!({ "body": "words" }).as_object().cloned().unwrap();
        let specs = vec![spec("body", true)];
        let assets = vec![image(Some("A photograph of the building"))];
        assert!(publish_blockers(&candidate("Title", &fields, &specs, &assets, &[])).is_empty());
    }

    #[test]
    fn empty_required_fields_block_with_their_key() {
        let fields = json!({ "body": "   " }).as_object().cloned().unwrap();
        let specs = vec![spec("body", true), spec("optional", false)];
        let blockers = publish_blockers(&candidate("Title", &fields, &specs, &[], &[]));
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].rule, "required_field_empty");
        assert_eq!(blockers[0].subject, "body");
    }

    /// The accessibility gate, and the reason it exists.
    #[test]
    fn an_image_without_alt_text_blocks_publication() {
        let fields = Map::new();
        for alt in [None, Some(""), Some("   ")] {
            let assets = vec![image(alt)];
            let blockers = publish_blockers(&candidate("Title", &fields, &[], &assets, &[]));
            assert_eq!(blockers.len(), 1, "alt {alt:?} should block");
            assert_eq!(blockers[0].rule, "image_alt_text_missing");
            assert!(blockers[0].remedy.contains("screen reader"));
        }
    }

    /// Only images: demanding alt text for a PDF would train editors to
    /// type nonsense into the box.
    #[test]
    fn non_images_do_not_need_alt_text() {
        let fields = Map::new();
        let assets = vec![ReferencedAsset {
            pid: Uuid::new_v4(),
            exists: true,
            kind: Some("document".to_string()),
            alt_text: None,
        }];
        assert!(publish_blockers(&candidate("Title", &fields, &[], &assets, &[])).is_empty());
    }

    #[test]
    fn missing_reference_targets_block_and_are_not_double_reported() {
        let fields = Map::new();
        let assets = vec![ReferencedAsset {
            pid: Uuid::new_v4(),
            exists: false,
            kind: None,
            alt_text: None,
        }];
        let entries = vec![ReferencedEntry {
            pid: Uuid::new_v4(),
            exists: false,
            key: None,
        }];
        let blockers = publish_blockers(&candidate("Title", &fields, &[], &assets, &entries));
        assert_eq!(blockers.len(), 2);
        assert!(blockers.iter().all(|b| b.rule == "reference_missing"));
        // A missing asset is reported once, not also as missing alt text.
        assert!(!blockers.iter().any(|b| b.rule == "image_alt_text_missing"));
    }

    #[test]
    fn a_blank_title_blocks() {
        let fields = Map::new();
        let blockers = publish_blockers(&candidate("   ", &fields, &[], &[], &[]));
        assert_eq!(blockers[0].rule, "title_required");
    }

    /// Every blocker carries a remedy: "cannot publish" with no next
    /// step is the message this whole module exists to avoid.
    #[test]
    fn every_blocker_names_a_remedy() {
        let fields = Map::new();
        let specs = vec![spec("body", true)];
        let assets = vec![image(None)];
        let blockers = publish_blockers(&candidate("", &fields, &specs, &assets, &[]));
        assert_eq!(blockers.len(), 3);
        for blocker in &blockers {
            assert!(!blocker.rule.is_empty());
            assert!(!blocker.subject.is_empty());
            assert!(!blocker.remedy.trim().is_empty());
        }
    }

    /// A routable page with no address would publish into nowhere.
    #[test]
    fn a_routable_page_needs_an_address() {
        let fields = Map::new();
        let mut candidate = candidate("Title", &fields, &[], &[], &[]);
        candidate.routable = true;
        candidate.has_path = false;
        let blockers = publish_blockers(&candidate);
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].rule, "route_missing");
        assert!(blockers[0].remedy.contains("address"));

        candidate.has_path = true;
        assert!(publish_blockers(&candidate).is_empty());

        // A non-routable type has no address to be missing.
        candidate.routable = false;
        candidate.has_path = false;
        assert!(publish_blockers(&candidate).is_empty());
    }
}
