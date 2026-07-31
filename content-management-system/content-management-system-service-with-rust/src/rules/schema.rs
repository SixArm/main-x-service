//! Content-type field schemas (CMS-R2): the declared shape of a
//! content type, its validation, and the **compatibility classifier**
//! that decides whether an edit is `additive`, `tightening`, or
//! `breaking`. Pure and DB-free.
//!
//! ## Why a classifier exists at all
//!
//! A content type is operator-declared data, so it changes while
//! content already exists under it. Two honest positions are available
//! and one dishonest one:
//!
//! - Re-validate every stored revision on each schema edit — correct,
//!   and unaffordable on a real corpus.
//! - Record which `schema_version` each revision was written under,
//!   keep validating it against that, and **report** which stored
//!   content a tightening would now reject (the `needs_migration`
//!   insight, CMS-R21). This is what we do.
//! - Pretend the edit is free and let reads fail later. This is what a
//!   silent schema edit does, and it is why the classifier refuses to
//!   apply a `breaking` change without an explicit confirmation and a
//!   reason.
//!
//! The classifier is therefore not a nicety: it is the thing that tells
//! an operator, *before* the write, which of the three situations they
//! are in.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

/// The field kinds a content type may declare (spec `authoring.md`).
pub const FIELD_KINDS: &[&str] = &[
    "text",
    "rich_text",
    "number",
    "boolean",
    "date",
    "datetime",
    "choice",
    "media",
    "reference",
    "entity_ref",
    "url",
    "geo",
    "json",
];

/// Field keys reserved by the envelope around a revision's `fields`
/// map, refused so a declared field cannot shadow one.
pub const RESERVED_FIELD_KEYS: &[&str] = &[
    "id",
    "pid",
    "key",
    "locale",
    "status",
    "blocks",
    "title",
    "seo",
    "created_at",
    "updated_at",
];

/// Maximum fields one content type may declare.
pub const MAX_FIELDS: usize = 64;
/// Maximum options a `choice` field may declare.
pub const MAX_OPTIONS: usize = 128;
/// Maximum length of a declared key, label, or option.
pub const MAX_LABEL_LEN: usize = 128;
/// Ceiling for a field's declared `max_len` (the family text cap).
pub const MAX_TEXT_LEN: usize = 1024;

/// Per-field validation rules, all optional.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Validation {
    /// Maximum text length (text-ish kinds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_len: Option<u32>,
    /// Minimum numeric value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    /// Maximum numeric value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// Permitted values for a `choice` field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Permitted target entity types for an `entity_ref` field
    /// (`course`, `event`, …); empty ⇒ any registered type.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_types: Vec<String>,
    /// Permitted target content-type keys for a `reference` field;
    /// empty ⇒ any type.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_keys: Vec<String>,
}

/// One declared field of a content type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// Stable machine key (`snake_case`), unique within the type.
    pub key: String,
    /// Human label for editors.
    pub label: String,
    /// One of [`FIELD_KINDS`].
    pub kind: String,
    /// Whether a value is required before publish.
    #[serde(default)]
    pub required: bool,
    /// Whether the field holds a list of values.
    #[serde(default)]
    pub repeatable: bool,
    /// Per-field validation rules.
    #[serde(default)]
    pub validation: Validation,
}

