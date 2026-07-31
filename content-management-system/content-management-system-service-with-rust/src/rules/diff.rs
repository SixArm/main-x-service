//! Revision diff (CMS-R3) — pure and DB-free.
//!
//! History you cannot read is not much use: the point of an append-only
//! chain is that an editor can see what a save actually changed, and
//! decide whether to restore.
//!
//! This is a **positional** diff: blocks are compared by index, so
//! inserting a block at the top reports that block as added and the
//! rest as changed. It is not a move-detecting or longest-common-
//! subsequence diff, and the type says so rather than implying a
//! precision it does not have. A smarter alignment is a later
//! improvement that changes no stored data.

use serde_json::{Map, Value};

/// What happened to one position or key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    /// Present after, absent before.
    Added,
    /// Present before, absent after.
    Removed,
    /// Present in both, different.
    Changed,
}

/// One changed block position.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlockChange {
    /// The block index this concerns.
    pub index: usize,
    /// What happened.
    pub change: Change,
    /// The block kind before, when there was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_before: Option<String>,
    /// The block kind after, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_after: Option<String>,
}

/// One changed field key.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldChange {
    /// The field key.
    pub key: String,
    /// What happened.
    pub change: Change,
}

/// The difference between two revisions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Diff {
    /// Whether the title changed.
    pub title_changed: bool,
    /// Whether the SEO block changed.
    pub seo_changed: bool,
    /// Block-level changes, in index order.
    pub blocks: Vec<BlockChange>,
    /// Field-level changes, in key order.
    pub fields: Vec<FieldChange>,
    /// Whether anything changed at all.
    pub identical: bool,
    /// How the block comparison works, stated in the payload so a
    /// reader is not left to infer it from the numbers.
    pub block_comparison: &'static str,
}

/// One side of a diff.
#[derive(Debug, Clone, Copy)]
pub struct Side<'a> {
    /// The revision's title.
    pub title: &'a str,
    /// Its block document.
    pub blocks: &'a [Value],
    /// Its typed field values.
    pub fields: &'a Map<String, Value>,
    /// Its SEO block.
    pub seo: &'a Value,
}

