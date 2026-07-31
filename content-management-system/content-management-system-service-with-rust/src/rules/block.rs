//! Block documents (CMS-R4, CMS-D5): the structured body of a
//! revision, validated and sanitized at the write boundary. Pure and
//! DB-free.
//!
//! A body is an ordered list of typed blocks with structured inline
//! marks — never stored markup. Three properties fall out of that and
//! are worth naming, because they are the reasons for the shape:
//!
//! 1. **Safety.** There is no markup to smuggle. The one place HTML may
//!    appear (an `embed` block's `html`) is sanitized on write
//!    ([`crate::rules::sanitize`]) and re-escaped by the channel.
//! 2. **Portability.** The same document renders to a web page, an app
//!    screen, or a kiosk panel — the point of headless delivery.
//! 3. **Queryability.** Blocks are walkable, which is what makes
//!    reference extraction and "where used" possible at all
//!    ([`crate::rules::reference`]).
//!
//! **Unknown block kinds and unknown keys are refused, never dropped.**
//! Silently discarding part of what an author sent is the failure mode
//! where work disappears between the editor and the database, and the
//! author finds out after publishing.

use serde_json::Value;

use crate::rules::sanitize;
use crate::rules::tokens::BLOCK_KINDS;

/// Maximum blocks in one document.
pub const MAX_BLOCKS: usize = 512;
/// Maximum characters in one block's text.
pub const MAX_BLOCK_TEXT: usize = 8192;
/// Maximum items in a list block.
pub const MAX_LIST_ITEMS: usize = 256;
/// Maximum inline marks on one block.
pub const MAX_MARKS: usize = 256;
/// Maximum nesting depth of any JSON inside a block (a deeply nested
/// payload is a parser-time cost, not a document).
pub const MAX_JSON_DEPTH: usize = 8;
/// Maximum characters of embedded HTML in one `embed` block.
pub const MAX_EMBED_HTML: usize = 16_384;

/// Inline mark kinds permitted on a block's text.
pub const MARK_KINDS: &[&str] = &["strong", "em", "code", "link"];

/// Callout tones.
pub const CALLOUT_TONES: &[&str] = &["info", "warn", "success", "danger"];

/// The keys each block kind accepts, beyond the mandatory `kind`.
/// Anything else is refused by name (see the module docs).
fn permitted_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "heading" => &["text", "level", "marks"],
        "paragraph" => &["text", "marks"],
        "list" => &["items", "ordered"],
        "quote" => &["text", "attribution", "marks"],
        "code" => &["text", "language"],
        "image" => &["asset", "alt", "caption"],
        "embed" => &["url", "html", "provider", "caption"],
        "callout" => &["tone", "text", "marks"],
        "reference" => &["entry", "entity_ref", "label"],
        // `divider` carries no payload; an unknown kind is refused
        // before this is consulted.
        _ => &[],
    }
}

/// Validate a block document, returning every problem found (empty ⇒
/// valid). Problems name their path (`blocks[3].kind`) so an editor can
/// be pointed at the offending block rather than the whole document.
#[must_use]
pub fn validate_document(blocks: &[Value]) -> Vec<String> {
    let mut problems = Vec::new();
    if blocks.len() > MAX_BLOCKS {
        problems.push(format!("blocks exceeds {MAX_BLOCKS} entries"));
    }
    for (index, block) in blocks.iter().enumerate() {
        problems.extend(validate_block(index, block));
    }
    problems
}