/// Whether `key` is a legal field key: `snake_case`, starting with a
/// lowercase letter.
#[must_use]
pub fn is_field_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Validate a declared field set, returning every problem found (empty
/// ⇒ valid).
#[must_use]
pub fn validate_fields(fields: &[FieldSpec]) -> Vec<String> {
    let mut problems = Vec::new();
    if fields.is_empty() {
        problems.push("fields must declare at least one field".to_string());
    }
    if fields.len() > MAX_FIELDS {
        problems.push(format!("fields exceeds {MAX_FIELDS} entries"));
    }
    let mut seen = BTreeSet::new();
    for field in fields {
        let key = field.key.as_str();
        if !is_field_key(key) {
            problems.push(format!(
                "fields[{key}].key must be snake_case starting with a letter"
            ));
        }
        if key.len() > MAX_LABEL_LEN {
            problems.push(format!(
                "fields[{key}].key exceeds {MAX_LABEL_LEN} characters"
            ));
        }
        if RESERVED_FIELD_KEYS.contains(&key) {
            problems.push(format!("fields[{key}].key is reserved"));
        }
        if !seen.insert(key) {
            problems.push(format!("fields[{key}].key is duplicated"));
        }
        if field.label.trim().is_empty() {
            problems.push(format!("fields[{key}].label is required"));
        }
        if field.label.len() > MAX_LABEL_LEN {
            problems.push(format!(
                "fields[{key}].label exceeds {MAX_LABEL_LEN} characters"
            ));
        }
        if !FIELD_KINDS.contains(&field.kind.as_str()) {
            problems.push(format!(
                "fields[{key}].kind must be one of {FIELD_KINDS:?}, got {:?}",
                field.kind
            ));
        }
        problems.extend(validate_field_validation(field));
    }
    problems
}

/// Per-kind validation rules for one field.
fn validate_field_validation(field: &FieldSpec) -> Vec<String> {
    let key = field.key.as_str();
    let v = &field.validation;
    let mut problems = Vec::new();

    if field.kind == "choice" {
        if v.options.is_empty() {
            problems.push(format!(
                "fields[{key}].validation.options is required for a choice field"
            ));
        }
        if v.options.len() > MAX_OPTIONS {
            problems.push(format!(
                "fields[{key}].validation.options exceeds {MAX_OPTIONS} entries"
            ));
        }
        let mut seen = BTreeSet::new();
        for option in &v.options {
            if option.trim().is_empty() {
                problems.push(format!(
                    "fields[{key}].validation.options entries must be non-blank"
                ));
            }
            if option.len() > MAX_LABEL_LEN {
                problems.push(format!(
                    "fields[{key}].validation.options entry exceeds {MAX_LABEL_LEN} characters"
                ));
            }
            if !seen.insert(option.as_str()) {
                problems.push(format!(
                    "fields[{key}].validation.options entry {option:?} is duplicated"
                ));
            }
        }
    } else if !v.options.is_empty() {
        problems.push(format!(
            "fields[{key}].validation.options applies only to a choice field"
        ));
    }

    // An `entity_ref` field names the entity types it may point at, and
    // those must be types the family actually registers — a typo here
    // would otherwise declare a reference that can never be satisfied.
    if field.kind == "entity_ref" {
        for entity_type in &v.entity_types {
            if entity_ref::EntityType::from_token(entity_type).is_none() {
                problems.push(format!(
                    "fields[{key}].validation.entity_types entry {entity_type:?} is not a known entity type"
                ));
            }
        }
    } else if !v.entity_types.is_empty() {
        problems.push(format!(
            "fields[{key}].validation.entity_types applies only to an entity_ref field"
        ));
    }

    if field.kind != "reference" && !v.type_keys.is_empty() {
        problems.push(format!(
            "fields[{key}].validation.type_keys applies only to a reference field"
        ));
    }

    if let Some(max_len) = v.max_len {
        if max_len == 0 {
            problems.push(format!("fields[{key}].validation.max_len must be positive"));
        }
        if max_len as usize > MAX_TEXT_LEN {
            problems.push(format!(
                "fields[{key}].validation.max_len exceeds the {MAX_TEXT_LEN}-character family cap"
            ));
        }
    }
    if let (Some(min), Some(max)) = (v.min, v.max)
        && min > max
    {
        problems.push(format!("fields[{key}].validation.min exceeds max"));
    }
    problems
}

/// Validate a revision's **field values** against the content type's
/// declared fields (CMS-R3).
///
/// Required-ness is deliberately **not** checked here. A draft is
/// allowed to be incomplete — that is what a draft is — and refusing a
/// save because a field is still empty would make the editor fight the
/// system all the way to publish. The required-field gate runs at
/// publish (CMS-R11), where refusing is actionable. Use
/// [`missing_required`] for that.
///
/// Unknown keys **are** refused, because silently dropping a value an
/// author sent is how work disappears.
#[must_use]
pub fn validate_values(
    specs: &[FieldSpec],
    values: &serde_json::Map<String, Value>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for key in values.keys() {
        if !specs.iter().any(|s| &s.key == key) {
            problems.push(format!("fields.{key} is not a field of this content type"));
        }
    }
    for spec in specs {
        let Some(value) = values.get(&spec.key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let key = spec.key.as_str();
        if spec.repeatable {
            match value.as_array() {
                Some(items) => {
                    if items.len() > MAX_VALUE_ARRAY_LEN {
                        problems.push(format!(
                            "fields.{key} exceeds {MAX_VALUE_ARRAY_LEN} entries"
                        ));
                    }
                    for (index, item) in items.iter().enumerate() {
                        problems.extend(validate_value(
                            &format!("fields.{key}[{index}]"),
                            spec,
                            item,
                        ));
                    }
                }
                None => problems.push(format!("fields.{key} must be a list")),
            }
        } else if value.is_array() {
            problems.push(format!("fields.{key} must not be a list"));
        } else {
            problems.extend(validate_value(&format!("fields.{key}"), spec, value));
        }
    }
    problems
}

/// The declared-but-absent required fields, for the publish gate
/// (CMS-R11). A field counts as present when it has a non-null,
/// non-blank value.
#[must_use]
pub fn missing_required(
    specs: &[FieldSpec],
    values: &serde_json::Map<String, Value>,
) -> Vec<String> {
    specs
        .iter()
        .filter(|spec| spec.required)
        .filter(|spec| match values.get(&spec.key) {
            None | Some(Value::Null) => true,
            Some(Value::String(text)) => text.trim().is_empty(),
            Some(Value::Array(items)) => items.is_empty(),
            Some(_) => false,
        })
        .map(|spec| spec.key.clone())
        .collect()
}

/// Maximum entries in one repeatable field's value.
pub const MAX_VALUE_ARRAY_LEN: usize = 256;

/// Validate one scalar value at `path` against its declared kind.
fn validate_value(path: &str, spec: &FieldSpec, value: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let v = &spec.validation;
    let wrong = |expected: &str| format!("{path} must be {expected}");

    match spec.kind.as_str() {
        "text" | "rich_text" => match value.as_str() {
            Some(text) => {
                let cap = v.max_len.map_or(MAX_TEXT_LEN, |m| m as usize);
                if text.chars().count() > cap {
                    problems.push(format!("{path} exceeds {cap} characters"));
                }
            }
            None => problems.push(wrong("text")),
        },
        "number" => match value.as_f64() {
            Some(number) => {
                #[allow(clippy::cast_precision_loss)] // bounds are small integers
                if let Some(min) = v.min
                    && number < min as f64
                {
                    problems.push(format!("{path} is below the minimum {min}"));
                }
                #[allow(clippy::cast_precision_loss)]
                if let Some(max) = v.max
                    && number > max as f64
                {
                    problems.push(format!("{path} is above the maximum {max}"));
                }
            }
            None => problems.push(wrong("a number")),
        },
        "boolean" => {
            if !value.is_boolean() {
                problems.push(wrong("true or false"));
            }
        }
        "date" => match value.as_str() {
            Some(text) if chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok() => {}
            _ => problems.push(wrong("a date (YYYY-MM-DD)")),
        },
        "datetime" => match value.as_str() {
            Some(text) if chrono::DateTime::parse_from_rfc3339(text).is_ok() => {}
            _ => problems.push(wrong("an RFC 3339 datetime")),
        },
        "choice" => match value.as_str() {
            Some(text) if v.options.iter().any(|o| o == text) => {}
            Some(text) => problems.push(format!(
                "{path} must be one of {:?}, got {text:?}",
                v.options
            )),
            None => problems.push(wrong("one of the declared options")),
        },
        "media" | "reference" => match value.as_str() {
            Some(text) if uuid::Uuid::parse_str(text).is_ok() => {}
            _ => problems.push(wrong("a uuid")),
        },
        "entity_ref" => match value.as_str().map(str::parse::<entity_ref::EntityRef>) {
            Some(Ok(reference)) => {
                if !v.entity_types.is_empty()
                    && !v
                        .entity_types
                        .iter()
                        .any(|t| t == reference.entity_type.as_str())
                {
                    problems.push(format!(
                        "{path} must reference one of {:?}, got {:?}",
                        v.entity_types,
                        reference.entity_type.as_str()
                    ));
                }
            }
            _ => problems.push(wrong("an EntityRef URN")),
        },
        "url" => match value.as_str() {
            Some(text) if text.starts_with("https://") || text.starts_with("http://") => {}
            _ => problems.push(wrong("an absolute http(s) URL")),
        },
        "geo" => {
            let latitude = value.get("lat").and_then(Value::as_f64);
            let longitude = value.get("lon").and_then(Value::as_f64);
            match (latitude, longitude) {
                (Some(lat), Some(lon))
                    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) => {}
                _ => problems.push(wrong("{lat, lon} within valid bounds")),
            }
        }
        // The escape hatch: any JSON, bounded so it cannot become a
        // parser-time cost, and never rendered as markup.
        "json" => {
            if json_depth(value) > MAX_JSON_DEPTH {
                problems.push(format!("{path} nests deeper than {MAX_JSON_DEPTH}"));
            }
        }
        _ => problems.push(format!("{path} has an unknown declared kind")),
    }
    problems
}

/// Maximum nesting depth of a `json` field value.
pub const MAX_JSON_DEPTH: usize = 8;

/// The nesting depth of a JSON value (a scalar is depth 1).
fn json_depth(value: &Value) -> usize {
    match value {
        Value::Object(map) => 1 + map.values().map(json_depth).max().unwrap_or(0),
        Value::Array(items) => 1 + items.iter().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

/// How an edit to a content type relates to content already stored
/// under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    /// Nothing already stored can become invalid.
    Additive,
    /// Stored content may no longer satisfy the schema; it stays
    /// readable and is reported as `needs_migration`.
    Tightening,
    /// The value shape itself changed (a field removed, its kind or
    /// arity altered): stored values cannot be interpreted under the
    /// new declaration.
    Breaking,
}

impl Compatibility {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Additive => "additive",
            Self::Tightening => "tightening",
            Self::Breaking => "breaking",
        }
    }
}

