//! The bulk "row" shape + the flattening declaration
//! (`agents/share/bulk-import-export.md` §5; crate spec §10.7) — shared
//! by every flat / line format ([`super::csv`], [`super::jsonl`]). One
//! column list, one classification per column; each format renders it in
//! its own way (CSV as text cells, JSONL as one JSON object per line),
//! but neither redeclares it, so the two formats' column sets cannot
//! drift apart from each other.
//!
//! ## The wire row = an optional `pid` + every `Organization` field
//!
//! `organization_matcher::Organization` carries **no id of its own**
//! (unlike person's `Person::id`) — the service assigns `pid` on create.
//! [`to_row_value`] / [`from_row_value`] are the one place that merges an
//! `Option<Uuid>` pid with the organization's own fields into a single
//! wire object (JSONL: the object *is* the line; CSV: the object's paths
//! are the column cells), so every codec encodes/decodes the identical
//! shape.
//!
//! - the **`pid`** — a single top-level scalar column;
//! - **scalar** fields → one column each (`name`, `legal_name`, `url`,
//!   `jurisdiction`, `founding_date`, `telephone`, `email`);
//! - the single nested **`address`** object → **dotted** columns
//!   (`address.street_address`, `address.locality`, `address.region`,
//!   `address.postal_code`, `address.country`);
//! - every **array / array-of-objects** → a single **JSON-encoded**
//!   column (`identifiers`, `alternate_names`, `same_as`, `keywords`).

use organization_matcher::Organization;
use serde_json::Value;
use uuid::Uuid;

/// How a column's value is classified — every consumer decides for itself
/// how to *render* a classification (CSV: text with an empty-cell
/// convention; JSONL: the object *is* the classification already).
#[derive(Clone, Copy)]
pub enum Kind {
    /// A JSON scalar (string, or absent/null); absent ⇒ the field's own
    /// `Option`/`default` applies.
    Scalar,
    /// A nested value carried as compact JSON text; always present (never
    /// omitted), so a required array such as `identifiers` always
    /// round-trips.
    Json,
}

/// One column: its header, the path into the bulk-row wire `Value`, and
/// its [`Kind`].
pub struct Column {
    /// The column/header name (CSV header cell; the wire object key when
    /// nested via `path`).
    pub header: &'static str,
    /// The path into the bulk-row wire `Value` this column reads/writes.
    pub path: &'static [&'static str],
    /// How this column's value is classified.
    pub kind: Kind,
}

/// The organization column set (crate spec §10.7). Order is the export
/// column order; import matches by header name (CSV), so the order is
/// not load-bearing for reading.
pub const COLUMNS: &[Column] = &[
    col("pid", &["pid"], Kind::Scalar),
    col("name", &["name"], Kind::Scalar),
    col("legal_name", &["legal_name"], Kind::Scalar),
    col("url", &["url"], Kind::Scalar),
    col("jurisdiction", &["jurisdiction"], Kind::Scalar),
    col("founding_date", &["founding_date"], Kind::Scalar),
    col("telephone", &["telephone"], Kind::Scalar),
    col("email", &["email"], Kind::Scalar),
    col(
        "address.street_address",
        &["address", "street_address"],
        Kind::Scalar,
    ),
    col("address.locality", &["address", "locality"], Kind::Scalar),
    col("address.region", &["address", "region"], Kind::Scalar),
    col(
        "address.postal_code",
        &["address", "postal_code"],
        Kind::Scalar,
    ),
    col("address.country", &["address", "country"], Kind::Scalar),
    col("identifiers", &["identifiers"], Kind::Json),
    col("alternate_names", &["alternate_names"], Kind::Json),
    col("same_as", &["same_as"], Kind::Json),
    col("keywords", &["keywords"], Kind::Json),
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

/// Merge an optional `pid` + the organization into one wire `Value`
/// object: the organization's own fields plus a top-level `pid` (a
/// string when present, `null` when absent). This is the row shape every
/// bulk format encodes.
///
/// # Errors
///
/// Returns a [`serde_json::Error`] if `org` fails to serialize (never
/// happens for a well-formed `Organization` — `serde_json::to_value`
/// over a plain-data struct is infallible in practice, but the `Result`
/// keeps this composable with fallible callers).
pub fn to_row_value(pid: Option<Uuid>, org: &Organization) -> serde_json::Result<Value> {
    let mut value = serde_json::to_value(org)?;
    if let Value::Object(map) = &mut value {
        map.insert(
            "pid".to_string(),
            pid.map_or(Value::Null, |p| Value::String(p.to_string())),
        );
    }
    Ok(value)
}

/// The inverse of [`to_row_value`]: split a wire row `Value` back into
/// `(had_explicit_pid, pid, Organization)`.
///
/// `had_explicit_pid` is `true` only when the row's own `pid` cell/key
/// was present and non-blank — the answer
/// [`super::stable_key::is_keyless`] needs, and simpler to compute here
/// than in person's `Person::id` case: `pid` is a genuine
/// `Option<Uuid>`, not a field that silently defaults to a fresh UUID on
/// deserialize, so there is no ambiguity to sniff around.
///
/// # Errors
///
/// A per-row error message (§7 contract: caller records it and moves on)
/// when the `pid` key is present but not a valid UUID string, or when
/// the remaining fields do not deserialize into an [`Organization`].
pub fn from_row_value(mut value: Value) -> Result<(bool, Option<Uuid>, Organization), String> {
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
    let org: Organization = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok((had_explicit_pid, pid, org))
}

#[cfg(test)]
mod tests {
    use super::{COLUMNS, from_row_value, header, to_row_value};
    use organization_matcher::Organization;
    use uuid::Uuid;

    #[test]
    fn header_lists_the_declared_columns() {
        let h = header();
        assert_eq!(h.len(), COLUMNS.len());
        assert_eq!(h.first(), Some(&"pid"));
        assert!(h.contains(&"name"));
        assert!(h.contains(&"address.locality"));
        assert!(h.contains(&"identifiers"));
    }

    /// `to_row_value` / `from_row_value` round-trip both the pid and the
    /// organization payload.
    #[test]
    fn row_value_round_trips_pid_and_organization() {
        let pid = Uuid::new_v4();
        let org = Organization::new("Acme, Inc.");
        let value = to_row_value(Some(pid), &org).unwrap();
        assert_eq!(value["pid"], pid.to_string());
        let (had_explicit_pid, parsed_pid, parsed_org) = from_row_value(value).unwrap();
        assert!(had_explicit_pid);
        assert_eq!(parsed_pid, Some(pid));
        assert_eq!(parsed_org.name, "Acme, Inc.");
    }

    /// A `None` pid round-trips as an absent/`null` pid and
    /// `had_explicit_pid = false`.
    #[test]
    fn row_value_with_no_pid_is_not_explicit() {
        let org = Organization::new("Acme");
        let value = to_row_value(None, &org).unwrap();
        assert!(value["pid"].is_null());
        let (had_explicit_pid, parsed_pid, _) = from_row_value(value).unwrap();
        assert!(!had_explicit_pid);
        assert_eq!(parsed_pid, None);
    }

    /// A malformed `pid` value is a per-row error, not a panic or a
    /// silent `None`.
    #[test]
    fn a_non_uuid_pid_is_a_row_error() {
        let org = Organization::new("Acme");
        let mut value = to_row_value(None, &org).unwrap();
        value["pid"] = serde_json::json!("not-a-uuid");
        assert!(from_row_value(value).is_err());
    }
}
