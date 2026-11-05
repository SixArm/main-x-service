//! The person wire-type flattening declaration
//! (`agents/share/bulk-import-export.md` §5) — shared by every flat /
//! columnar bulk format ([`super::csv`], [`super::parquet_format`]). One
//! column list, one classification per column; each format renders it in
//! its own way (CSV as text cells, Parquet as typed Arrow columns), but
//! neither redeclares it, so the two formats' column sets cannot drift
//! apart from each other.
//!
//! - **scalar** fields → one column each (`id`, `active`, `gender`,
//!   `birth_date`, `tax_id`, `deceased`, `deceased_datetime`,
//!   `marital_status`, `multiple_birth`, `managing_organization`,
//!   `created_at`, `updated_at`);
//! - the primary **name** (a single nested object) → **dotted** columns
//!   (`name.family` scalar; its `use_type` / `given` / `prefix` / `suffix`
//!   parts follow the rule below);
//! - every **array / array-of-objects** → a single **JSON-encoded** column
//!   (`name.given`, `name.prefix`, `name.suffix`, `identifiers`,
//!   `additional_names`, `telecom`, `documents`, `emergency_contacts`,
//!   `addresses`, `photo`, `links`).
//!
//! The exact column set is declared in the crate spec (§10.6).

use serde_json::Value;

/// How a column's value is classified — every consumer decides for itself
/// how to *render* a classification (CSV: text with an empty-cell
/// convention; Parquet: a typed, nullable Arrow column).
#[derive(Clone, Copy)]
pub enum Kind {
    /// A JSON scalar; absent ⇒ the field's own `Option`/`default` applies.
    Scalar,
    /// A boolean; absent ⇒ the field's own `Option`/`default` applies.
    Bool,
    /// A nested value carried as compact JSON text; always present (never
    /// omitted), so a required array such as `name.given` always
    /// round-trips.
    Json,
}

/// One column: its header, the path into the person wire `Value`, and its
/// [`Kind`].
pub struct Column {
    /// The column/header name (CSV header cell; Parquet field name).
    pub header: &'static str,
    /// The path into the person wire `Value` this column reads/writes.
    pub path: &'static [&'static str],
    /// How this column's value is classified.
    pub kind: Kind,
}

/// The person column set (spec §10.6). Order is the export column order;
/// import matches by header name (CSV), so the order is not load-bearing
/// for reading.
pub const COLUMNS: &[Column] = &[
    col("id", &["id"], Kind::Scalar),
    col("active", &["active"], Kind::Bool),
    col("name.family", &["name", "family"], Kind::Scalar),
    col("name.use_type", &["name", "use_type"], Kind::Scalar),
    col("name.given", &["name", "given"], Kind::Json),
    col("name.prefix", &["name", "prefix"], Kind::Json),
    col("name.suffix", &["name", "suffix"], Kind::Json),
    col("gender", &["gender"], Kind::Scalar),
    col("birth_date", &["birth_date"], Kind::Scalar),
    col("tax_id", &["tax_id"], Kind::Scalar),
    col("deceased", &["deceased"], Kind::Bool),
    col("deceased_datetime", &["deceased_datetime"], Kind::Scalar),
    col("marital_status", &["marital_status"], Kind::Scalar),
    col("multiple_birth", &["multiple_birth"], Kind::Bool),
    col(
        "managing_organization",
        &["managing_organization"],
        Kind::Scalar,
    ),
    col("created_at", &["created_at"], Kind::Scalar),
    col("updated_at", &["updated_at"], Kind::Scalar),
    col("identifiers", &["identifiers"], Kind::Json),
    col("additional_names", &["additional_names"], Kind::Json),
    col("telecom", &["telecom"], Kind::Json),
    col("documents", &["documents"], Kind::Json),
    col("emergency_contacts", &["emergency_contacts"], Kind::Json),
    col("addresses", &["addresses"], Kind::Json),
    col("photo", &["photo"], Kind::Json),
    col("links", &["links"], Kind::Json),
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

#[cfg(test)]
mod tests {
    use super::{COLUMNS, header};

    #[test]
    fn header_lists_the_declared_columns() {
        let h = header();
        assert_eq!(h.len(), COLUMNS.len());
        assert_eq!(h.first(), Some(&"id"));
        assert!(h.contains(&"name.family"));
        assert!(h.contains(&"identifiers"));
        assert!(h.contains(&"links"));
    }
}