/// One classified difference between two field declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    /// The field key the change concerns.
    pub field: String,
    /// How severe the change is.
    pub level: Compatibility,
    /// What changed, in words an operator can act on.
    pub detail: String,
}

/// The overall classification of one content-type edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    /// The most severe level among [`Self::changes`] (`additive` when
    /// there are none).
    pub level: Compatibility,
    /// Every classified difference, in field order.
    pub changes: Vec<Change>,
}

impl Classification {
    /// Whether this edit requires the explicit `confirm_breaking` flag
    /// and a reason (CMS-R2).
    #[must_use]
    pub const fn requires_confirmation(&self) -> bool {
        matches!(self.level, Compatibility::Breaking)
    }
}

/// Classify an edit from `old` to `new` field declarations.
///
/// The rules, and why each lands where it does:
///
/// | Change | Level | Because |
/// |---|---|---|
/// | field added, optional | additive | stored content is still valid |
/// | field added, required | tightening | stored content lacks it |
/// | field removed | **breaking** | stored values lose their meaning |
/// | `kind` changed | **breaking** | the stored value shape no longer parses |
/// | `repeatable` changed | **breaking** | a list is not a scalar, either way |
/// | optional → required | tightening | stored blanks now fail |
/// | required → optional | additive | strictly looser |
/// | `choice` option removed | tightening | a stored value may be gone |
/// | `choice` option added | additive | strictly looser |
/// | `max_len` lowered / bound narrowed | tightening | stored values may exceed it |
/// | `max_len` raised / bound widened / removed | additive | strictly looser |
/// | reference / entity-type scope narrowed | tightening | stored targets may be excluded |
/// | label changed | additive | cosmetic |
#[must_use]
pub fn classify(old: &[FieldSpec], new: &[FieldSpec]) -> Classification {
    let mut changes = Vec::new();

    for old_field in old {
        let Some(new_field) = new.iter().find(|f| f.key == old_field.key) else {
            changes.push(Change {
                field: old_field.key.clone(),
                level: Compatibility::Breaking,
                detail: "field removed".to_string(),
            });
            continue;
        };
        changes.extend(classify_field(old_field, new_field));
    }

    for new_field in new {
        if old.iter().any(|f| f.key == new_field.key) {
            continue;
        }
        changes.push(if new_field.required {
            Change {
                field: new_field.key.clone(),
                level: Compatibility::Tightening,
                detail: "required field added".to_string(),
            }
        } else {
            Change {
                field: new_field.key.clone(),
                level: Compatibility::Additive,
                detail: "optional field added".to_string(),
            }
        });
    }

    let level = changes
        .iter()
        .map(|c| c.level)
        .max()
        .unwrap_or(Compatibility::Additive);
    Classification { level, changes }
}

