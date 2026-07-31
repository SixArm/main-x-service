//! Reference extraction (CMS-R5, CMS-D8) — pure and DB-free.
//!
//! On every save the core walks the block document and the field values
//! and returns every entry, asset, and `EntityRef` the revision points
//! at. The controller writes those as `content_references` rows **in
//! the revision's transaction**, which is what makes three things true
//! at once:
//!
//! - "Where used" is an index lookup, not a scan of every document.
//! - A delete that would break a live reference can be **refused**
//!   rather than discovered later by a reader hitting a gap.
//! - A broken reference (target archived, unpublished, or missing) is a
//!   derived finding (CMS-R21) instead of a 404.
//!
//! Extraction is deliberately **syntactic**: it records what the
//! document says it points at, without checking that the target exists.
//! Existence is the controller's business (and, for cross-service
//! `EntityRef`s, may not be knowable locally at all) — while the
//! *edge itself* is a fact about the content, and stays true even when
//! the target is missing. That is precisely the case a
//! broken-reference report needs to be able to see.

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::rules::schema::FieldSpec;

/// What a reference points at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// Another entry in this service.
    Entry(Uuid),
    /// An asset in the library.
    Asset(Uuid),
    /// A record in another family service, by URN.
    Entity(String),
}

impl Target {
    /// The stored `kind` token.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Entry(_) => "entry",
            Self::Asset(_) => "asset",
            Self::Entity(_) => "entity",
        }
    }
}

/// One extracted edge, with the path it came from so an editor can be
/// pointed at the block or field that carries it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reference {
    /// Where the reference points.
    pub target: Target,
    /// The originating path: a field key, or `blocks[3].asset`.
    pub field_key: String,
}

/// Extract every reference from a revision's blocks and field values.
///
/// Results are sorted and de-duplicated by (target, path), so
/// re-saving an unchanged document produces an identical edge set and
/// the same document referencing one asset twice in the same field
/// records it once.
#[must_use]
pub fn extract(
    blocks: &[Value],
    fields: &Map<String, Value>,
    specs: &[FieldSpec],
) -> Vec<Reference> {
    let mut found = Vec::new();
    extract_from_blocks(blocks, &mut found);
    extract_from_fields(fields, specs, &mut found);
    found.sort();
    found.dedup();
    found
}

/// Walk the block document.
fn extract_from_blocks(blocks: &[Value], found: &mut Vec<Reference>) {
    for (index, block) in blocks.iter().enumerate() {
        let Some(object) = block.as_object() else {
            continue;
        };
        match object.get("kind").and_then(Value::as_str) {
            Some("image") => {
                if let Some(asset) = object.get("asset").and_then(Value::as_str)
                    && let Ok(id) = Uuid::parse_str(asset)
                {
                    found.push(Reference {
                        target: Target::Asset(id),
                        field_key: format!("blocks[{index}].asset"),
                    });
                }
            }
            Some("reference") => {
                if let Some(entry) = object.get("entry").and_then(Value::as_str)
                    && let Ok(id) = Uuid::parse_str(entry)
                {
                    found.push(Reference {
                        target: Target::Entry(id),
                        field_key: format!("blocks[{index}].entry"),
                    });
                }
                if let Some(entity) = object.get("entity_ref").and_then(Value::as_str)
                    && entity.parse::<entity_ref::EntityRef>().is_ok()
                {
                    found.push(Reference {
                        target: Target::Entity(entity.to_string()),
                        field_key: format!("blocks[{index}].entity_ref"),
                    });
                }
            }
            _ => {}
        }
    }
}