/// Validate one block at `index`.
#[allow(clippy::too_many_lines)] // one match over the block vocabulary
fn validate_block(index: usize, block: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(object) = block.as_object() else {
        problems.push(format!("blocks[{index}] must be an object"));
        return problems;
    };
    let Some(kind) = object.get("kind").and_then(Value::as_str) else {
        problems.push(format!("blocks[{index}].kind is required"));
        return problems;
    };
    if !BLOCK_KINDS.contains(&kind) {
        problems.push(format!(
            "blocks[{index}].kind must be one of {BLOCK_KINDS:?}, got {kind:?}"
        ));
        return problems;
    }
    if json_depth(block) > MAX_JSON_DEPTH {
        problems.push(format!(
            "blocks[{index}] nests deeper than {MAX_JSON_DEPTH}"
        ));
    }
    let permitted = permitted_keys(kind);
    for key in object.keys() {
        if key != "kind" && !permitted.contains(&key.as_str()) {
            problems.push(format!(
                "blocks[{index}].{key} is not a key of a {kind} block (permitted: {permitted:?})"
            ));
        }
    }

    let text = |field: &str| object.get(field).and_then(Value::as_str);
    let require_text = |field: &str, problems: &mut Vec<String>| -> Option<String> {
        match text(field) {
            Some(value) if !value.trim().is_empty() => {
                if value.chars().count() > MAX_BLOCK_TEXT {
                    problems.push(format!(
                        "blocks[{index}].{field} exceeds {MAX_BLOCK_TEXT} characters"
                    ));
                }
                Some(value.to_string())
            }
            Some(_) => {
                problems.push(format!("blocks[{index}].{field} must not be blank"));
                None
            }
            None => {
                problems.push(format!("blocks[{index}].{field} is required"));
                None
            }
        }
    };

    match kind {
        "heading" => {
            let body = require_text("text", &mut problems);
            match object.get("level").and_then(Value::as_u64) {
                // `h1` is the page's own title, which the channel owns;
                // a body that could emit one would fight the layout and
                // wreck the document outline.
                Some(level) if (2..=6).contains(&level) => {}
                Some(level) => problems.push(format!(
                    "blocks[{index}].level must be 2–6 (h1 belongs to the page title), got {level}"
                )),
                None => problems.push(format!("blocks[{index}].level is required")),
            }
            problems.extend(validate_marks(index, object.get("marks"), body.as_deref()));
        }
        "paragraph" => {
            let body = require_text("text", &mut problems);
            problems.extend(validate_marks(index, object.get("marks"), body.as_deref()));
        }
        "quote" => {
            let body = require_text("text", &mut problems);
            if let Some(attribution) = text("attribution")
                && attribution.chars().count() > MAX_BLOCK_TEXT
            {
                problems.push(format!("blocks[{index}].attribution is too long"));
            }
            problems.extend(validate_marks(index, object.get("marks"), body.as_deref()));
        }
        "code" => {
            require_text("text", &mut problems);
        }
        "callout" => {
            let body = require_text("text", &mut problems);
            match text("tone") {
                Some(tone) if CALLOUT_TONES.contains(&tone) => {}
                Some(tone) => problems.push(format!(
                    "blocks[{index}].tone must be one of {CALLOUT_TONES:?}, got {tone:?}"
                )),
                None => problems.push(format!("blocks[{index}].tone is required")),
            }
            problems.extend(validate_marks(index, object.get("marks"), body.as_deref()));
        }
        "list" => match object.get("items").and_then(Value::as_array) {
            Some(items) if !items.is_empty() => {
                if items.len() > MAX_LIST_ITEMS {
                    problems.push(format!(
                        "blocks[{index}].items exceeds {MAX_LIST_ITEMS} entries"
                    ));
                }
                for (i, item) in items.iter().enumerate() {
                    match item.as_str() {
                        Some(value) if !value.trim().is_empty() => {
                            if value.chars().count() > MAX_BLOCK_TEXT {
                                problems.push(format!("blocks[{index}].items[{i}] is too long"));
                            }
                        }
                        _ => problems
                            .push(format!("blocks[{index}].items[{i}] must be non-blank text")),
                    }
                }
            }
            Some(_) => problems.push(format!("blocks[{index}].items must not be empty")),
            None => problems.push(format!("blocks[{index}].items is required")),
        },
        "image" => {
            match text("asset") {
                Some(asset) if uuid::Uuid::parse_str(asset).is_ok() => {}
                Some(asset) => problems.push(format!(
                    "blocks[{index}].asset must be an asset uuid, got {asset:?}"
                )),
                None => problems.push(format!("blocks[{index}].asset is required")),
            }
            // Alt text is *not* required here: an image block may be
            // drafted before its asset has alt text, and the gate that
            // matters runs at publish (CMS-R11), where refusing is
            // actionable rather than merely obstructive.
        }
        "embed" => {
            let has_url = match text("url") {
                Some(url) if url.starts_with("https://") => true,
                Some(url) => {
                    problems.push(format!(
                        "blocks[{index}].url must be an https URL, got {url:?}"
                    ));
                    false
                }
                None => false,
            };
            let has_html = text("html").is_some_and(|html| !html.trim().is_empty());
            if let Some(html) = text("html")
                && html.chars().count() > MAX_EMBED_HTML
            {
                problems.push(format!(
                    "blocks[{index}].html exceeds {MAX_EMBED_HTML} characters"
                ));
            }
            if !has_url && !has_html {
                problems.push(format!("blocks[{index}] requires a url or html"));
            }
        }
        "reference" => {
            let entry = text("entry");
            let entity = text("entity_ref");
            match (entry, entity) {
                (Some(_), Some(_)) => problems.push(format!(
                    "blocks[{index}] must reference an entry or an entity, not both"
                )),
                (None, None) => problems.push(format!(
                    "blocks[{index}] requires an entry or an entity_ref"
                )),
                (Some(entry), None) if uuid::Uuid::parse_str(entry).is_err() => problems.push(
                    format!("blocks[{index}].entry must be an entry uuid, got {entry:?}"),
                ),
                (None, Some(entity)) if entity.parse::<entity_ref::EntityRef>().is_err() => {
                    problems.push(format!(
                        "blocks[{index}].entity_ref is not a valid EntityRef URN: {entity:?}"
                    ));
                }
                _ => {}
            }
        }
        // `divider` carries nothing; the permitted-key check above
        // already refuses any payload.
        _ => {}
    }
    problems
}