/// Classify the differences between two declarations of one field.
fn classify_field(old: &FieldSpec, new: &FieldSpec) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut push = |level, detail: String| {
        changes.push(Change {
            field: new.key.clone(),
            level,
            detail,
        });
    };

    if old.kind != new.kind {
        push(
            Compatibility::Breaking,
            format!("kind changed from {:?} to {:?}", old.kind, new.kind),
        );
    }
    if old.repeatable != new.repeatable {
        push(
            Compatibility::Breaking,
            format!(
                "repeatable changed from {} to {}",
                old.repeatable, new.repeatable
            ),
        );
    }
    match (old.required, new.required) {
        (false, true) => push(Compatibility::Tightening, "became required".to_string()),
        (true, false) => push(Compatibility::Additive, "became optional".to_string()),
        _ => {}
    }
    if old.label != new.label {
        push(Compatibility::Additive, "label changed".to_string());
    }

    let (old_v, new_v) = (&old.validation, &new.validation);

    let removed: Vec<&String> = old_v
        .options
        .iter()
        .filter(|o| !new_v.options.contains(o))
        .collect();
    if !removed.is_empty() {
        push(
            Compatibility::Tightening,
            format!("choice options removed: {removed:?}"),
        );
    }
    if new_v.options.iter().any(|o| !old_v.options.contains(o)) {
        push(Compatibility::Additive, "choice options added".to_string());
    }

    changes.extend(classify_scope(
        &new.key,
        "entity_types",
        &old_v.entity_types,
        &new_v.entity_types,
    ));
    changes.extend(classify_scope(
        &new.key,
        "type_keys",
        &old_v.type_keys,
        &new_v.type_keys,
    ));

    changes.extend(classify_bound(
        &new.key,
        "max_len",
        old_v.max_len.map(i64::from),
        new_v.max_len.map(i64::from),
        Bound::Upper,
    ));
    changes.extend(classify_bound(
        &new.key,
        "max",
        old_v.max,
        new_v.max,
        Bound::Upper,
    ));
    changes.extend(classify_bound(
        &new.key,
        "min",
        old_v.min,
        new_v.min,
        Bound::Lower,
    ));

    changes
}

