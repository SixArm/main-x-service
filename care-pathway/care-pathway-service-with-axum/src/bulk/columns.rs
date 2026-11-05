//! The bulk "row" shape + the flattening declaration
//! (`agents/share/bulk-import-export.md` §5; crate spec §9.4) — shared
//! by every flat / line format ([`super::csv`], [`super::jsonl`]). One
//! column list, one classification per column; each format renders it in
//! its own way (CSV as text cells, JSONL as one JSON object per line),
//! but neither redeclares it, so the two formats' column sets cannot
//! drift apart from each other.
//!
//! ## The wire row = an optional `pid` + an optional `active` + every
//! ## `CarePathway` field
//!
//! `care_pathway_matcher::CarePathway` carries **no id and no active
//! flag of its own** — both are properties of the stored row, not the
//! matcher DTO. [`to_row_value`] / [`from_row_value`] are the one place
//! that merges an `Option<Uuid>` pid, an `Option<bool>` active flag, and
//! the pathway's own fields into a single wire object (JSONL: the object
//! *is* the line; CSV: the object's paths are the column cells), so every
//! codec encodes/decodes the identical shape.
//!
//! - the **`pid`** — a single top-level scalar column;
//! - the **`active`** column — export-only (see [`from_row_value`]'s
//!   module-level "known limitation" note below);
//! - **scalar** fields → one column each (`name`, `pathway_code`,
//!   `provider_id`, `provider_name`, `care_setting`);
//! - **`in_language`** — a `Vec<String>` (BCP-47 codes), rendered as a
//!   single **JSON-encoded** column. The crate spec §9.4 groups it with
//!   the "one column each" scalars, but a list value needs the same
//!   JSON-in-cell treatment every other array field gets to round-trip
//!   losslessly; this is a deliberate reading, not a spec contradiction —
//!   it is still exactly one column, whose cell happens to hold JSON.
//! - every other **array / array-of-objects** → a single **JSON-encoded**
//!   column (`alternate_names`, `condition_codes`, `interventions`,
//!   `keywords`, `tags`, `identifiers`, `same_as`, `relationships`).
//!
//! **Known limitation — `active` is not applied on import.** The record
//! model's create/update path (`streaming::create_and_emit` /
//! `update_and_emit`) always writes an active row; there is no bulk
//! "reactivate" or "deactivate" operation in this rollout. The `active`
//! column is populated on **export** (denormalised from the stored row)
//! so an operator can see it, and is silently ignored on **import** — a
//! documented, narrow scope decision rather than a silent behavioural
//! gap (crate spec §9.4 / §13 T-10).

use care_pathway_matcher::CarePathway;
use serde_json::Value;
use uuid::Uuid;

/// How a column's value is classified — every consumer decides for itself
/// how to *render* a classification (CSV: text with an empty-cell
/// convention; JSONL: the object *is* the classification already).
#[derive(Clone, Copy)]
pub enum Kind {
    /// A JSON scalar (string, bool, or absent/null); absent ⇒ the
    /// field's own `Option`/`default` applies.
    Scalar,
    /// A nested value carried as compact JSON text; always present (never
    /// omitted), so a required array such as `identifiers` always
    /// round-trips.
    Json,
}

/// One column: its header, the path into the bulk-row wire `Value`, and
/// its [`Kind`].
pub struct Column {
    /// The column/header name (CSV header cell; the wire object key).
    pub header: &'static str,
    /// The path into the bulk-row wire `Value` this column reads/writes.
    pub path: &'static [&'static str],
    /// How this column's value is classified.
    pub kind: Kind,
}