/// Validate a block's inline marks against its text.
fn validate_marks(index: usize, marks: Option<&Value>, text: Option<&str>) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(marks) = marks else {
        return problems;
    };
    let Some(marks) = marks.as_array() else {
        problems.push(format!("blocks[{index}].marks must be an array"));
        return problems;
    };
    if marks.len() > MAX_MARKS {
        problems.push(format!("blocks[{index}].marks exceeds {MAX_MARKS} entries"));
    }
    let length = text.map_or(0, |t| t.chars().count() as u64);
    for (i, mark) in marks.iter().enumerate() {
        let Some(object) = mark.as_object() else {
            problems.push(format!("blocks[{index}].marks[{i}] must be an object"));
            continue;
        };
        let kind = object.get("kind").and_then(Value::as_str);
        match kind {
            Some(kind) if MARK_KINDS.contains(&kind) => {}
            Some(kind) => problems.push(format!(
                "blocks[{index}].marks[{i}].kind must be one of {MARK_KINDS:?}, got {kind:?}"
            )),
            None => problems.push(format!("blocks[{index}].marks[{i}].kind is required")),
        }
        // A mark that runs off the end of its text would be applied to
        // nothing — or, in a channel that clamps differently, to the
        // wrong words.
        let start = object.get("start").and_then(Value::as_u64);
        let end = object.get("end").and_then(Value::as_u64);
        match (start, end) {
            (Some(start), Some(end)) if start < end && end <= length => {}
            (Some(start), Some(end)) => problems.push(format!(
                "blocks[{index}].marks[{i}] range {start}..{end} does not fit the text (length {length})"
            )),
            _ => problems.push(format!(
                "blocks[{index}].marks[{i}] requires start and end offsets"
            )),
        }
        if kind == Some("link") {
            match object.get("href").and_then(Value::as_str) {
                Some(href)
                    if href.starts_with("https://")
                        || href.starts_with("http://")
                        || href.starts_with("mailto:")
                        || href.starts_with('/') => {}
                Some(href) => problems.push(format!(
                    "blocks[{index}].marks[{i}].href must be http(s), mailto, or a site-relative path, got {href:?}"
                )),
                None => problems.push(format!(
                    "blocks[{index}].marks[{i}].href is required for a link mark"
                )),
            }
        }
    }
    problems
}