/// Which direction makes a numeric bound stricter.
#[derive(Clone, Copy)]
enum Bound {
    /// A ceiling: lowering it is stricter.
    Upper,
    /// A floor: raising it is stricter.
    Lower,
}

/// Classify a change to one numeric bound. Absent-to-present is
/// tightening (a value previously unconstrained may now fail);
/// present-to-absent is additive.
fn classify_bound(
    field: &str,
    name: &str,
    old: Option<i64>,
    new: Option<i64>,
    bound: Bound,
) -> Vec<Change> {
    let level = match (old, new) {
        (None, None) => return Vec::new(),
        (Some(o), Some(n)) if o == n => return Vec::new(),
        (None, Some(_)) => Compatibility::Tightening,
        (Some(_), None) => Compatibility::Additive,
        (Some(o), Some(n)) => {
            let stricter = match bound {
                Bound::Upper => n < o,
                Bound::Lower => n > o,
            };
            if stricter {
                Compatibility::Tightening
            } else {
                Compatibility::Additive
            }
        }
    };
    let detail = match (old, new) {
        (None, Some(n)) => format!("{name} constraint added ({n})"),
        (Some(o), None) => format!("{name} constraint removed (was {o})"),
        (Some(o), Some(n)) => format!("{name} changed from {o} to {n}"),
        (None, None) => unreachable!("handled above"),
    };
    vec![Change {
        field: field.to_string(),
        level,
        detail,
    }]
}