/// The care-pathway column set (crate spec §9.4). Order is the export
/// column order; import matches by header name (CSV), so the order is
/// not load-bearing for reading.
pub const COLUMNS: &[Column] = &[
    col("pid", &["pid"], Kind::Scalar),
    col("active", &["active"], Kind::Scalar),
    col("name", &["name"], Kind::Scalar),
    col("pathway_code", &["pathway_code"], Kind::Scalar),
    col("provider_id", &["provider_id"], Kind::Scalar),
    col("provider_name", &["provider_name"], Kind::Scalar),
    col("care_setting", &["care_setting"], Kind::Scalar),
    col("in_language", &["in_language"], Kind::Json),
    col("alternate_names", &["alternate_names"], Kind::Json),
    col("condition_codes", &["condition_codes"], Kind::Json),
    col("interventions", &["interventions"], Kind::Json),
    col("keywords", &["keywords"], Kind::Json),
    col("tags", &["tags"], Kind::Json),
    col("identifiers", &["identifiers"], Kind::Json),
    col("same_as", &["same_as"], Kind::Json),
    col("relationships", &["relationships"], Kind::Json),
];

/// `const`-fn column constructor (keeps [`COLUMNS`] readable).
const fn col(header: &'static str, path: &'static [&'static str], kind: Kind) -> Column {
    Column { header, path, kind }
}

/// The export header row / column-name list, in [`COLUMNS`] order.
#[must_use]
pub fn header() -> Vec<&'static str> {
    COLUMNS.iter().map(|c| c.header).collect()
}

/// Navigate `value` by `path`, returning [`Value::Null`] for any missing key.
#[must_use]
pub fn get<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key).unwrap_or(&Value::Null);
    }
    cur
}