/// Sanitize a document in place, returning how many blocks were
/// altered. Today that is the `embed` block's `html`; import will use
/// the same path.
///
/// Sanitizing **before** validation and storing the sanitized form is
/// the point: what is validated is what is stored, and what is stored
/// was never trusted markup.
pub fn sanitize_document(blocks: &mut [Value]) -> usize {
    let mut altered = 0;
    for block in blocks.iter_mut() {
        let Some(object) = block.as_object_mut() else {
            continue;
        };
        if object.get("kind").and_then(Value::as_str) != Some("embed") {
            continue;
        }
        let Some(html) = object.get("html").and_then(Value::as_str) else {
            continue;
        };
        let clean = sanitize::sanitize_html(html);
        if clean != html {
            altered += 1;
            object.insert("html".to_string(), Value::String(clean));
        }
    }
    altered
}

/// The nesting depth of a JSON value (a scalar is depth 1).
fn json_depth(value: &Value) -> usize {
    match value {
        Value::Object(map) => 1 + map.values().map(json_depth).max().unwrap_or(0),
        Value::Array(items) => 1 + items.iter().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_document() -> Vec<Value> {
        vec![
            json!({ "kind": "heading", "text": "A title", "level": 2 }),
            json!({ "kind": "paragraph", "text": "Some words here",
                    "marks": [{ "kind": "strong", "start": 0, "end": 4 }] }),
            json!({ "kind": "list", "items": ["one", "two"], "ordered": false }),
            json!({ "kind": "divider" }),
        ]
    }

    #[test]
    fn a_well_formed_document_validates() {
        let problems = validate_document(&valid_document());
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn unknown_kinds_are_refused_by_path() {
        let problems = validate_document(&[json!({ "kind": "markdown", "text": "x" })]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("blocks[0].kind must be one of"));
    }

    /// Unknown keys are refused rather than dropped: silently
    /// discarding an author's field is how work disappears.
    #[test]
    fn unknown_keys_are_refused_not_dropped() {
        let problems = validate_document(&[json!({
            "kind": "paragraph", "text": "x", "html": "<script>alert(1)</script>"
        })]);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("blocks[0].html is not a key of a paragraph block")),
            "{problems:?}"
        );
    }

    #[test]
    fn required_payloads_are_enforced() {
        let problems = validate_document(&[
            json!({ "kind": "paragraph" }),
            json!({ "kind": "heading", "text": "x" }),
            json!({ "kind": "list", "items": [] }),
            json!({ "kind": "callout", "text": "x" }),
        ]);
        assert!(problems.iter().any(|p| p == "blocks[0].text is required"));
        assert!(problems.iter().any(|p| p == "blocks[1].level is required"));
        assert!(
            problems
                .iter()
                .any(|p| p.contains("blocks[2].items must not be empty"))
        );
        assert!(problems.iter().any(|p| p == "blocks[3].tone is required"));
    }

    /// `h1` belongs to the page title, which the channel owns.
    #[test]
    fn heading_levels_stop_at_h2() {
        let problems = validate_document(&[json!({ "kind": "heading", "text": "x", "level": 1 })]);
        assert!(problems[0].contains("must be 2–6"));
        assert!(
            validate_document(&[json!({ "kind": "heading", "text": "x", "level": 6 })]).is_empty()
        );
        assert!(
            !validate_document(&[json!({ "kind": "heading", "text": "x", "level": 7 })]).is_empty()
        );
    }

    /// A mark that runs past its text would decorate the wrong words,
    /// or nothing at all.
    #[test]
    fn marks_must_fit_their_text() {
        let problems = validate_document(&[json!({
            "kind": "paragraph", "text": "four", "marks": [{ "kind": "em", "start": 2, "end": 99 }]
        })]);
        assert!(problems[0].contains("does not fit the text"));

        let problems = validate_document(&[json!({
            "kind": "paragraph", "text": "four", "marks": [{ "kind": "em", "start": 3, "end": 3 }]
        })]);
        assert!(
            problems[0].contains("does not fit the text"),
            "{problems:?}"
        );

        // Offsets are counted in characters, not bytes, so a multi-byte
        // text is measured the way an editor sees it.
        assert!(
            validate_document(&[json!({
                "kind": "paragraph", "text": "héllo", "marks": [{ "kind": "em", "start": 0, "end": 5 }]
            })])
            .is_empty()
        );
    }

    #[test]
    fn link_marks_require_a_safe_href() {
        let bad = validate_document(&[json!({
            "kind": "paragraph", "text": "link",
            "marks": [{ "kind": "link", "start": 0, "end": 4, "href": "javascript:alert(1)" }]
        })]);
        assert!(bad[0].contains("must be http(s), mailto, or a site-relative path"));

        for href in [
            "https://a.test",
            "http://a.test",
            "mailto:a@b.test",
            "/about",
        ] {
            let problems = validate_document(&[json!({
                "kind": "paragraph", "text": "link",
                "marks": [{ "kind": "link", "start": 0, "end": 4, "href": href }]
            })]);
            assert!(
                problems.is_empty(),
                "{href} should be allowed: {problems:?}"
            );
        }

        let missing = validate_document(&[json!({
            "kind": "paragraph", "text": "link", "marks": [{ "kind": "link", "start": 0, "end": 4 }]
        })]);
        assert!(missing[0].contains("href is required"));
    }

    #[test]
    fn references_are_exclusive_and_well_formed() {
        let entry = uuid::Uuid::new_v4().to_string();
        let urn = format!("course:{}", uuid::Uuid::new_v4());
        assert!(validate_document(&[json!({ "kind": "reference", "entry": entry })]).is_empty());
        assert!(
            validate_document(&[json!({ "kind": "reference", "entity_ref": urn.clone() })])
                .is_empty()
        );
        assert!(
            validate_document(&[json!({ "kind": "reference", "entry": entry, "entity_ref": urn })])
                [0]
            .contains("not both")
        );
        assert!(
            validate_document(&[json!({ "kind": "reference" })])[0].contains("requires an entry")
        );
        assert!(
            validate_document(&[json!({ "kind": "reference", "entry": "nope" })])[0]
                .contains("must be an entry uuid")
        );
    }

    #[test]
    fn embeds_require_https_and_something_to_embed() {
        assert!(
            validate_document(&[json!({ "kind": "embed", "url": "http://a.test/v" })])[0]
                .contains("must be an https URL")
        );
        assert!(
            validate_document(&[json!({ "kind": "embed" })])[0].contains("requires a url or html")
        );
        assert!(
            validate_document(&[json!({ "kind": "embed", "url": "https://a.test/v" })]).is_empty()
        );
    }

    #[test]
    fn caps_are_enforced() {
        let long = "x".repeat(MAX_BLOCK_TEXT + 1);
        assert!(
            validate_document(&[json!({ "kind": "paragraph", "text": long })])[0]
                .contains("exceeds")
        );
        let many: Vec<Value> = (0..=MAX_BLOCKS)
            .map(|_| json!({ "kind": "divider" }))
            .collect();
        assert!(validate_document(&many)[0].contains("exceeds"));
    }

    /// Sanitization runs before storage and reports that it changed
    /// something, so a caller is never told their markup was stored
    /// verbatim when it was not.
    #[test]
    fn embed_html_is_sanitized_in_place() {
        let mut blocks = vec![
            json!({ "kind": "embed", "html": "<p>ok</p><script>alert(1)</script>" }),
            json!({ "kind": "paragraph", "text": "untouched" }),
        ];
        let altered = sanitize_document(&mut blocks);
        assert_eq!(altered, 1);
        let html = blocks[0]["html"].as_str().unwrap();
        assert!(html.contains("<p>ok</p>"));
        assert!(!html.contains("<script"));
        assert_eq!(blocks[1]["text"], "untouched");
        // A second pass changes nothing.
        assert_eq!(sanitize_document(&mut blocks), 0);
    }

    #[test]
    fn depth_is_bounded() {
        let mut nested = json!("leaf");
        for _ in 0..MAX_JSON_DEPTH + 2 {
            nested = json!({ "marks": nested });
        }
        let block = json!({ "kind": "paragraph", "text": "x", "marks": nested });
        assert!(
            validate_document(&[block])
                .iter()
                .any(|p| p.contains("nests deeper"))
        );
    }

    #[test]
    fn a_non_object_block_is_refused_without_panicking() {
        let problems = validate_document(&[json!("just a string"), json!(7), json!(null)]);
        assert_eq!(problems.len(), 3);
        assert!(problems[0].contains("must be an object"));
    }
}