/// Walk the typed field values, guided by the content type's declared
/// field kinds — so a `media` field is read as an asset reference and a
/// `text` field that happens to contain a UUID is not.
fn extract_from_fields(
    fields: &Map<String, Value>,
    specs: &[FieldSpec],
    found: &mut Vec<Reference>,
) {
    for spec in specs {
        let Some(value) = fields.get(&spec.key) else {
            continue;
        };
        let values: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            other => vec![other],
        };
        for item in values {
            let Some(text) = item.as_str() else { continue };
            let target = match spec.kind.as_str() {
                "media" => Uuid::parse_str(text).ok().map(Target::Asset),
                "reference" => Uuid::parse_str(text).ok().map(Target::Entry),
                "entity_ref" => text
                    .parse::<entity_ref::EntityRef>()
                    .ok()
                    .map(|_| Target::Entity(text.to_string())),
                _ => None,
            };
            if let Some(target) = target {
                found.push(Reference {
                    target,
                    field_key: spec.key.clone(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::schema::Validation;
    use serde_json::json;

    fn spec(key: &str, kind: &str, repeatable: bool) -> FieldSpec {
        FieldSpec {
            key: key.to_string(),
            label: key.to_string(),
            kind: kind.to_string(),
            required: false,
            repeatable,
            validation: Validation::default(),
        }
    }

    fn fields(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn blocks_yield_asset_entry_and_entity_edges() {
        let asset = Uuid::new_v4();
        let entry = Uuid::new_v4();
        let urn = format!("course:{}", Uuid::new_v4());
        let blocks = vec![
            json!({ "kind": "image", "asset": asset.to_string() }),
            json!({ "kind": "paragraph", "text": "no edges here" }),
            json!({ "kind": "reference", "entry": entry.to_string() }),
            json!({ "kind": "reference", "entity_ref": urn.clone() }),
        ];
        let found = extract(&blocks, &Map::new(), &[]);
        assert_eq!(found.len(), 3);
        assert!(
            found
                .iter()
                .any(|r| r.target == Target::Asset(asset) && r.field_key == "blocks[0].asset")
        );
        assert!(found.iter().any(|r| r.target == Target::Entry(entry)));
        assert!(
            found
                .iter()
                .any(|r| r.target == Target::Entity(urn.clone()))
        );
    }

    /// Field kinds decide what a value *means*: a UUID sitting in a
    /// text field is text, not a reference to something.
    #[test]
    fn only_reference_shaped_fields_yield_edges() {
        let asset = Uuid::new_v4();
        let specs = vec![spec("hero", "media", false), spec("note", "text", false)];
        let values = fields(&json!({
            "hero": asset.to_string(),
            "note": Uuid::new_v4().to_string(),
        }));
        let found = extract(&[], &values, &specs);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].target, Target::Asset(asset));
        assert_eq!(found[0].field_key, "hero");
    }

    #[test]
    fn repeatable_fields_yield_one_edge_per_value() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let specs = vec![spec("related", "reference", true)];
        let values = fields(&json!({ "related": [a.to_string(), b.to_string()] }));
        let found = extract(&[], &values, &specs);
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|r| r.target == Target::Entry(a)));
        assert!(found.iter().any(|r| r.target == Target::Entry(b)));
    }

    /// The same target twice in one place is one edge; the same target
    /// in two different places is two, because "where used" wants both
    /// locations.
    #[test]
    fn duplicates_collapse_per_path_but_not_across_paths() {
        let asset = Uuid::new_v4();
        let specs = vec![spec("gallery", "media", true), spec("hero", "media", false)];
        let values = fields(&json!({
            "gallery": [asset.to_string(), asset.to_string()],
            "hero": asset.to_string(),
        }));
        let found = extract(&[], &values, &specs);
        assert_eq!(found.len(), 2);
        assert_eq!(
            found
                .iter()
                .filter(|r| r.target == Target::Asset(asset))
                .count(),
            2
        );
    }

    /// Extraction never panics on malformed input, and skips values it
    /// cannot parse rather than inventing an edge.
    #[test]
    fn malformed_values_are_skipped_not_guessed() {
        let specs = vec![
            spec("hero", "media", false),
            spec("about", "entity_ref", false),
        ];
        let values = fields(&json!({ "hero": "not-a-uuid", "about": "not-a-urn" }));
        let blocks = vec![
            json!({ "kind": "image", "asset": "nope" }),
            json!({ "kind": "reference", "entry": 7 }),
            json!("not even an object"),
        ];
        assert!(extract(&blocks, &values, &specs).is_empty());
    }

    /// Re-extracting an unchanged document yields an identical set, so
    /// a re-save does not churn the edge index.
    #[test]
    fn extraction_is_stable() {
        let blocks = vec![json!({ "kind": "image", "asset": Uuid::new_v4().to_string() })];
        assert_eq!(
            extract(&blocks, &Map::new(), &[]),
            extract(&blocks, &Map::new(), &[])
        );
    }
}