/// Diff `from` → `to`.
#[must_use]
pub fn diff(from: Side<'_>, to: Side<'_>) -> Diff {
    let mut blocks = Vec::new();
    let kind = |block: Option<&Value>| {
        block
            .and_then(|b| b.get("kind"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    };
    for index in 0..from.blocks.len().max(to.blocks.len()) {
        let before = from.blocks.get(index);
        let after = to.blocks.get(index);
        let change = match (before, after) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(_), Some(_)) => Change::Changed,
            (None, Some(_)) => Change::Added,
            (Some(_), None) => Change::Removed,
            (None, None) => continue,
        };
        blocks.push(BlockChange {
            index,
            change,
            kind_before: kind(before),
            kind_after: kind(after),
        });
    }

    let mut fields = Vec::new();
    let mut keys: Vec<&String> = from.fields.keys().chain(to.fields.keys()).collect();
    keys.sort();
    keys.dedup();
    for key in keys {
        let before = from.fields.get(key);
        let after = to.fields.get(key);
        let change = match (before, after) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(_), Some(_)) => Change::Changed,
            (None, Some(_)) => Change::Added,
            (Some(_), None) => Change::Removed,
            (None, None) => continue,
        };
        fields.push(FieldChange {
            key: key.clone(),
            change,
        });
    }

    let title_changed = from.title != to.title;
    let seo_changed = from.seo != to.seo;
    Diff {
        identical: !title_changed && !seo_changed && blocks.is_empty() && fields.is_empty(),
        title_changed,
        seo_changed,
        blocks,
        fields,
        block_comparison: "positional: blocks are compared by index, not aligned by content",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap_or_default()
    }

    fn side<'a>(
        title: &'a str,
        blocks: &'a [Value],
        fields: &'a Map<String, Value>,
        seo: &'a Value,
    ) -> Side<'a> {
        Side {
            title,
            blocks,
            fields,
            seo,
        }
    }

    #[test]
    fn an_unchanged_revision_diffs_to_nothing() {
        let blocks = vec![json!({ "kind": "paragraph", "text": "same" })];
        let fields = map(&json!({ "a": 1 }));
        let seo = json!({ "meta_title": "t" });
        let d = diff(
            side("Title", &blocks, &fields, &seo),
            side("Title", &blocks, &fields, &seo),
        );
        assert!(d.identical);
        assert!(d.blocks.is_empty() && d.fields.is_empty());
        assert!(!d.title_changed && !d.seo_changed);
    }

    #[test]
    fn block_changes_report_index_and_kinds() {
        let before = vec![
            json!({ "kind": "paragraph", "text": "one" }),
            json!({ "kind": "paragraph", "text": "two" }),
        ];
        let after = vec![
            json!({ "kind": "heading", "text": "one", "level": 2 }),
            json!({ "kind": "paragraph", "text": "two" }),
            json!({ "kind": "divider" }),
        ];
        let fields = Map::new();
        let seo = json!({});
        let d = diff(
            side("t", &before, &fields, &seo),
            side("t", &after, &fields, &seo),
        );
        assert_eq!(d.blocks.len(), 2);
        assert_eq!(d.blocks[0].change, Change::Changed);
        assert_eq!(d.blocks[0].kind_before.as_deref(), Some("paragraph"));
        assert_eq!(d.blocks[0].kind_after.as_deref(), Some("heading"));
        assert_eq!(d.blocks[1].change, Change::Added);
        assert_eq!(d.blocks[1].index, 2);
        assert!(!d.identical);
    }

    #[test]
    fn removed_blocks_are_reported() {
        let before = vec![json!({ "kind": "divider" }), json!({ "kind": "divider" })];
        let after = vec![json!({ "kind": "divider" })];
        let fields = Map::new();
        let seo = json!({});
        let d = diff(
            side("t", &before, &fields, &seo),
            side("t", &after, &fields, &seo),
        );
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].change, Change::Removed);
        assert_eq!(d.blocks[0].kind_before.as_deref(), Some("divider"));
        assert!(d.blocks[0].kind_after.is_none());
    }

    #[test]
    fn field_changes_cover_added_removed_and_changed() {
        let blocks: Vec<Value> = Vec::new();
        let before = map(&json!({ "keep": 1, "drop": 2, "edit": 3 }));
        let after = map(&json!({ "keep": 1, "edit": 4, "new": 5 }));
        let seo = json!({});
        let d = diff(
            side("t", &blocks, &before, &seo),
            side("t", &blocks, &after, &seo),
        );
        let by_key = |k: &str| d.fields.iter().find(|f| f.key == k).map(|f| f.change);
        assert_eq!(by_key("drop"), Some(Change::Removed));
        assert_eq!(by_key("edit"), Some(Change::Changed));
        assert_eq!(by_key("new"), Some(Change::Added));
        assert_eq!(by_key("keep"), None);
    }

    #[test]
    fn title_and_seo_changes_are_reported_separately() {
        let blocks: Vec<Value> = Vec::new();
        let fields = Map::new();
        let d = diff(
            side(
                "Before",
                &blocks,
                &fields,
                &json!({ "robots": "index,follow" }),
            ),
            side(
                "After",
                &blocks,
                &fields,
                &json!({ "robots": "noindex,follow" }),
            ),
        );
        assert!(d.title_changed);
        assert!(d.seo_changed);
        assert!(!d.identical);
    }

    /// The payload states how blocks were compared, so nobody reads a
    /// positional diff as a content-aligned one.
    #[test]
    fn the_comparison_method_is_disclosed() {
        let blocks: Vec<Value> = Vec::new();
        let fields = Map::new();
        let seo = json!({});
        let d = diff(
            side("t", &blocks, &fields, &seo),
            side("t", &blocks, &fields, &seo),
        );
        assert!(d.block_comparison.contains("positional"));
    }
}