/// Set `val` at `path` inside `map`, creating intermediate objects.
pub fn set(map: &mut serde_json::Map<String, Value>, path: &[&str], val: Value) {
    match path {
        [key] => {
            map.insert((*key).to_string(), val);
        }
        [key, rest @ ..] => {
            let entry = map
                .entry((*key).to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if let Value::Object(obj) = entry {
                set(obj, rest, val);
            }
        }
        [] => {}
    }
}

/// Merge an optional `pid` + an optional `active` flag + the pathway into
/// one wire `Value` object: the pathway's own fields plus top-level `pid`
/// / `active` keys (each a string / bool when present, `null` when
/// absent). This is the row shape every bulk format encodes.
///
/// # Errors
///
/// Returns a [`serde_json::Error`] if `pathway` fails to serialize (never
/// happens for a well-formed `CarePathway` — `serde_json::to_value` over
/// a plain-data struct is infallible in practice, but the `Result` keeps
/// this composable with fallible callers).
pub fn to_row_value(
    pid: Option<Uuid>,
    active: Option<bool>,
    pathway: &CarePathway,
) -> serde_json::Result<Value> {
    let mut value = serde_json::to_value(pathway)?;
    if let Value::Object(map) = &mut value {
        map.insert(
            "pid".to_string(),
            pid.map_or(Value::Null, |p| Value::String(p.to_string())),
        );
        map.insert(
            "active".to_string(),
            active.map_or(Value::Null, Value::Bool),
        );
    }
    Ok(value)
}

/// The inverse of [`to_row_value`]: split a wire row `Value` back into
/// `(had_explicit_pid, pid, active, CarePathway)`.
///
/// `had_explicit_pid` is `true` only when the row's own `pid` cell/key
/// was present and non-blank — the answer [`super::stable_key::is_keyless`]
/// needs, and simpler to compute here than in person's `Person::id` case:
/// `pid` is a genuine `Option<Uuid>`, not a field that silently defaults
/// to a fresh UUID on deserialize, so there is no ambiguity to sniff
/// around. `active` is parsed for completeness (export round-trips it)
/// but the import pipeline deliberately never applies it — see the
/// module docs' "known limitation".
///
/// # Errors
///
/// A per-row error message (§7 contract: caller records it and moves on)
/// when the `pid` key is present but not a valid UUID string, when
/// `active` is present but not a boolean, or when the remaining fields do
/// not deserialize into a [`CarePathway`].
pub fn from_row_value(
    mut value: Value,
) -> Result<(bool, Option<Uuid>, Option<bool>, CarePathway), String> {
    let (had_explicit_pid, pid) = match value.as_object_mut().and_then(|m| m.remove("pid")) {
        None | Some(Value::Null) => (false, None),
        Some(Value::String(s)) if s.trim().is_empty() => (false, None),
        Some(Value::String(s)) => {
            let parsed =
                Uuid::parse_str(s.trim()).map_err(|e| format!("pid: invalid UUID: {e}"))?;
            (true, Some(parsed))
        }
        Some(other) => return Err(format!("pid: expected a string, got {other}")),
    };
    // JSONL carries a genuine JSON `Bool`; CSV/TSV cells are always text
    // (`Kind::Scalar` renders every non-null/non-string value via
    // `Display`, and decode always sets a `Value::String` back), so
    // `"true"`/`"false"` text is accepted too.
    let active = match value.as_object_mut().and_then(|m| m.remove("active")) {
        None | Some(Value::Null) => None,
        Some(Value::Bool(b)) => Some(b),
        Some(Value::String(s)) if s == "true" => Some(true),
        Some(Value::String(s)) if s == "false" => Some(false),
        Some(other) => return Err(format!("active: expected a boolean, got {other}")),
    };
    let pathway: CarePathway = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok((had_explicit_pid, pid, active, pathway))
}

#[cfg(test)]
mod tests {
    use super::{COLUMNS, from_row_value, header, to_row_value};
    use care_pathway_matcher::CarePathway;
    use uuid::Uuid;

    #[test]
    fn header_lists_the_declared_columns() {
        let h = header();
        assert_eq!(h.len(), COLUMNS.len());
        assert_eq!(h.first(), Some(&"pid"));
        assert!(h.contains(&"active"));
        assert!(h.contains(&"name"));
        assert!(h.contains(&"in_language"));
        assert!(h.contains(&"identifiers"));
    }

    /// `to_row_value` / `from_row_value` round-trip the pid, the active
    /// flag, and the pathway payload.
    #[test]
    fn row_value_round_trips_pid_active_and_pathway() {
        let pid = Uuid::new_v4();
        let pathway = CarePathway::new("Acute Stroke Pathway");
        let value = to_row_value(Some(pid), Some(true), &pathway).unwrap();
        assert_eq!(value["pid"], pid.to_string());
        assert_eq!(value["active"], true);
        let (had_explicit_pid, parsed_pid, active, parsed) = from_row_value(value).unwrap();
        assert!(had_explicit_pid);
        assert_eq!(parsed_pid, Some(pid));
        assert_eq!(active, Some(true));
        assert_eq!(parsed.name, "Acute Stroke Pathway");
    }

    /// A `None` pid/active round-trips as an absent/`null` value and
    /// `had_explicit_pid = false`.
    #[test]
    fn row_value_with_no_pid_or_active_is_not_explicit() {
        let pathway = CarePathway::new("Acute Stroke Pathway");
        let value = to_row_value(None, None, &pathway).unwrap();
        assert!(value["pid"].is_null());
        assert!(value["active"].is_null());
        let (had_explicit_pid, parsed_pid, active, _) = from_row_value(value).unwrap();
        assert!(!had_explicit_pid);
        assert_eq!(parsed_pid, None);
        assert_eq!(active, None);
    }

    /// A malformed `pid` value is a per-row error, not a panic or a
    /// silent `None`.
    #[test]
    fn a_non_uuid_pid_is_a_row_error() {
        let pathway = CarePathway::new("Acute Stroke Pathway");
        let mut value = to_row_value(None, None, &pathway).unwrap();
        value["pid"] = serde_json::json!("not-a-uuid");
        assert!(from_row_value(value).is_err());
    }

    /// A malformed `active` value is a per-row error too.
    #[test]
    fn a_non_bool_active_is_a_row_error() {
        let pathway = CarePathway::new("Acute Stroke Pathway");
        let mut value = to_row_value(None, None, &pathway).unwrap();
        value["active"] = serde_json::json!("yes");
        assert!(from_row_value(value).is_err());
    }
}