/// Classify a change to a list-shaped *scope* (permitted entity types
/// or content-type keys), where an **empty list means "any"**. Widening
/// to any, or adding a permitted value, is additive; narrowing from any,
/// or removing a permitted value, is tightening.
fn classify_scope(field: &str, name: &str, old: &[String], new: &[String]) -> Vec<Change> {
    if old == new {
        return Vec::new();
    }
    let mut changes = Vec::new();
    let mut push = |level, detail: String| {
        changes.push(Change {
            field: field.to_string(),
            level,
            detail,
        });
    };
    match (old.is_empty(), new.is_empty()) {
        (true, false) => push(
            Compatibility::Tightening,
            format!("{name} narrowed from any to {new:?}"),
        ),
        (false, true) => push(
            Compatibility::Additive,
            format!("{name} widened to any (was {old:?})"),
        ),
        _ => {
            let removed: Vec<&String> = old.iter().filter(|v| !new.contains(v)).collect();
            if !removed.is_empty() {
                push(
                    Compatibility::Tightening,
                    format!("{name} removed: {removed:?}"),
                );
            }
            if new.iter().any(|v| !old.contains(v)) {
                push(Compatibility::Additive, format!("{name} added"));
            }
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(key: &str, kind: &str) -> FieldSpec {
        FieldSpec {
            key: key.to_string(),
            label: key.to_string(),
            kind: kind.to_string(),
            required: false,
            repeatable: false,
            validation: Validation::default(),
        }
    }

    // ---- validation -----------------------------------------------

    #[test]
    fn a_well_formed_field_set_validates() {
        let mut choice = field("section", "choice");
        choice.validation.options = vec!["news".to_string(), "guide".to_string()];
        let mut course = field("about_course", "entity_ref");
        course.validation.entity_types = vec!["course".to_string()];
        let fields = vec![field("summary", "text"), choice, course];
        assert!(
            validate_fields(&fields).is_empty(),
            "{:?}",
            validate_fields(&fields)
        );
    }

    #[test]
    fn field_keys_are_snake_case_unique_and_unreserved() {
        assert!(is_field_key("body_html"));
        assert!(!is_field_key("Body"));
        assert!(!is_field_key("2body"));
        assert!(!is_field_key(""));
        assert!(!is_field_key("body-html"));

        let fields = vec![
            field("Body", "text"),
            field("pid", "text"),
            field("a", "text"),
            field("a", "text"),
        ];
        let problems = validate_fields(&fields);
        assert!(problems.iter().any(|p| p.contains("snake_case")));
        assert!(problems.iter().any(|p| p.contains("is reserved")));
        assert!(problems.iter().any(|p| p.contains("is duplicated")));
    }

    #[test]
    fn an_empty_field_set_is_refused() {
        assert!(
            validate_fields(&[])
                .iter()
                .any(|p| p.contains("at least one field"))
        );
    }

    #[test]
    fn unknown_kinds_are_refused() {
        let problems = validate_fields(&[field("x", "markdown")]);
        assert!(problems.iter().any(|p| p.contains("kind must be one of")));
    }

    #[test]
    fn choice_requires_options_and_other_kinds_refuse_them() {
        let problems = validate_fields(&[field("section", "choice")]);
        assert!(problems.iter().any(|p| p.contains("options is required")));

        let mut text = field("summary", "text");
        text.validation.options = vec!["a".to_string()];
        let problems = validate_fields(&[text]);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("applies only to a choice field"))
        );
    }

    /// A reference to an entity type the family does not register can
    /// never be satisfied, so the typo is caught at declaration time.
    #[test]
    fn entity_ref_types_must_be_known() {
        let mut good = field("about", "entity_ref");
        good.validation.entity_types = vec!["course".to_string()];
        assert!(validate_fields(&[good]).is_empty());

        let mut bad = field("about", "entity_ref");
        bad.validation.entity_types = vec!["kourse".to_string()];
        assert!(
            validate_fields(&[bad])
                .iter()
                .any(|p| p.contains("is not a known entity type"))
        );
    }

    #[test]
    fn numeric_bounds_and_caps_are_checked() {
        let mut f = field("n", "number");
        f.validation.min = Some(10);
        f.validation.max = Some(1);
        assert!(
            validate_fields(&[f])
                .iter()
                .any(|p| p.contains("min exceeds max"))
        );

        let mut f = field("t", "text");
        f.validation.max_len = Some(0);
        assert!(
            validate_fields(&[f])
                .iter()
                .any(|p| p.contains("must be positive"))
        );

        let mut f = field("t", "text");
        f.validation.max_len = Some(u32::try_from(MAX_TEXT_LEN).unwrap() + 1);
        assert!(
            validate_fields(&[f])
                .iter()
                .any(|p| p.contains("family cap"))
        );
    }

    /// The pair the `needs_migration` health rule depends on.
    ///
    /// `validate_values` inspects only fields that are *present*, so on
    /// its own it is silent about the commonest migration of all — a
    /// field that became required after the content was written. That
    /// gap made the health view reassuring and wrong until the rule
    /// consulted `missing_required` as well, which is what the seeded
    /// corpus caught.
    #[test]
    fn a_newly_required_field_is_invisible_to_validate_values_and_visible_to_missing_required() {
        let mut summary = field("summary", "text");
        summary.required = true;
        let specs = vec![field("section", "text"), summary];

        // Content written before `summary` existed.
        let old_content: serde_json::Map<String, Value> =
            serde_json::from_value(serde_json::json!({ "section": "news" })).unwrap();

        assert!(
            validate_values(&specs, &old_content).is_empty(),
            "every value present is still valid — which is why this check alone is not enough"
        );
        assert_eq!(missing_required(&specs, &old_content), vec!["summary"]);

        // Once the field is supplied, both agree it is fine.
        let migrated: serde_json::Map<String, Value> =
            serde_json::from_value(serde_json::json!({ "section": "news", "summary": "Now set" }))
                .unwrap();
        assert!(validate_values(&specs, &migrated).is_empty());
        assert!(missing_required(&specs, &migrated).is_empty());

        // A blank string does not satisfy a required field.
        let blank: serde_json::Map<String, Value> =
            serde_json::from_value(serde_json::json!({ "section": "news", "summary": "   " }))
                .unwrap();
        assert_eq!(missing_required(&specs, &blank), vec!["summary"]);
    }

    // ---- compatibility --------------------------------------------

    #[test]
    fn no_change_is_additive_with_no_changes() {
        let fields = vec![field("summary", "text")];
        let c = classify(&fields, &fields);
        assert_eq!(c.level, Compatibility::Additive);
        assert!(c.changes.is_empty());
        assert!(!c.requires_confirmation());
    }

    #[test]
    fn adding_an_optional_field_is_additive() {
        let old = vec![field("summary", "text")];
        let new = vec![field("summary", "text"), field("standfirst", "text")];
        let c = classify(&old, &new);
        assert_eq!(c.level, Compatibility::Additive);
        assert_eq!(c.changes[0].detail, "optional field added");
    }

    #[test]
    fn adding_a_required_field_is_tightening() {
        let old = vec![field("summary", "text")];
        let mut required = field("standfirst", "text");
        required.required = true;
        let new = vec![field("summary", "text"), required];
        let c = classify(&old, &new);
        assert_eq!(c.level, Compatibility::Tightening);
        assert!(!c.requires_confirmation());
    }

    #[test]
    fn removing_a_field_is_breaking_and_needs_confirmation() {
        let old = vec![field("summary", "text"), field("standfirst", "text")];
        let new = vec![field("summary", "text")];
        let c = classify(&old, &new);
        assert_eq!(c.level, Compatibility::Breaking);
        assert!(c.requires_confirmation());
        assert_eq!(c.changes[0].detail, "field removed");
    }

    #[test]
    fn changing_kind_or_arity_is_breaking() {
        let old = vec![field("summary", "text")];
        let new = vec![field("summary", "number")];
        assert_eq!(classify(&old, &new).level, Compatibility::Breaking);

        let mut repeat = field("summary", "text");
        repeat.repeatable = true;
        assert_eq!(
            classify(&old, &[repeat.clone()]).level,
            Compatibility::Breaking
        );
        // ...and back again: a scalar cannot hold a list either.
        assert_eq!(classify(&[repeat], &old).level, Compatibility::Breaking);
    }

    #[test]
    fn requiredness_moves_both_ways() {
        let optional = vec![field("summary", "text")];
        let mut r = field("summary", "text");
        r.required = true;
        let required = vec![r];
        assert_eq!(
            classify(&optional, &required).level,
            Compatibility::Tightening
        );
        assert_eq!(
            classify(&required, &optional).level,
            Compatibility::Additive
        );
    }

    #[test]
    fn choice_options_narrow_and_widen() {
        let mut old = field("section", "choice");
        old.validation.options = vec!["news".to_string(), "guide".to_string()];
        let mut fewer = old.clone();
        fewer.validation.options = vec!["news".to_string()];
        let mut more = old.clone();
        more.validation.options.push("opinion".to_string());

        let narrowed = classify(std::slice::from_ref(&old), std::slice::from_ref(&fewer));
        assert_eq!(narrowed.level, Compatibility::Tightening);
        assert!(narrowed.changes[0].detail.contains("options removed"));

        assert_eq!(
            classify(std::slice::from_ref(&old), std::slice::from_ref(&more)).level,
            Compatibility::Additive
        );
    }

    #[test]
    fn bounds_tighten_in_the_direction_that_excludes_values() {
        let mut loose = field("summary", "text");
        loose.validation.max_len = Some(500);
        let mut tight = loose.clone();
        tight.validation.max_len = Some(100);
        assert_eq!(
            classify(std::slice::from_ref(&loose), std::slice::from_ref(&tight)).level,
            Compatibility::Tightening
        );
        assert_eq!(
            classify(std::slice::from_ref(&tight), std::slice::from_ref(&loose)).level,
            Compatibility::Additive
        );

        // A floor is the mirror image: raising it excludes values.
        let mut low = field("n", "number");
        low.validation.min = Some(0);
        let mut high = low.clone();
        high.validation.min = Some(5);
        assert_eq!(
            classify(std::slice::from_ref(&low), std::slice::from_ref(&high)).level,
            Compatibility::Tightening
        );
        assert_eq!(
            classify(std::slice::from_ref(&high), std::slice::from_ref(&low)).level,
            Compatibility::Additive
        );

        // Adding a constraint where there was none can exclude stored
        // values; removing one never can.
        let plain = field("summary", "text");
        assert_eq!(
            classify(std::slice::from_ref(&plain), std::slice::from_ref(&tight)).level,
            Compatibility::Tightening
        );
        assert_eq!(
            classify(std::slice::from_ref(&tight), std::slice::from_ref(&plain)).level,
            Compatibility::Additive
        );
    }

    /// An empty scope list means "any", so filling it in is a
    /// narrowing even though the list grew.
    #[test]
    fn scope_lists_treat_empty_as_any() {
        let any = field("about", "entity_ref");
        let mut scoped = any.clone();
        scoped.validation.entity_types = vec!["course".to_string()];
        let narrowed = classify(std::slice::from_ref(&any), std::slice::from_ref(&scoped));
        assert_eq!(narrowed.level, Compatibility::Tightening);
        assert!(narrowed.changes[0].detail.contains("narrowed from any"));
        assert_eq!(
            classify(std::slice::from_ref(&scoped), std::slice::from_ref(&any)).level,
            Compatibility::Additive
        );
    }

    /// The overall level is the most severe change, not the last one.
    #[test]
    fn the_worst_change_wins() {
        let old = vec![field("a", "text"), field("b", "text")];
        let new = vec![field("a", "number"), field("b", "text"), field("c", "text")];
        let c = classify(&old, &new);
        assert_eq!(c.level, Compatibility::Breaking);
        assert!(
            c.changes
                .iter()
                .any(|ch| ch.level == Compatibility::Additive)
        );
    }

    #[test]
    fn wire_tokens_are_stable() {
        assert_eq!(Compatibility::Additive.as_str(), "additive");
        assert_eq!(Compatibility::Tightening.as_str(), "tightening");
        assert_eq!(Compatibility::Breaking.as_str(), "breaking");
        assert_eq!(
            serde_json::to_string(&Compatibility::Breaking).unwrap(),
            "\"breaking\""
        );
    }
}
